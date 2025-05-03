use std::{
    marker::PhantomData,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};

#[cfg(feature = "tracing")]
use tracing::instrument;

use bevy::{
    diagnostic::{Diagnostic, DiagnosticPath, Diagnostics, RegisterDiagnostic},
    ecs::entity::EntityHashMap,
    prelude::*,
    tasks::AsyncComputeTaskPool,
    transform::TransformSystem,
};
use bevy::asset::uuid::Uuid;
use polyanya::{Layer, Mesh, Triangulation};

use crate::{obstacles::ObstacleSource, NavMesh};

/// A Marker component for an obstacle that can be cached.
///
/// Caching obstacles can help to optimize the [`NavMesh`] generation process.
/// A partial [`NavMesh`] will be built with them, then updated with the dynamic obstacles.
#[derive(Component, Clone, Copy, Debug)]
pub struct CachableObstacle;

/// A NavMesh that will be updated automatically.
#[derive(Component, Debug, Deref)]
#[require(NavMeshStatus, NavMeshUpdateMode, Transform, GlobalTransform)]
pub struct ManagedNavMesh(Handle<NavMesh>);

impl ManagedNavMesh {
    /// Create a new [`ManagedNavMesh`] with the provided id.
    pub fn from_id(id: u128) -> Self {
        Self(Handle::Weak(AssetId::Uuid {
            uuid: Uuid::from_u128(id),
        }))
    }

    /// Create a new [`ManagedNavMesh`].
    ///
    /// This can be used when there is a single NavMesh in the scene.
    /// Otherwise use [`Self::from_id`] with unique IDs for each NavMesh.
    pub fn single() -> Self {
        Self(Handle::Weak(AssetId::Uuid {
            uuid: Uuid::from_u128(0),
        }))
    }
}

impl From<ManagedNavMesh> for AssetId<NavMesh> {
    fn from(navmesh: ManagedNavMesh) -> Self {
        navmesh.id()
    }
}

impl From<&ManagedNavMesh> for AssetId<NavMesh> {
    fn from(navmesh: &ManagedNavMesh) -> Self {
        navmesh.id()
    }
}

/// Settings for nav mesh generation.
#[derive(Component, Clone, Debug)]
#[require(ManagedNavMesh = ManagedNavMesh::single())]
pub struct NavMeshSettings {
    /// The minimum area that a point of an obstacle must impact. Otherwise, the obstacle will be simplified by removing this point.
    ///
    /// This value depends on the scale of your obstacles and agents. The default value is `0.0`.
    /// Having a non-zero value can help to remove small artifacts from the generated [`NavMesh`], and speed up generation and pathfinding.
    pub simplify: f32,
    /// The number of iterations to merge polygons during the [`NavMesh`] generation.
    ///
    /// It's rarely useful to set this value to a number greater than `3`. The default value is `0`.
    pub merge_steps: usize,
    /// The default search delta used for pathfinding within the [`NavMesh`].
    ///
    /// This controls the radius of the circle used to search for a point in a [`NavMesh`].
    /// This value depends on the scale of your obstacles and agents. The default value is `0.01`.
    /// Increasing this value can help if you often get pathfinding errors when agents are too close to a border. Those errors happens because of rounding errors.
    pub default_search_delta: f32,
    /// The default number of search steps used for pathfinding within the [`NavMesh`].
    ///
    /// This controls the number of time of the circle used to search for a point in a [`NavMesh`] is expanded by the [`Self::default_search_delta`]. The default value is `4`.
    /// Increasing this value can help if you often get pathfinding errors when agents are too close to a border. Those errors happens because of rounding errors.
    pub default_search_steps: u32,
    /// The fixed edges and obstacles that define the structure of the [`NavMesh`].
    ///
    /// Creating this [`Triangulation`] can be done with the [`Triangulation::from_outer_edges`] method, and static obstacles added with [`Triangulation::add_obstacles`].
    pub fixed: Triangulation,
    /// The duration in seconds after which a [`NavMesh`] build is canceled if not completed.
    pub build_timeout: Option<f32>,
    /// A cache of the last build from obstacles marked as [`CachableObstacle`].
    pub cached: Option<Triangulation>,
    /// The upward shift applied to sample obstacles from the ground.
    ///
    /// This value should be greater than `0.0` in 3D environments, as colliders lying flat on a surface are not considered intersecting.
    /// The default value is `0.1`.
    pub upward_shift: f32,
    /// The specific layer to update in the [`NavMesh`]. If `None`, the first layer will be updated.
    ///
    /// Layers are used when the [`NavMesh`] has overlapping parts, or parts with different traversal costs.
    pub layer: Option<u8>,
    /// Segments used to stitch together multiple layers in the [`NavMesh`].
    pub stitches: Vec<((u8, u8), [Vec2; 2])>,
    /// The scale of the [`NavMesh`], defaulting to `Vec2::ONE`.
    ///
    /// This scale is used to adjust the size of the [`NavMesh`] when displaying it.
    ///
    /// If the `detailed-layers` feature is enabled, it is also used for pathfinding to modify the traversal cost of this layer.
    pub scale: Vec2,
    /// The radius of the agent used to inflate obstacles in the [`NavMesh`].
    pub agent_radius: f32,
    /// Determines if the agent radius should be applied to the outer edges of the [`NavMesh`].
    ///
    /// When using layers, applying the agent radius to outer edges can block stitching them together.
    pub agent_radius_on_outer_edge: bool,
}

impl Default for NavMeshSettings {
    fn default() -> Self {
        Self {
            simplify: 0.0,
            merge_steps: 0,
            default_search_delta: 0.01,
            default_search_steps: 4,
            fixed: Triangulation::from_outer_edges(&[]),
            build_timeout: None,
            cached: None,
            // Value is arbitrary, but shouldn't be 0.0. colliders lying flat on a surface are not considered as intersecting with 0.0
            upward_shift: 0.1,
            layer: None,
            stitches: vec![],
            scale: Vec2::ONE,
            agent_radius: 0.0,
            agent_radius_on_outer_edge: false,
        }
    }
}

/// Status of the navmesh generation
#[derive(Component, Debug, Copy, Clone, PartialEq, Eq, Default)]
pub enum NavMeshStatus {
    /// The [`NavMesh`] has not yet been built.
    Building,
    /// The [`NavMesh`] has been successfully built and is ready for use.
    Built,
    /// The last build failed. The [`NavMesh`] may still be available from a previous build, but it will be out of date.
    ///
    /// This can occur if the build takes longer than the [`NavMeshSettings::build_timeout`] defined in the settings.
    Failed,
    /// The last build command failed, and the resulting [`NavMesh`] is invalid and cannot be used for pathfinding.
    ///
    /// This can occur if the [`NavMesh`] has different layers that have not yet all been built.
    #[default]
    Invalid,
    /// The build task was canceled. This can occur if [`NavMeshSettings`] associated changed before the last build was completed.
    Cancelled,
}

/// Control when to update the navmesh
#[derive(Component, Debug, Copy, Clone)]
pub enum NavMeshUpdateMode {
    /// Update the [`NavMesh`] on every change.
    Direct,
    /// Update the [`NavMesh`] on every debounced change, at most every `f32` seconds.
    Debounced(f32),
    /// Update the [`NavMesh`] on demand. Set to `true` to trigger an update.
    OnDemand(bool),
}

impl Default for NavMeshUpdateMode {
    fn default() -> Self {
        Self::OnDemand(false)
    }
}

/// If this component is added to an entity with the [`NavMeshBundle`], updating the [`NavMesh`] will be blocking.
/// Otherwise, it will be done asynchronous and occur on the [`AsyncComputeTaskPool`].
///
/// This can cause the game to lag if updating the [`NavMesh`] takes longer than a frame. This is not recommended to use.
#[derive(Component, Debug, Copy, Clone)]
pub struct NavMeshUpdateModeBlocking;

#[cfg_attr(feature = "tracing", instrument(skip_all))]
fn build_navmesh<T: ObstacleSource>(
    obstacles: Vec<(GlobalTransform, T)>,
    cached_obstacles: Vec<(GlobalTransform, T)>,
    settings: NavMeshSettings,
    mesh_transform: Transform,
) -> (Option<Triangulation>, Layer) {
    let up = (mesh_transform.forward(), settings.upward_shift);
    let scale = settings.scale;
    let base = if settings.cached.is_none() {
        let mut base = settings.fixed;
        base.set_agent_radius(settings.agent_radius);
        base.set_agent_radius_simplification(settings.simplify);
        base.agent_radius_on_outer_edge(settings.agent_radius_on_outer_edge);
        let obstacle_polys = cached_obstacles
            .iter()
            .flat_map(|(transform, obstacle)| {
                obstacle
                    .get_polygons(transform, &mesh_transform, up)
                    .into_iter()
            })
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
        .flat_map(|(transform, obstacle)| {
            obstacle
                .get_polygons(transform, &mesh_transform, up)
                .into_iter()
        })
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
    (
        if cached_obstacles.is_empty() {
            None
        } else {
            Some(base)
        },
        layer,
    )
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
            task_ages.insert(entity, time.elapsed_secs());
        } else if let Some(age) = task_ages.get(&entity).cloned() {
            if settings.is_changed() {
                *status = NavMeshStatus::Cancelled;
                commands.entity(entity).remove::<NavmeshUpdateTask>();
                task_ages.remove(&entity);
            }
            let Some(timeout) = settings.build_timeout else {
                continue;
            };
            if time.elapsed_secs() - age > timeout {
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
    to_cache: Option<Triangulation>,
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
        val.0 -= time.delta_secs();
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
            settings.bypass_change_detection().cached = None;
        }
    }
    for (_, mut settings, ..) in &mut navmeshes {
        if settings.is_changed() {
            debug!("cache cleared due to settings change");
            settings.bypass_change_detection().cached = None;
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
                let start = Instant::now();
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
                        let start = Instant::now();
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
        &'a ManagedNavMesh,
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
            if to_cache.is_some() {
                debug!("cache updated");
                // This is internal and shouldn't trigger change detection
                settings.bypass_change_detection().cached = to_cache;
            }
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
                if let Some(navmesh) = navmeshes.get(&handle.0) {
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
                            search_delta: settings.default_search_delta,
                            search_steps: settings.default_search_steps,
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
                    navmeshes.insert(&handle.0, navmesh);
                } else if let Some(navmesh) = navmeshes.get_mut(&handle.0) {
                    failed_stitches.extend(previously_failed);
                    failed_stitches.sort_unstable();
                    failed_stitches.dedup();
                    navmesh.building = Some(crate::BuildingMesh {
                        mesh,
                        failed_stitches,
                    });
                } else {
                    let navmesh = NavMesh::from_polyanya_mesh(mesh);
                    navmeshes.insert(&handle.0, navmesh);
                    *status = NavMeshStatus::Invalid;
                }
            } else {
                mesh.layers = vec![layer];
                let mut navmesh = NavMesh::from_polyanya_mesh(mesh);
                navmesh.set_transform(global_transform.compute_transform());
                navmeshes.insert(&handle.0, navmesh);
                *status = NavMeshStatus::Built;
            }
            diagnostics.add_measurement(&NAVMESH_BUILD_DURATION, || duration.as_secs_f64());
        }
    }
}

/// Plugin to enable automatic [`NavMesh`] updates.
///
/// - `Obstacle` is the component type that provides the position and shape of an obstacle.
/// - `Marker` is the component type that marks an entity as an obstacle. It defaults to `Obstacle`, so that it's not needed if all entities with `Obstacle` are obstacles.
///
/// # Example
///
/// When using [`Aabb`](bevy::render::primitives::Aabb) as the obstacle shape, the [`Obstacle`] component should be [`Aabb`](bevy::render::primitives::Aabb), and you should use a `Marker` component type of your own to differentiate between entities that are obstacles and those that aren't.
///
/// ```no_run
/// use bevy::{
///     prelude::*,
///     render::primitives::Aabb,
/// };
/// use vleue_navigator::prelude::*;
///
/// #[derive(Component)]
/// struct MyObstacle;
///
/// App::new().add_plugins((
///     DefaultPlugins,
///     VleueNavigatorPlugin,
///     NavmeshUpdaterPlugin::<Aabb, MyObstacle>::default(),
/// ))
/// .run();
/// ```

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

/// A diagnostic path for measuring the duration of the [`NavMesh`] build process.
pub const NAVMESH_BUILD_DURATION: DiagnosticPath =
    DiagnosticPath::const_new("navmesh_build_duration");

impl<Obstacle: ObstacleSource, Marker: Component> Plugin
    for NavmeshUpdaterPlugin<Obstacle, Marker>
{
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            trigger_navmesh_build::<Marker, Obstacle>.after(TransformSystem::TransformPropagate),
        )
        .add_systems(PreUpdate, (drop_dead_tasks, update_navmesh_asset).chain())
        .register_diagnostic(Diagnostic::new(NAVMESH_BUILD_DURATION));

        #[cfg(feature = "avian2d")]
        {
            app.add_observer(crate::obstacles::avian2d::on_sleeping_inserted)
                .add_observer(crate::obstacles::avian2d::on_sleeping_removed);
        }
        #[cfg(feature = "avian3d")]
        {
            app.add_observer(crate::obstacles::avian3d::on_sleeping_inserted)
                .add_observer(crate::obstacles::avian3d::on_sleeping_removed);
        }
    }
}
