use std::{
    collections::VecDeque,
    sync::{Arc, RwLock},
    time::Duration,
};

use bevy::{
    color::palettes,
    app::TaskPoolThreadAssignmentPolicy,
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    math::Vec3Swizzles,
    prelude::*,
    tasks::AsyncComputeTaskPool,
    platform_support::time::Instant,
    window::{PrimaryWindow, WindowResized},
};
use rand::prelude::*;

use vleue_navigator::{NavMesh, VleueNavigatorPlugin};

fn main() {
    App::new()
        .insert_resource(ClearColor(palettes::css::BLACK.into()))
        .add_plugins((
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Navmesh with Polyanya".to_string(),
                        fit_canvas_to_parent: true,
                        ..default()
                    }),
                    ..default()
                }) // This example will be async heavy, increase the default threadpool
                .set(TaskPoolPlugin {
                    task_pool_options: TaskPoolOptions {
                        async_compute: TaskPoolThreadAssignmentPolicy {
                            min_threads: 1,
                            max_threads: usize::MAX,
                            percent: 1.0,
                            on_thread_spawn: None,
                            on_thread_destroy: None,
                        },
                        ..default()
                    },
                }),
            FrameTimeDiagnosticsPlugin::default(),
            LogDiagnosticsPlugin::default(),
            VleueNavigatorPlugin,
        ))
        .init_resource::<Stats>()
        .insert_resource(TaskMode::Blocking)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                on_mesh_change,
                go_somewhere,
                compute_paths,
                poll_path_tasks,
                move_navigator,
                mode_change,
            ),
        )
        .add_systems(FixedUpdate, (spawn, update_ui))
        .insert_resource(Time::<Fixed>::from_seconds(0.1))
        .run();
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Resource)]
enum TaskMode {
    Async,
    Blocking,
}

#[derive(Resource)]
struct Meshes {
    aurora: Handle<NavMesh>,
}

const MESH_SIZE: Vec2 = Vec2::new(1024.0, 768.0);

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);
    commands.insert_resource(Meshes {
        aurora: asset_server.load("aurora-merged.polyanya.mesh"),
    });
    commands
        .spawn((
            Text::default(),
            Node {
                position_type: PositionType::Absolute,
                margin: UiRect {
                    top: Val::Px(5.0),
                    left: Val::Px(5.0),
                    ..default()
                },
                ..default()
            },
        ))
        .with_children(|p| {
            [
                ("Agents: ", 30.0),
                ("0\n", 30.0),
                ("FPS: ", 20.0),
                ("0.0\n", 20.0),
                ("Task duration: ", 20.0),
                ("0.0\n", 20.0),
                ("Task overhead: ", 20.0),
                ("0.0\n", 20.0),
                ("space: ", 15.0),
                ("\n", 15.0),
            ]
            .into_iter()
            .for_each(|(text, font_size)| {
                p.spawn((
                    TextSpan::new(text.to_string()),
                    TextFont {
                        font_size,
                        ..default()
                    },
                ));
            });
        });
}

fn on_mesh_change(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    navmeshes: Res<Assets<NavMesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    known_meshes: Res<Meshes>,
    mut current_mesh_entity: Local<Option<Entity>>,
    primary_window: Query<&Window, With<PrimaryWindow>>,
    window_resized: EventReader<WindowResized>,
    mut wait_for_mesh: Local<bool>,
) {
    if !window_resized.is_empty() || *wait_for_mesh {
        let handle = &known_meshes.aurora;
        if let Some(navmesh) = navmeshes.get(handle) {
            *wait_for_mesh = false;
            if let Some(entity) = *current_mesh_entity {
                commands.entity(entity).despawn();
            }
            let window = primary_window.single().unwrap();
            let factor = (window.width() / MESH_SIZE.x).min(window.height() / MESH_SIZE.y);
            *current_mesh_entity = Some(
                commands
                    .spawn((
                        Mesh2d(meshes.add(navmesh.to_mesh()).into()),
                        Transform::from_translation(Vec3::new(
                            -MESH_SIZE.x / 2.0 * factor,
                            -MESH_SIZE.y / 2.0 * factor,
                            0.0,
                        ))
                        .with_scale(Vec3::splat(factor)),
                        MeshMaterial2d(materials.add(ColorMaterial::from(Color::Srgba(
                            palettes::tailwind::ZINC_700,
                        )))),
                    ))
                    .id(),
            );
        } else {
            *wait_for_mesh = true;
        }
    }
}

#[derive(Component)]
struct Navigator {
    speed: f32,
}

#[derive(Component)]
struct Target {
    target: Vec2,
}

#[derive(Component)]
struct Path {
    path: Vec<Vec2>,
}

fn spawn(
    primary_window: Query<&Window, With<PrimaryWindow>>,
    mut commands: Commands,
    navmeshes: Res<Assets<NavMesh>>,
    known_meshes: Res<Meshes>,
) {
    if navmeshes.contains(&known_meshes.aurora) {
        let window = primary_window.single().unwrap();
        let mut rng = rand::thread_rng();
        let screen = Vec2::new(window.width(), window.height());
        let factor = (screen.x / MESH_SIZE.x).min(screen.y / MESH_SIZE.y);

        #[cfg(target_arch = "wasm32")]
        let per_update = 20;
        #[cfg(not(target_arch = "wasm32"))]
        let per_update = 100;

        let mut to_spawn = Vec::with_capacity(per_update);
        for _ in 0..per_update {
            let in_mesh = *[
                Vec2::new(575.0, 410.0),
                Vec2::new(387.0, 524.0),
                Vec2::new(762.0, 692.0),
                Vec2::new(991.0, 426.0),
                Vec2::new(746.0, 241.0),
                Vec2::new(391.0, 231.0),
                Vec2::new(25.0, 433.0),
                Vec2::new(300.0, 679.0),
            ]
            .choose(&mut rng)
            .unwrap();
            let position = (in_mesh - MESH_SIZE / 2.0) * factor;
            let color = Hsla::hsl(rng.gen_range(0.0..360.0), 1.0, 0.5);

            to_spawn.push((
                Sprite {
                    color: Color::Srgba(color.into()),
                    custom_size: Some(Vec2::ONE),
                    ..default()
                },
                Transform::from_translation(position.extend(1.0)).with_scale(Vec3::splat(5.0)),
                Navigator {
                    speed: rng.gen_range(50.0..100.0),
                },
            ));
        }
        commands.spawn_batch(to_spawn);
    }
}

#[derive(Default)]
struct TaskResult {
    path: Option<polyanya::Path>,
    done: bool,
    delay: f32,
    duration: f32,
}

#[derive(Component)]
struct FindingPath(Arc<RwLock<TaskResult>>);

fn compute_paths(
    mut commands: Commands,
    with_target: Query<(Entity, &Target, &Transform), Changed<Target>>,
    meshes: Res<Assets<NavMesh>>,
    primary_window: Query<&Window, With<PrimaryWindow>>,
    task_mode: Res<TaskMode>,
    mesh: Res<Meshes>,
) {
    let mesh = if let Some(mesh) = meshes.get(&mesh.aurora) {
        mesh
    } else {
        return;
    };
    for (entity, target, transform) in &with_target {
        let window = primary_window.single().unwrap();
        let factor = (window.width() / MESH_SIZE.x).min(window.height() / MESH_SIZE.y);

        let in_mesh = transform.translation.truncate() / factor + MESH_SIZE / 2.0;

        let to = target.target;
        let mesh = mesh.clone();
        let finding = FindingPath(Arc::new(RwLock::new(TaskResult::default())));
        let writer = finding.0.clone();
        let start = Instant::now();
        let task_mode = *task_mode;
        AsyncComputeTaskPool::get()
            .spawn(async move {
                let delay = (Instant::now() - start).as_secs_f32();
                let path = if task_mode == TaskMode::Async {
                    mesh.get_path(in_mesh, to).await
                } else {
                    mesh.path(in_mesh, to)
                };
                *writer.write().unwrap() = TaskResult {
                    path,
                    done: true,
                    delay,
                    duration: (Instant::now() - start).as_secs_f32() - delay,
                };
            })
            .detach();
        commands.entity(entity).insert(finding);
    }
}

#[derive(Resource, Default)]
struct Stats {
    pathfinding_duration: VecDeque<f32>,
    task_delay: VecDeque<f32>,
}

fn poll_path_tasks(
    mut commands: Commands,
    computing: Query<(Entity, &FindingPath, &Transform)>,
    mut stats: ResMut<Stats>,
    navmeshes: Res<Assets<NavMesh>>,
    meshes: Res<Meshes>,
    primary_window: Query<&Window, With<PrimaryWindow>>,
) {
    for (entity, task, transform) in &computing {
        let mut task = task.0.write().unwrap();
        if task.done {
            stats.pathfinding_duration.push_front(task.duration);
            stats.pathfinding_duration.truncate(100);
            stats.task_delay.push_front(task.delay);
            stats.task_delay.truncate(100);
            if let Some(path) = task.path.take() {
                commands
                    .entity(entity)
                    .insert(Path { path: path.path })
                    .remove::<FindingPath>();
            } else {
                let window = primary_window.single().unwrap();
                let screen = Vec2::new(window.width(), window.height());
                let factor = (screen.x / MESH_SIZE.x).min(screen.y / MESH_SIZE.y);

                if !navmeshes
                    .get(&meshes.aurora)
                    .unwrap()
                    .is_in_mesh(transform.translation.xy() / factor + MESH_SIZE / 2.0)
                {
                    commands.entity(entity).despawn();
                }

                commands
                    .entity(entity)
                    .remove::<FindingPath>()
                    .remove::<Target>();
            }
        }
    }
}

fn move_navigator(
    mut query: Query<(Entity, &mut Transform, &mut Path, &Navigator)>,
    primary_window: Query<&Window, With<PrimaryWindow>>,
    time: Res<Time>,
    par_commands: ParallelCommands,
) {
    let window = primary_window.single().unwrap();
    let factor = (window.width() / MESH_SIZE.x).min(window.height() / MESH_SIZE.y);
    query
        .par_iter_mut()
        .for_each(|(entity, mut transform, mut path, navigator)| {
            let next = (path.path[0] - MESH_SIZE / 2.0) * factor;
            let toward = next - transform.translation.xy();
            // TODO: compare this in mesh dimensions, not in display dimensions
            if toward.length() < time.delta_secs() * navigator.speed * 2.0 {
                path.path.remove(0);
                if path.path.is_empty() {
                    par_commands.command_scope(|mut commands| {
                        commands.entity(entity).remove::<Path>().remove::<Target>();
                    });
                }
            }
            transform.translation +=
                (toward.normalize() * time.delta_secs() * navigator.speed).extend(0.0);
        });
}

fn go_somewhere(
    query: Query<
        Entity,
        (
            With<Navigator>,
            Without<Path>,
            Without<FindingPath>,
            Without<Target>,
        ),
    >,
    mut commands: Commands,
) {
    let mut rng = rand::thread_rng();
    for navigator in &query {
        let target = Vec2::new(
            rng.gen_range(0.0..MESH_SIZE.x),
            rng.gen_range(0.0..MESH_SIZE.y),
        );
        commands.entity(navigator).insert(Target { target });
    }
}

fn update_ui(
    ui_query: Query<Entity, With<Text>>,
    mut text_writer: TextUiWriter,
    agents: Query<&Navigator>,
    mut count: Local<usize>,
    stats: Res<Stats>,
    diagnostics: Res<DiagnosticsStore>,
    task_mode: Res<TaskMode>,
) {
    let new_count = agents.iter().len();
    let text = ui_query.single().unwrap();
    *text_writer.text(text, 2) = format!("{}\n", new_count);
    *text_writer.text(text, 4) = format!(
        "{:.2}\n",
        diagnostics
            .get(&FrameTimeDiagnosticsPlugin::FPS)
            .and_then(|d| d.average())
            .unwrap_or_default()
    );

    *text_writer.text(text, 6) = format!(
        "{:?}\n",
        Duration::from_secs_f32(
            stats.pathfinding_duration.iter().sum::<f32>()
                / (stats.pathfinding_duration.len().max(1) as f32)
        ),
    );
    *text_writer.text(text, 8) = format!(
        "{:?}\n",
        Duration::from_secs_f32(
            stats.task_delay.iter().sum::<f32>() / (stats.task_delay.len().max(1) as f32)
        )
    );
    *text_writer.text(text, 10) = format!("{:?}\n", *task_mode);
    *count = new_count;
}

fn mode_change(keyboard_input: Res<ButtonInput<KeyCode>>, mut task_mode: ResMut<TaskMode>) {
    if keyboard_input.just_pressed(KeyCode::Space) {
        match *task_mode {
            TaskMode::Async => *task_mode = TaskMode::Blocking,
            TaskMode::Blocking => *task_mode = TaskMode::Async,
        }
    }
}
