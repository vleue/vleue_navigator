use std::{
    marker::PhantomData,
    sync::{Arc, RwLock},
    time::Duration,
};

#[cfg(feature = "tracing")]
use tracing::instrument;

use bevy::{
    diagnostic::{Diagnostic, DiagnosticPath, Diagnostics, RegisterDiagnostic},
    ecs::entity::EntityHashMap,
    prelude::*,
    tasks::AsyncComputeTaskPool,
    utils::HashMap,
};
use polyanya::Triangulation;

use crate::{obstacles::ObstacleSource, NavMesh};

/// An obstacle that won't change and can be cached
#[derive(Component, Clone, Copy, Debug)]
pub struct CachableObstacle;

/// Bundle for preparing an auto updated navmesh. To use with plugin [`NavmeshUpdaterPlugin`].
#[derive(Bundle, Debug)]
pub struct NavMeshBundle {
    /// Settings for this navmesh updates.
    pub settings: NavMeshSettings,
    /// Status of the last navmesh update.
    pub status: NavMeshStatus,
    /// Handle to the navmesh.
    pub handle: Handle<NavMesh>,
    /// Transform of the navmesh. Used to transform point in 3d to 2d (by ignoring the `z` axis).
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
    /// Duration in seconds after which to cancel a navmesh build
    pub build_timeout: Option<f32>,
    /// Cache from the last build from obstacles that are [`CachableObstacle`]
    pub cached: Option<Triangulation>,
}

impl Default for NavMeshSettings {
    fn default() -> Self {
        Self {
            simplify: 0.0,
            merge_steps: 2,
            default_delta: 0.01,
            fixed: Triangulation::from_outer_edges(&[]),
            build_timeout: None,
            cached: None,
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
    ///
    /// This can happen if the build takes longer than the `build_timeout` defined in the settings.
    Failed,
    /// Cancelled build task. This can happen if settings changed before the build was completed.
    Cancelled,
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
    cached_obstacles: Vec<(GlobalTransform, T)>,
    settings: NavMeshSettings,
    mesh_transform: Transform,
) -> (Triangulation, NavMesh) {
    let base = if settings.cached.is_none() {
        let mut base = settings.fixed;
        let obstacle_aabbs = cached_obstacles
            .iter()
            .map(|(transform, obstacle)| obstacle.get_polygon(transform, &mesh_transform));
        base.add_obstacles(obstacle_aabbs);
        if settings.simplify != 0.0 {
            base.simplify(settings.simplify);
        }
        base.prebuild();
        base
    } else {
        settings.cached.unwrap()
    };
    let mut triangulation = base.clone();
    let obstacle_aabbs = obstacles
        .iter()
        .map(|(transform, obstacle)| obstacle.get_polygon(transform, &mesh_transform))
        .filter(|p| !p.is_empty());
    triangulation.add_obstacles(obstacle_aabbs);

    if settings.simplify != 0.0 {
        triangulation.simplify(settings.simplify);
    }
    let mut navmesh = triangulation.as_navmesh();
    for _ in 0..settings.merge_steps {
        if !navmesh.merge_polygons() {
            break;
        }
    }
    navmesh.bake();
    navmesh.set_delta(settings.default_delta);
    let mut navmesh = NavMesh::from_polyanya_mesh(navmesh);
    navmesh.set_transform(mesh_transform);
    (base, navmesh)
}

fn drop_dead_tasks(
    mut commands: Commands,
    mut navmeshes: Query<
        (Entity, &mut NavMeshStatus, Ref<NavMeshSettings>),
        With<NavmeshUpdateTask>,
    >,
    time: Res<Time>,
    mut task_ages: Local<EntityHashMap<f32>>,
) {
    for (entity, mut status, settings) in &mut navmeshes {
        if status.is_changed() {
            task_ages.insert(entity, time.elapsed_seconds());
        } else if let Some(age) = task_ages.get(&entity).cloned() {
            if settings.is_changed() {
                *status = NavMeshStatus::Cancelled;
                commands.entity(entity).remove::<NavmeshUpdateTask>();
                task_ages.remove(&entity);
            }
            let Some(timeout) = settings.build_timeout else {
                continue;
            };
            if time.elapsed_seconds() - age > timeout {
                *status = NavMeshStatus::Failed;
                commands.entity(entity).remove::<NavmeshUpdateTask>();
                task_ages.remove(&entity);
                warn!("NavMesh build timed out for {:?}", entity);
            }
        }
    }
}

/// Task holder for a navmesh update.
#[derive(Component, Clone)]
pub struct NavmeshUpdateTask(Arc<RwLock<Option<TaskResult>>>);

struct TaskResult {
    navmesh: NavMesh,
    duration: Duration,
    to_cache: Triangulation,
}

type NavMeshToUpdateQuery<'world, 'state, 'a, 'b, 'c, 'd, 'e, 'f> = Query<
    'world,
    'state,
    (
        Entity,
        &'a mut NavMeshSettings,
        Ref<'b, Transform>,
        &'c NavMeshUpdateMode,
        &'d mut NavMeshStatus,
        Option<&'e NavMeshUpdateModeBlocking>,
        Option<&'f NavmeshUpdateTask>,
    ),
>;

type ObstacleQueries<'world, 'state, 'a, 'b, 'c, Obstacle, Marker> = (
    Query<
        'world,
        'state,
        (Ref<'a, GlobalTransform>, &'b Obstacle),
        (With<Marker>, Without<CachableObstacle>),
    >,
    Query<
        'world,
        'state,
        (
            Ref<'a, GlobalTransform>,
            &'b Obstacle,
            Ref<'c, CachableObstacle>,
        ),
        With<Marker>,
    >,
);

fn trigger_navmesh_build<Marker: Component, Obstacle: ObstacleSource>(
    mut commands: Commands,
    (dynamic_obstacles, cachable_obstacles): ObstacleQueries<Obstacle, Marker>,
    removed_obstacles: RemovedComponents<Marker>,
    removed_cachable_obstacles: RemovedComponents<CachableObstacle>,
    mut navmeshes: NavMeshToUpdateQuery,
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
    if !removed_cachable_obstacles.is_empty()
        || cachable_obstacles.iter().any(|(_, _, c)| c.is_added())
    {
        for (_, mut settings, ..) in &mut navmeshes {
            debug!("cache cleared due to cachable obstacle change");
            settings.cached = None;
        }
    }
    for (_, mut settings, ..) in &mut navmeshes {
        if settings.is_changed() {
            debug!("cache cleared due to settings change");
            settings.cached = None;
        }
    }

    let has_removed_obstacles = !removed_obstacles.is_empty();
    let mut to_check = navmeshes
        .iter_mut()
        .filter_map(|(entity, settings, _, mode, ..)| {
            if settings.is_changed()
                || has_removed_obstacles
                || matches!(mode, NavMeshUpdateMode::OnDemand(true))
                || dynamic_obstacles
                    .iter()
                    .any(|(t, _)| t.is_changed() && !t.is_added())
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
        if let Ok((entity, settings, transform, update_mode, mut status, is_blocking, updating)) =
            navmeshes.get_mut(entity)
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
            let cached_obstacles = if settings.cached.is_none() {
                cachable_obstacles
                    .iter()
                    .map(|(t, o, _)| (*t, o.clone()))
                    .collect::<Vec<_>>()
            } else {
                vec![]
            };
            let obstacles_local = dynamic_obstacles
                .iter()
                .map(|(t, o)| (*t, o.clone()))
                .collect::<Vec<_>>();
            let settings_local = settings.clone();
            let transform_local = *transform;

            *status = NavMeshStatus::Building;
            let updating = NavmeshUpdateTask(Arc::new(RwLock::new(None)));
            let writer = updating.0.clone();
            if is_blocking.is_some() {
                let start = bevy::utils::Instant::now();
                let (to_cache, navmesh) = build_navmesh(
                    obstacles_local,
                    cached_obstacles,
                    settings_local,
                    transform_local,
                );
                *writer.write().unwrap() = Some(TaskResult {
                    navmesh,
                    duration: start.elapsed(),
                    to_cache,
                });
            } else {
                AsyncComputeTaskPool::get()
                    .spawn(async move {
                        let start = bevy::utils::Instant::now();
                        let (to_cache, navmesh) = build_navmesh(
                            obstacles_local,
                            cached_obstacles,
                            settings_local,
                            transform_local,
                        );
                        *writer.write().unwrap() = Some(TaskResult {
                            navmesh,
                            duration: start.elapsed(),
                            to_cache,
                        });
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
        &mut NavMeshSettings,
    )>,
    mut navmeshes: ResMut<Assets<NavMesh>>,
    mut diagnostics: Diagnostics,
) {
    for (entity, handle, task, mut status, mut settings) in &mut live_navmeshes {
        let mut task = task.0.write().unwrap();
        if task.is_some() {
            let TaskResult {
                navmesh,
                duration,
                to_cache,
            } = task.take().unwrap();
            commands.entity(entity).remove::<NavmeshUpdateTask>();
            // This is internal and shouldn't trigger change detection
            settings.bypass_change_detection().cached = Some(to_cache);

            debug!("navmesh built");
            navmeshes.insert(handle, navmesh);
            *status = NavMeshStatus::Built;
            diagnostics.add_measurement(&NAVMESH_BUILD_DURATION, || duration.as_secs_f64());
        }
    }
}

/// Plugin to enable automatic navmesh updates.
/// - `Marker` is the component type that marks an entity as an obstacle.
/// - `Obstacle` is the component type that provides the position and shape of an obstacle.
#[derive(Debug)]
pub struct NavmeshUpdaterPlugin<Obstacle: ObstacleSource, Marker: Component = Obstacle> {
    marker1: PhantomData<Marker>,
    marker2: PhantomData<Obstacle>,
}

impl<Marker: Component, Obstacle: ObstacleSource> Default
    for NavmeshUpdaterPlugin<Obstacle, Marker>
{
    fn default() -> Self {
        Self {
            marker1: Default::default(),
            marker2: Default::default(),
        }
    }
}

/// Diagnostic path for the duration of navmesh build.
pub const NAVMESH_BUILD_DURATION: DiagnosticPath =
    DiagnosticPath::const_new("navmesh_build_duration");

impl<Obstacle: ObstacleSource, Marker: Component> Plugin
    for NavmeshUpdaterPlugin<Obstacle, Marker>
{
    fn build(&self, app: &mut App) {
        app.add_systems(PostUpdate, trigger_navmesh_build::<Marker, Obstacle>)
            .add_systems(PreUpdate, (drop_dead_tasks, update_navmesh_asset).chain())
            .register_diagnostic(Diagnostic::new(NAVMESH_BUILD_DURATION));

        #[cfg(feature = "avian2d")]
        {
            app.observe(crate::obstacles::avian2d::on_sleeping_inserted)
                .observe(crate::obstacles::avian2d::on_sleeping_removed);
        }
        #[cfg(feature = "avian3d")]
        {
            app.observe(crate::obstacles::avian3d::on_sleeping_inserted)
                .observe(crate::obstacles::avian3d::on_sleeping_removed);
        }
    }
}
