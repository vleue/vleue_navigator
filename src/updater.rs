use std::{
    marker::PhantomData,
    sync::{Arc, RwLock},
};

#[cfg(feature = "tracing")]
use tracing::instrument;

use bevy::{prelude::*, tasks::AsyncComputeTaskPool, utils::HashMap};
use polyanya::Triangulation;

use crate::{obstacles::ObstacleSource, NavMesh};

/// Bundle for preparing an auto updated navmesh. To use with plugin [`NavmeshUpdaterPlugin`].
#[derive(Bundle, Debug)]
pub struct NavMeshBundle {
    /// Settings for this navmesh updates.
    pub settings: NavMeshSettings,
    /// Status of the last navmesh update.
    pub status: NavMeshStatus,
    /// Handle to the navmesh.
    pub handle: Handle<NavMesh>,
    /// Transform of the navmesh. USed to transform point in 3d to 2d (by ignoring the `z` axis).
    pub transform: Transform,
    /// How to trigger navmesh updates.
    pub update_mode: NavMeshUpdateMode,
}

impl Default for NavMeshBundle {
    fn default() -> Self {
        Self {
            settings: NavMeshSettings::default(),
            status: NavMeshStatus::Building,
            handle: Default::default(),
            transform: Default::default(),
            update_mode: NavMeshUpdateMode::OnDemand(false),
        }
    }
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

impl Default for NavMeshSettings {
    fn default() -> Self {
        Self {
            simplify: 0.0,
            merge_steps: 2,
            default_delta: 0.01,
            fixed: Triangulation::from_outer_edges(&[]),
        }
    }
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

/// If this component is added to an entity with the `NavMeshBundle`, updating the navmesh will be blocking. Otherwise
/// it will be async and happen on the [`AsyncComputeTaskPool`].
#[derive(Component, Debug, Copy, Clone)]
pub struct NavMeshUpdateModeBlocking;

#[cfg_attr(feature = "tracing", instrument(skip_all))]
fn build_navmesh<T: ObstacleSource>(
    obstacles: Vec<(GlobalTransform, T)>,
    settings: NavMeshSettings,
    mesh_transform: Transform,
) -> Result<NavMesh, ()> {
    let obstacle_aabbs = obstacles
        .iter()
        .map(|(transform, obstacle)| obstacle.get_polygon(transform, &mesh_transform))
        .filter(|polygon| !polygon.is_empty());
    let mut triangulation = settings.fixed.clone();
    triangulation.add_obstacles(obstacle_aabbs);
    triangulation.merge_overlapping_obstacles();
    if settings.simplify != 0.0 {
        triangulation.simplify(settings.simplify);
    }
    if let Some(mut navmesh) = triangulation.as_navmesh() {
        for _ in 0..settings.merge_steps {
            if !navmesh.merge_polygons() {
                break;
            }
        }
        navmesh.bake();
        navmesh.set_delta(settings.default_delta);
        let mut navmesh = NavMesh::from_polyanya_mesh(navmesh);
        navmesh.set_transform(mesh_transform);
        Ok(navmesh)
    } else {
        Err(())
    }
}

/// Task holder for a navmesh update.
#[derive(Component, Debug, Clone)]
pub struct NavmeshUpdateTask(Arc<RwLock<Option<Result<NavMesh, ()>>>>);

type NavMeshToUpdateQuery<'world, 'state, 'a, 'b, 'c, 'd, 'e> = Query<
    'world,
    'state,
    (
        Entity,
        Ref<'a, NavMeshSettings>,
        Ref<'b, Transform>,
        &'c NavMeshUpdateMode,
        Option<&'d NavMeshUpdateModeBlocking>,
        Option<&'e NavmeshUpdateTask>,
    ),
>;

fn trigger_navmesh_build<Marker: Component, Obstacle: ObstacleSource>(
    mut commands: Commands,
    obstacles: Query<(Ref<GlobalTransform>, &Obstacle), With<Marker>>,
    removed_obstacles: RemovedComponents<Marker>,
    navmeshes: NavMeshToUpdateQuery,
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
        .filter_map(|(entity, settings, _, mode, _, _)| {
            if obstacles
                .iter()
                .any(|(t, _)| t.is_changed() && !t.is_added())
                || settings.is_changed()
                || has_removed_obstacles
                || matches!(mode, NavMeshUpdateMode::OnDemand(true))
            {
                Some(entity)
            } else {
                None
            }
        })
        .chain(retrigger)
        .collect::<Vec<_>>();
    to_check.sort_unstable();
    to_check.dedup();
    for entity in to_check.into_iter() {
        if let Ok((entity, settings, transform, update_mode, is_blocking, updating)) =
            navmeshes.get(entity)
        {
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
            if updating.is_some() {
                continue;
            }
            let obstacles_local = obstacles
                .iter()
                .map(|(t, o)| (*t, o.clone()))
                .collect::<Vec<_>>();
            let settings_local = settings.clone();
            let transform_local = *transform;

            let updating = NavmeshUpdateTask(Arc::new(RwLock::new(None)));
            let writer = updating.0.clone();
            if is_blocking.is_some() {
                let navmesh = build_navmesh(obstacles_local, settings_local, transform_local);
                *writer.write().unwrap() = Some(navmesh);
            } else {
                AsyncComputeTaskPool::get()
                    .spawn(async move {
                        let navmesh =
                            build_navmesh(obstacles_local, settings_local, transform_local);
                        *writer.write().unwrap() = Some(navmesh);
                    })
                    .detach();
            }
            commands.entity(entity).insert(updating);
        }
    }
}

fn update_navmesh_asset(
    mut commands: Commands,
    mut live_navmeshes: Query<(
        Entity,
        &Handle<NavMesh>,
        &NavmeshUpdateTask,
        &mut NavMeshStatus,
    )>,
    mut navmeshes: ResMut<Assets<NavMesh>>,
) {
    for (entity, handle, task, mut status) in &mut live_navmeshes {
        let mut task = task.0.write().unwrap();
        if task.is_some() {
            let navmesh_built = task.take().unwrap();
            commands.entity(entity).remove::<NavmeshUpdateTask>();

            match navmesh_built {
                Ok(navmesh) => {
                    debug!("navmesh built");
                    navmeshes.insert(handle, navmesh);
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

impl<Marker: Component, Obstacle: ObstacleSource + std::fmt::Debug> Plugin
    for NavmeshUpdaterPlugin<Marker, Obstacle>
{
    fn build(&self, app: &mut App) {
        app.add_systems(PostUpdate, trigger_navmesh_build::<Marker, Obstacle>)
            .add_systems(PreUpdate, update_navmesh_asset);
    }
}
