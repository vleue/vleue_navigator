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
    transform::systems::sync_simple_transforms,
};
use polyanya::{Layer, Mesh, Triangulation};

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
    /// Global Transform of the navmesh.
    pub global_transform: GlobalTransform,
    /// How to trigger navmesh updates.
    pub update_mode: NavMeshUpdateMode,
}

impl NavMeshBundle {
    /// Create a new `NavMeshBundle` with the provided id used for the handle of the `NavMesh`.
    ///
    /// In case there are several `NavMeshBundle`s with the same handle, they will overwrite each others unless they target different layers.
    pub fn with_unique_id(id: u128) -> Self {
        Self {
            handle: Handle::<NavMesh>::weak_from_u128(id),
            ..Self::with_default_id()
        }
    }

    /// Create a new `NavMeshBundle` with the default handle for the `NavMesh`.
    ///
    /// In case there are several `NavMeshBundle`s with the same handle, they will overwrite each others unless they target different layers.
    pub fn with_default_id() -> Self {
        Self {
            settings: NavMeshSettings::default(),
            status: NavMeshStatus::Invalid,
            handle: Default::default(),
            transform: Default::default(),
            global_transform: Default::default(),
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
    /// Upward shift to sample obstacles from the ground
    ///
    /// It should be greater than `0.0` in 3d as colliders lying flat on a surface are not considered intersecting. Default value is `0.1`.
    pub upward_shift: f32,
    /// Specific layer to update. If none, the first layer will be updated.
    pub layer: Option<u8>,
    /// If there are several layers, stitch them together with these segments.
    pub stitches: Vec<((u8, u8), [Vec2; 2])>,
    /// Scale of this navmesh. Defaults to `Vec2::ONE`.
    ///
    /// Used to scale the navmesh to the correct size when displaying it
    ///
    /// if feature `detailed-layers` is enabled, it's also used for path finding to change the traversal cost of this layer.
    pub scale: Vec2,
}

impl Default for NavMeshSettings {
    fn default() -> Self {
        Self {
            simplify: 0.0,
            merge_steps: 0,
            default_delta: 0.01,
            fixed: Triangulation::from_outer_edges(&[]),
            build_timeout: None,
            cached: None,
            // Value is arbitrary, but shouldn't be 0.0. colliders lying flat on a surface are not considered as intersecting with 0.0
            upward_shift: 0.1,
            layer: None,
            stitches: vec![],
            scale: Vec2::ONE,
        }
    }
}

/// Status of the navmesh generation
#[derive(Component, Debug, Copy, Clone, PartialEq, Eq)]
pub enum NavMeshStatus {
    /// Not yet built
    Building,
    /// Built and ready to use
    Built,
    /// Last build command failed. The mesh may still be available from a previous build, but it will be out of date.
    ///
    /// This can happen if the build takes longer than the `build_timeout` defined in the settings.
    Failed,
    /// Last build command failed, and the resulting mesh is invalid and can't be used for pathfinding.
    ///
    /// This can happen if the mesh has different layers that have not yet all been built.
    Invalid,
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
) -> (Triangulation, Layer) {
    let up = (mesh_transform.forward(), settings.upward_shift);
    let scale = settings.scale;
    let base = if settings.cached.is_none() {
        let mut base = settings.fixed;
        let obstacle_polys = cached_obstacles
            .iter()
            .map(|(transform, obstacle)| {
                obstacle
                    .get_polygons(transform, &mesh_transform, up)
                    .into_iter()
            })
            .flatten()
            .filter(|p: &Vec<Vec2>| !p.is_empty())
            .map(|p| p.into_iter().map(|v| v / scale).collect::<Vec<_>>());
        base.add_obstacles(obstacle_polys);
        if settings.simplify != 0.0 {
            base.simplify(settings.simplify);
        }
        base.prebuild();
        base
    } else {
        settings.cached.unwrap()
    };
    let mut triangulation = base.clone();
    let obstacle_polys = obstacles
        .iter()
        .map(|(transform, obstacle)| {
            obstacle
                .get_polygons(transform, &mesh_transform, up)
                .into_iter()
        })
        .flatten()
        .filter(|p: &Vec<Vec2>| !p.is_empty())
        .map(|p| p.into_iter().map(|v| v / scale).collect::<Vec<_>>());
    triangulation.add_obstacles(obstacle_polys);

    if settings.simplify != 0.0 {
        triangulation.simplify(settings.simplify);
    }
    let mut layer = triangulation.as_layer();

    for _ in 0..settings.merge_steps {
        layer.merge_polygons();
    }
    #[cfg(feature = "detailed-layers")]
    {
        layer.scale = scale;
    }
    layer.remove_useless_vertices();
    (base, layer)
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
    layer: Layer,
    duration: Duration,
    to_cache: Triangulation,
}

type NavMeshToUpdateQuery<'world, 'state, 'a, 'b, 'c, 'd, 'e, 'f> = Query<
    'world,
    'state,
    (
        Entity,
        &'a mut NavMeshSettings,
        Ref<'b, GlobalTransform>,
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
        (Ref<'a, GlobalTransform>, Ref<'b, Obstacle>),
        (With<Marker>, Without<CachableObstacle>),
    >,
    Query<
        'world,
        'state,
        (&'a GlobalTransform, &'b Obstacle, Ref<'c, CachableObstacle>),
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
    mut ready_to_update: Local<EntityHashMap<(f32, bool)>>,
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
                    .any(|(t, o)| t.is_changed() || o.is_changed())
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
        if let Ok((
            entity,
            settings,
            global_transform,
            update_mode,
            mut status,
            is_blocking,
            updating,
        )) = navmeshes.get_mut(entity)
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
            let transform_local = global_transform.compute_transform();

            *status = NavMeshStatus::Building;
            let updating = NavmeshUpdateTask(Arc::new(RwLock::new(None)));
            let writer = updating.0.clone();
            if is_blocking.is_some() {
                let start = bevy::utils::Instant::now();
                let (to_cache, layer) = build_navmesh(
                    obstacles_local,
                    cached_obstacles,
                    settings_local,
                    transform_local,
                );
                *writer.write().unwrap() = Some(TaskResult {
                    layer,
                    duration: start.elapsed(),
                    to_cache,
                });
            } else {
                AsyncComputeTaskPool::get()
                    .spawn(async move {
                        let start = bevy::utils::Instant::now();
                        let (to_cache, layer) = build_navmesh(
                            obstacles_local,
                            cached_obstacles,
                            settings_local,
                            transform_local,
                        );
                        *writer.write().unwrap() = Some(TaskResult {
                            layer,
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

type NavMeshWaitingUpdateQuery<'world, 'state, 'a, 'b, 'c, 'd, 'e> = Query<
    'world,
    'state,
    (
        Entity,
        &'a Handle<NavMesh>,
        &'b NavmeshUpdateTask,
        &'c GlobalTransform,
        &'d mut NavMeshStatus,
        &'e mut NavMeshSettings,
    ),
>;

fn update_navmesh_asset(
    mut commands: Commands,
    mut live_navmeshes: NavMeshWaitingUpdateQuery,
    mut navmeshes: ResMut<Assets<NavMesh>>,
    mut diagnostics: Diagnostics,
) {
    for (entity, handle, task, global_transform, mut status, mut settings) in &mut live_navmeshes {
        let mut task = task.0.write().unwrap();
        if let Some(TaskResult {
            layer,
            duration,
            to_cache,
        }) = task.take()
        {
            let mut failed_stitches = vec![];
            commands.entity(entity).remove::<NavmeshUpdateTask>();
            // This is internal and shouldn't trigger change detection
            settings.bypass_change_detection().cached = Some(to_cache);
            debug!(
                "navmesh {:?} ({:?}) built{}",
                handle,
                entity,
                if let Some(layer) = &settings.layer {
                    format!(" (layer {})", layer)
                } else {
                    "".to_string()
                }
            );
            let (previous_navmesh_transform, mut mesh, mut previously_failed) =
                if let Some(navmesh) = navmeshes.get(handle) {
                    if let Some(mesh) = navmesh.building.as_ref() {
                        (
                            navmesh.transform(),
                            mesh.mesh.clone(),
                            mesh.failed_stitches.clone(),
                        )
                    } else {
                        (navmesh.transform(), (*navmesh.get()).clone(), vec![])
                    }
                } else {
                    (
                        Transform::IDENTITY,
                        Mesh {
                            layers: vec![],
                            delta: settings.default_delta,
                        },
                        vec![],
                    )
                };

            if let Some(layer_id) = &settings.layer {
                *status = NavMeshStatus::Built;

                if mesh.layers.len() < *layer_id as usize + 1 {
                    mesh.layers.resize(
                        mesh.layers.len().max(*layer_id as usize + 1),
                        Layer::default(),
                    );
                }
                mesh.remove_stitches_to_layer(*layer_id);
                mesh.layers[*layer_id as usize] = layer;
                // TODO: rotate this to get the value in the correct space
                mesh.layers[*layer_id as usize].offset = global_transform.translation().xz();

                let stitch_segments =
                    settings
                        .stitches
                        .iter()
                        .filter_map(|((from, to), segment)| {
                            let other = if from == layer_id {
                                to
                            } else if to == layer_id {
                                from
                            } else {
                                return None;
                            };
                            Some((other, [segment[0], segment[1]]))
                        });
                let layer_from = &mesh.layers[*layer_id as usize];
                let mut stitch_vertices = vec![];
                'stitching: for (target_layer, stitch_segment) in stitch_segments {
                    if mesh.layers.len() < *target_layer as usize + 1 {
                        *status = NavMeshStatus::Invalid;
                        continue 'stitching;
                    }
                    if mesh.layers[*target_layer as usize].vertices.is_empty() {
                        *status = NavMeshStatus::Invalid;
                        continue 'stitching;
                    }
                    let layer_to = &mesh.layers[*target_layer as usize];

                    let indices_from = layer_from.get_vertices_on_segment(
                        stitch_segment[0] - layer_from.offset,
                        stitch_segment[1] - layer_from.offset,
                    );
                    let indices_to = layer_to.get_vertices_on_segment(
                        stitch_segment[0] - layer_to.offset,
                        stitch_segment[1] - layer_to.offset,
                    );
                    if indices_from.len() != indices_to.len() {
                        debug!(
                            "navmesh {:?} ({:?}) layer {} update: error stitching to layer {:?} (different number of stitching points on each side)",
                            handle, entity, layer_id, target_layer
                        );
                        *status = NavMeshStatus::Failed;
                        failed_stitches.push((*layer_id, *target_layer));
                        continue 'stitching;
                    }

                    let stitch_indices = indices_from
                        .into_iter()
                        .zip(indices_to.into_iter())
                        .collect::<Vec<_>>();
                    for indices in &stitch_indices {
                        if (layer_from.vertices[indices.0].coords + layer_from.offset)
                            .distance_squared(layer_to.vertices[indices.1].coords + layer_to.offset)
                            > 0.001
                        {
                            debug!(
                                "navmesh {:?} ({:?}) layer {} update: error stitching to layer {:?} (stitching points don't match)",
                                handle, entity, layer_id, target_layer
                            );
                            *status = NavMeshStatus::Failed;
                            failed_stitches.push((*layer_id, *target_layer));
                            continue 'stitching;
                        }
                    }

                    previously_failed
                        .retain(|(from, to)| !(*from == *layer_id && *to == *target_layer));
                    previously_failed
                        .retain(|(from, to)| !(*to == *layer_id && *from == *target_layer));
                    stitch_vertices.push(((*layer_id, *target_layer), stitch_indices));
                }
                mesh.restitch_layer_at_vertices(*layer_id, stitch_vertices, false);

                if *status == NavMeshStatus::Built && previously_failed.is_empty() {
                    let mut navmesh = NavMesh::from_polyanya_mesh(mesh);
                    if *layer_id == 0 {
                        navmesh.set_transform(global_transform.compute_transform());
                    } else {
                        navmesh.set_transform(previous_navmesh_transform);
                    }
                    navmeshes.insert(handle, navmesh);
                } else if let Some(navmesh) = navmeshes.get_mut(handle) {
                    failed_stitches.extend(previously_failed);
                    failed_stitches.sort_unstable();
                    failed_stitches.dedup();
                    navmesh.building = Some(crate::BuildingMesh {
                        mesh,
                        failed_stitches,
                    });
                } else {
                    let navmesh = NavMesh::from_polyanya_mesh(mesh);
                    navmeshes.insert(handle, navmesh);
                    *status = NavMeshStatus::Invalid;
                }
            } else {
                mesh.layers = vec![layer];
                let mut navmesh = NavMesh::from_polyanya_mesh(mesh);
                navmesh.set_transform(global_transform.compute_transform());
                navmeshes.insert(handle, navmesh);
                *status = NavMeshStatus::Built;
            }
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
        app.add_systems(
            PostUpdate,
            trigger_navmesh_build::<Marker, Obstacle>.after(sync_simple_transforms),
        )
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
