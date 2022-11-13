use std::{
    collections::VecDeque,
    sync::{Arc, RwLock},
    time::Duration,
};

use bevy::{
    core::TaskPoolThreadAssignmentPolicy,
    diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    math::Vec3Swizzles,
    prelude::*,
    sprite::MaterialMesh2dBundle,
    tasks::AsyncComputeTaskPool,
    time::FixedTimestep,
    utils::Instant,
    window::WindowResized,
};
use rand::prelude::*;

use bevy_pathmesh::{PathMesh, PathmeshPlugin};
use bevy_prototype_debug_lines::{DebugLines, DebugLinesPlugin};

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    window: WindowDescriptor {
                        title: "Navmesh with Polyanya".to_string(),
                        fit_canvas_to_parent: true,
                        ..default()
                    },
                    ..default()
                })
                // This example will be async heavy, increase the default threadpool
                .set(CorePlugin {
                    task_pool_options: TaskPoolOptions {
                        async_compute: TaskPoolThreadAssignmentPolicy {
                            min_threads: 1,
                            max_threads: usize::MAX,
                            percent: 1.0,
                        },
                        ..default()
                    },
                }),
        )
        .add_plugin(DebugLinesPlugin::default())
        .add_plugin(PathmeshPlugin)
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_plugin(LogDiagnosticsPlugin::default())
        .init_resource::<Stats>()
        .insert_resource(TaskMode::Blocking)
        .insert_resource(DisplayMode::Line)
        .add_startup_system(setup)
        .add_system(on_mesh_change)
        .add_system(go_somewhere)
        .add_system(compute_paths)
        .add_system(poll_path_tasks)
        .add_system(move_navigator)
        .add_system(display_path)
        .add_system(mode_change)
        .add_system_set(
            SystemSet::new()
                .with_run_criteria(FixedTimestep::step(0.1))
                .with_system(spawn)
                .with_system(update_ui),
        )
        .run();
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Resource)]
enum TaskMode {
    Async,
    Blocking,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Resource)]
enum DisplayMode {
    Line,
    Nothing,
}

#[derive(Resource)]
struct Meshes {
    aurora: Handle<PathMesh>,
}

const MESH_SIZE: Vec2 = Vec2::new(1024.0, 768.0);

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2dBundle::default());
    commands.insert_resource(Meshes {
        aurora: asset_server.load("aurora-merged.polyanya.mesh"),
    });
    let font = asset_server.load("fonts/FiraMono-Medium.ttf");
    commands.spawn(TextBundle {
        text: Text::from_sections([
            TextSection::new(
                "Agents: ",
                TextStyle {
                    font: font.clone_weak(),
                    font_size: 30.0,
                    color: Color::WHITE,
                },
            ),
            TextSection::new(
                "0\n",
                TextStyle {
                    font: font.clone_weak(),
                    font_size: 30.0,
                    color: Color::WHITE,
                },
            ),
            TextSection::new(
                "FPS: ",
                TextStyle {
                    font: font.clone_weak(),
                    font_size: 20.0,
                    color: Color::WHITE,
                },
            ),
            TextSection::new(
                "0.0\n",
                TextStyle {
                    font: font.clone_weak(),
                    font_size: 20.0,
                    color: Color::WHITE,
                },
            ),
            TextSection::new(
                "Task duration: ",
                TextStyle {
                    font: font.clone_weak(),
                    font_size: 20.0,
                    color: Color::WHITE,
                },
            ),
            TextSection::new(
                "0.0\n",
                TextStyle {
                    font: font.clone_weak(),
                    font_size: 20.0,
                    color: Color::WHITE,
                },
            ),
            TextSection::new(
                "Task overhead: ",
                TextStyle {
                    font: font.clone_weak(),
                    font_size: 20.0,
                    color: Color::WHITE,
                },
            ),
            TextSection::new(
                "0.0\n",
                TextStyle {
                    font: font.clone_weak(),
                    font_size: 20.0,
                    color: Color::WHITE,
                },
            ),
            TextSection::new(
                "space - ",
                TextStyle {
                    font: font.clone_weak(),
                    font_size: 15.0,
                    color: Color::WHITE,
                },
            ),
            TextSection::new(
                "\n",
                TextStyle {
                    font: font.clone_weak(),
                    font_size: 15.0,
                    color: Color::WHITE,
                },
            ),
            TextSection::new(
                "l - ",
                TextStyle {
                    font: font.clone_weak(),
                    font_size: 15.0,
                    color: Color::WHITE,
                },
            ),
            TextSection::new(
                "\n",
                TextStyle {
                    font,
                    font_size: 15.0,
                    color: Color::WHITE,
                },
            ),
        ]),
        style: Style {
            position_type: PositionType::Absolute,
            position: UiRect {
                top: Val::Px(5.0),
                left: Val::Px(5.0),
                ..default()
            },
            ..default()
        },
        ..default()
    });
}

fn on_mesh_change(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    pathmeshes: Res<Assets<PathMesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    path_meshes: Res<Meshes>,
    mut current_mesh_entity: Local<Option<Entity>>,
    windows: Res<Windows>,
    window_resized: EventReader<WindowResized>,
    mut wait_for_mesh: Local<bool>,
) {
    if !window_resized.is_empty() || *wait_for_mesh {
        let handle = &path_meshes.aurora;
        if let Some(pathmesh) = pathmeshes.get(handle) {
            *wait_for_mesh = false;
            if let Some(entity) = *current_mesh_entity {
                commands.entity(entity).despawn();
            }
            let window = windows.primary();
            let factor = (window.width() / MESH_SIZE.x).min(window.height() / MESH_SIZE.y);
            *current_mesh_entity = Some(
                commands
                    .spawn(MaterialMesh2dBundle {
                        mesh: meshes.add(pathmesh.to_mesh()).into(),
                        transform: Transform::from_translation(Vec3::new(
                            -MESH_SIZE.x / 2.0 * factor,
                            -MESH_SIZE.y / 2.0 * factor,
                            0.0,
                        ))
                        .with_scale(Vec3::splat(factor)),
                        material: materials.add(ColorMaterial::from(Color::DARK_GRAY)),
                        ..default()
                    })
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
    color: Color,
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
    windows: Res<Windows>,
    mut commands: Commands,
    pathmeshes: Res<Assets<PathMesh>>,
    path_meshes: Res<Meshes>,
) {
    if pathmeshes.contains(&path_meshes.aurora) {
        let mut rng = rand::thread_rng();
        let screen = Vec2::new(windows.primary().width(), windows.primary().height());
        let factor = (screen.x / MESH_SIZE.x).min(screen.y / MESH_SIZE.y);

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
        let color = Color::hsl(rng.gen_range(0.0..360.0), 1.0, 0.5).as_rgba();
        commands.spawn((
            SpriteBundle {
                sprite: Sprite {
                    color,
                    custom_size: Some(Vec2::ONE),
                    ..default()
                },
                transform: Transform::from_translation(position.extend(1.0))
                    .with_scale(Vec3::splat(5.0)),
                ..default()
            },
            Navigator {
                speed: rng.gen_range(50.0..100.0),
                color,
            },
        ));
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
    meshes: Res<Assets<PathMesh>>,
    windows: Res<Windows>,
    task_mode: Res<TaskMode>,
    mesh: Res<Meshes>,
) {
    let mesh = if let Some(mesh) = meshes.get(&mesh.aurora) {
        mesh
    } else {
        return;
    };
    for (entity, target, transform) in &with_target {
        let window = windows.primary();
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
    pathmeshes: Res<Assets<PathMesh>>,
    meshes: Res<Meshes>,
    windows: Res<Windows>,
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
                let screen = Vec2::new(windows.primary().width(), windows.primary().height());
                let factor = (screen.x / MESH_SIZE.x).min(screen.y / MESH_SIZE.y);

                if !pathmeshes
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
    windows: Res<Windows>,
    time: Res<Time>,
    mut commands: Commands,
) {
    let window = windows.primary();
    let factor = (window.width() / MESH_SIZE.x).min(window.height() / MESH_SIZE.y);
    for (entity, mut transform, mut path, navigator) in &mut query {
        let next = (path.path[0] - MESH_SIZE / 2.0) * factor;
        let toward = next - transform.translation.xy();
        // TODO: compare this in mesh dimensions, not in display dimensions
        if toward.length() < time.delta_seconds() * navigator.speed * 2.0 {
            path.path.remove(0);
            if path.path.is_empty() {
                commands.entity(entity).remove::<Path>().remove::<Target>();
            }
        }
        transform.translation +=
            (toward.normalize() * time.delta_seconds() * navigator.speed).extend(0.0);
    }
}

fn display_path(
    query: Query<(&Transform, &Path, &Navigator)>,
    mut lines: ResMut<DebugLines>,
    windows: Res<Windows>,
    display_mode: Res<DisplayMode>,
) {
    if *display_mode == DisplayMode::Line {
        let window = windows.primary();
        let factor = (window.width() / MESH_SIZE.x).min(window.height() / MESH_SIZE.y);

        for (transform, path, navigator) in &query {
            (1..path.path.len()).for_each(|i| {
                lines.line_colored(
                    ((path.path[i - 1] - MESH_SIZE / 2.0) * factor).extend(0f32),
                    ((path.path[i] - MESH_SIZE / 2.0) * factor).extend(0f32),
                    0f32,
                    navigator.color,
                );
            });
            if let Some(next) = path.path.first() {
                lines.line_colored(
                    transform.translation,
                    ((*next - MESH_SIZE / 2.0) * factor).extend(0f32),
                    0f32,
                    navigator.color,
                );
            }
        }
    }
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
        commands.entity(navigator).insert(Target { target: target });
    }
}

fn update_ui(
    mut ui_query: Query<&mut Text>,
    agents: Query<&Navigator>,
    mut count: Local<usize>,
    stats: Res<Stats>,
    diagnostics: Res<Diagnostics>,
    task_mode: Res<TaskMode>,
    display_mode: Res<DisplayMode>,
) {
    let new_count = agents.iter().len();
    let mut text = ui_query.single_mut();
    text.sections[1].value = format!("{}\n", new_count);
    text.sections[3].value = format!(
        "{:.2}\n",
        diagnostics
            .get(FrameTimeDiagnosticsPlugin::FPS)
            .and_then(|d| d.average())
            .unwrap_or_default()
    );

    text.sections[5].value = format!(
        "{:?}\n",
        Duration::from_secs_f32(
            stats.pathfinding_duration.iter().sum::<f32>()
                / (stats.pathfinding_duration.len().max(1) as f32)
        ),
    );
    text.sections[7].value = format!(
        "{:?}\n",
        Duration::from_secs_f32(
            stats.task_delay.iter().sum::<f32>() / (stats.task_delay.len().max(1) as f32)
        )
    );
    text.sections[9].value = format!("{:?}\n", *task_mode);
    text.sections[11].value = format!(
        "{}",
        match *display_mode {
            DisplayMode::Line => "hide lines",
            DisplayMode::Nothing => "display lines",
        }
    );
    *count = new_count;
}

fn mode_change(
    keyboard_input: Res<Input<KeyCode>>,
    mut task_mode: ResMut<TaskMode>,
    mut display_mode: ResMut<DisplayMode>,
) {
    if keyboard_input.just_pressed(KeyCode::Space) {
        match *task_mode {
            TaskMode::Async => *task_mode = TaskMode::Blocking,
            TaskMode::Blocking => *task_mode = TaskMode::Async,
        }
    }
    if keyboard_input.just_pressed(KeyCode::L) {
        match *display_mode {
            DisplayMode::Line => *display_mode = DisplayMode::Nothing,
            DisplayMode::Nothing => *display_mode = DisplayMode::Line,
        }
    }
}
