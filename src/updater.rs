use std::{
    marker::PhantomData,
    sync::{Arc, RwLock},
};

#[cfg(feature = "tracing")]
use tracing::instrument;

use bevy::{
    math::{vec3, Vec3Swizzles},
    prelude::*,
    render::primitives::Aabb,
    tasks::AsyncComputeTaskPool,
    utils::HashMap,
};
use polyanya::Triangulation;

use crate::PathMesh;

/// Bundle for preparing an auto updated navmesh. To use with plugin [`NavmeshUpdaterPlugin`].
#[derive(Bundle, Debug)]
pub struct NavMeshBundle {
    /// Settings for this navmesh updates.
    pub settings: NavMeshSettings,
    /// Status of the last navmesh update.
    pub status: NavMeshStatus,
    /// Handle to the navmesh.
    pub handle: Handle<PathMesh>,
    /// Transform of the navmesh. USed to transform point in 3d to 2d (by ignoring the `z` axis).
    pub transform: Transform,
    /// How to trigger navmesh updates.
    pub update_mode: NavMeshUpdateMode,
}

/// Settings for nav mesh generation.
#[derive(Component, Clone, Debug)]
pub struct NavMeshSettings {
    /// Minimum area a point of an obstacle must impact
    pub simplify: f32,
    /// Number of times to merge polygons
    pub merge_steps: usize,
    /// Default delta use for the navmesh during pathing
    pub default_delta: f32,
    /// Fixed edges and obstacles of the mesh
    pub fixed: Triangulation,
}

/// Status of the navmesh generation
#[derive(Component, Debug, Copy, Clone)]
pub enum NavMeshStatus {
    /// Not yet built
    Building,
    /// Built and ready to use
    Built,
    /// Last build command failed. The mesh may still be available from a previous build, but it will be out of date.
    Failed,
}

/// Control when to update the navmesh
#[derive(Component, Debug, Copy, Clone)]
pub enum NavMeshUpdateMode {
    /// On every change
    Direct,
    /// On every debounced change, at maximum every `f32` seconds
    Debounced(f32),
    /// On demand, set it to `true` to trigger an update
    OnDemand(bool),
}

/// Trait to mark a component as the source of position and shape of an obstacle.
pub trait ObstacleSource: Component + Clone {
    /// Get the polygon of the obstacle in the local space of the mesh.
    fn get_polygon(&self, transform: &GlobalTransform, transform: &Transform) -> Vec<Vec2>;
}

impl ObstacleSource for Aabb {
    fn get_polygon(&self, transform: &GlobalTransform, mesh_transform: &Transform) -> Vec<Vec2> {
        let transform = transform.compute_transform();
        let to_vec2 = |v: Vec3| mesh_transform.transform_point(v).xy();

        vec![
            to_vec2(transform.transform_point(vec3(
                -self.half_extents.x,
                0.0,
                self.half_extents.z,
            ))),
            to_vec2(transform.transform_point(vec3(
                -self.half_extents.x,
                0.0,
                -self.half_extents.z,
            ))),
            to_vec2(transform.transform_point(vec3(
                self.half_extents.x,
                0.0,
                -self.half_extents.z,
            ))),
            to_vec2(transform.transform_point(vec3(self.half_extents.x, 0.0, self.half_extents.z))),
        ]
    }
}

#[cfg_attr(feature = "tracing", instrument(skip_all))]
fn build_pathmesh<T: ObstacleSource>(
    obstacles: Vec<(GlobalTransform, T)>,
    settings: NavMeshSettings,
    mesh_transform: Transform,
) -> Result<PathMesh, ()> {
    let obstacle_aabbs = obstacles
        .iter()
        .map(|(transform, obstacle)| obstacle.get_polygon(transform, &mesh_transform))
        .collect::<Vec<_>>();
    let mut triangulation = settings.fixed.clone();
    triangulation.add_obstacles(obstacle_aabbs);
    triangulation.merge_overlapping_obstacles();
    triangulation.simplify(settings.simplify);
    if let Some(mut navmesh) = triangulation.as_navmesh() {
        for _ in 0..settings.merge_steps {
            navmesh.merge_polygons();
        }
        navmesh.set_delta(settings.default_delta);
        let mut pathmesh = PathMesh::from_polyanya_mesh(navmesh);
        pathmesh.set_transform(mesh_transform);
        Ok(pathmesh)
    } else {
        Err(())
    }
}

#[derive(Component)]
struct NavmeshUpdateTask(Arc<RwLock<Option<Result<PathMesh, ()>>>>);

fn update_navmesh<Marker: Component, Obstacle: ObstacleSource>(
    mut commands: Commands,
    obstacles: Query<(Ref<GlobalTransform>, &Obstacle), With<Marker>>,
    removed_obstacles: RemovedComponents<Marker>,
    navmeshes: Query<
        (
            Entity,
            Ref<NavMeshSettings>,
            Ref<Transform>,
            &NavMeshUpdateMode,
        ),
        Without<NavmeshUpdateTask>,
    >,
    time: Res<Time>,
    mut ready_to_update: Local<HashMap<Entity, (f32, bool)>>,
) {
    let keys = ready_to_update.keys().cloned().collect::<Vec<_>>();
    let mut retrigger = vec![];
    for key in keys {
        let val = ready_to_update.get_mut(&key).unwrap();
        val.0 -= time.delta_seconds();
        if val.0 < 0.0 {
            if val.1 {
                retrigger.push(key);
            }
            ready_to_update.remove(&key);
        }
    }
    let has_removed_obstacles = !removed_obstacles.is_empty();
    let mut to_check = navmeshes
        .iter()
        .filter_map(|(entity, settings, _, mode)| {
            if obstacles.iter().any(|(t, _)| t.is_changed())
                || settings.is_changed()
                || has_removed_obstacles
                || matches!(mode, NavMeshUpdateMode::OnDemand(true))
            {
                Some(entity)
            } else {
                None
            }
        })
        .chain(retrigger.into_iter())
        .collect::<Vec<_>>();
    to_check.sort_unstable();
    to_check.dedup();
    for entity in to_check.into_iter() {
        if let Ok((entity, settings, transform, update_mode)) = navmeshes.get(entity) {
            if let Some(val) = ready_to_update.get_mut(&entity) {
                val.1 = true;
                continue;
            }
            match *update_mode {
                NavMeshUpdateMode::Debounced(seconds) => {
                    ready_to_update.insert(entity, (seconds, false));
                }
                NavMeshUpdateMode::OnDemand(false) => {
                    continue;
                }
                NavMeshUpdateMode::OnDemand(true) => {
                    commands
                        .entity(entity)
                        .insert(NavMeshUpdateMode::OnDemand(false));
                }
                _ => (),
            };
            let obstacles_local = obstacles
                .iter()
                .map(|(t, o)| (*t, o.clone()))
                .collect::<Vec<_>>();
            let settings_local = settings.clone();
            let transform_local = *transform;

            let updating = NavmeshUpdateTask(Arc::new(RwLock::new(None)));
            let writer = updating.0.clone();

            AsyncComputeTaskPool::get()
                .spawn(async move {
                    let pathmesh = build_pathmesh(obstacles_local, settings_local, transform_local);
                    *writer.write().unwrap() = Some(pathmesh);
                })
                .detach();
            commands.entity(entity).insert(updating);
        }
    }
}

fn update_navmesh2(
    mut commands: Commands,
    mut navmeshes: Query<(
        Entity,
        &Handle<PathMesh>,
        &NavmeshUpdateTask,
        &mut NavMeshStatus,
    )>,
    mut pathmeshes: ResMut<Assets<PathMesh>>,
) {
    for (entity, handle, task, mut status) in &mut navmeshes {
        let mut task = task.0.write().unwrap();
        if task.is_some() {
            let pathmesh_built = task.take().unwrap();
            commands.entity(entity).remove::<NavmeshUpdateTask>();

            match pathmesh_built {
                Ok(navmesh) => {
                    debug!("navmesh built");
                    let _ = pathmeshes.set(handle, navmesh);
                    *status = NavMeshStatus::Built;
                }
                Err(()) => {
                    warn!("navmesh build failed");
                    *status = NavMeshStatus::Failed;
                }
            }
        }
    }
}

/// Plugin to enable automatic navmesh updates.
/// - `Marker` is the component type that marks an entity as an obstacle.
/// - `Obstacle` is the component type that provides the position and shape of an obstacle.
#[derive(Debug)]
pub struct NavmeshUpdaterPlugin<Marker: Component, Obstacle: ObstacleSource> {
    marker1: PhantomData<Marker>,
    marker2: PhantomData<Obstacle>,
}

impl<Marker: Component, Obstacle: ObstacleSource> Default
    for NavmeshUpdaterPlugin<Marker, Obstacle>
{
    fn default() -> Self {
        Self {
            marker1: Default::default(),
            marker2: Default::default(),
        }
    }
}

impl<Marker: Component, Obstacle: ObstacleSource> Plugin
    for NavmeshUpdaterPlugin<Marker, Obstacle>
{
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            (update_navmesh::<Marker, Obstacle>, update_navmesh2),
        );
    }
}
