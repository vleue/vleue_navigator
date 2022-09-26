use std::{
    collections::VecDeque,
    sync::{Arc, RwLock},
};

use bevy::{
    core::TaskPoolThreadAssignmentPolicy, math::Vec3Swizzles, prelude::*,
    sprite::MaterialMesh2dBundle, tasks::AsyncComputeTaskPool, time::FixedTimestep, utils::Instant,
    window::WindowResized,
};
use rand::prelude::*;

use bevy_pathmesh::{PathMesh, PathmeshPlugin};
use bevy_prototype_debug_lines::{DebugLines, DebugLinesPlugin};

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(WindowDescriptor {
            title: "Navmesh with Polyanya".to_string(),
            fit_canvas_to_parent: true,
            ..default()
        })
        // This example will be async heavy, increase the default threadpool
        .insert_resource(DefaultTaskPoolOptions {
            async_compute: TaskPoolThreadAssignmentPolicy {
                min_threads: 1,
                max_threads: usize::MAX,
                percent: 1.0,
            },
            ..default()
        })
        .add_plugins(DefaultPlugins)
        .add_plugin(DebugLinesPlugin::default())
        .add_plugin(PathmeshPlugin)
        .init_resource::<Stats>()
        .add_startup_system(setup)
        .add_system(on_mesh_change)
        .add_system(go_somewhere)
        .add_system(compute_paths)
        .add_system(poll_path_tasks)
        .add_system(move_navigator)
        .add_system(display_path)
        .add_system(update_ui)
        .add_system_set(
            SystemSet::new()
                .with_run_criteria(FixedTimestep::step(0.5))
                .with_system(spawn),
        )
        .run();
}

struct Meshes {
    aurora: Handle<PathMesh>,
}

const MESH_SIZE: Vec2 = Vec2::new(1024.0, 768.0);

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn_bundle(Camera2dBundle::default());
    commands.insert_resource(Meshes {
        aurora: asset_server.load("aurora-merged.polyanya.mesh"),
    });
    let font = asset_server.load("fonts/FiraMono-Medium.ttf");
    commands.spawn_bundle(TextBundle {
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
                "Task duration: ",
                TextStyle {
                    font: font.clone_weak(),
                    font_size: 30.0,
                    color: Color::WHITE,
                },
            ),
            TextSection::new(
                "0.0",
                TextStyle {
                    font,
                    font_size: 30.0,
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
    navigator: Query<Entity, With<Navigator>>,
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
            if let Ok(entity) = navigator.get_single() {
                commands.entity(entity).despawn();
            }
            let window = windows.primary();
            let factor = (window.width() / MESH_SIZE.x).min(window.height() / MESH_SIZE.y);
            *current_mesh_entity = Some(
                commands
                    .spawn_bundle(MaterialMesh2dBundle {
                        mesh: meshes.add(pathmesh.blocking().to_mesh()).into(),
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
    pathmesh: Handle<PathMesh>,
}

#[derive(Component)]
struct Path {
    path: Vec<Vec2>,
}

fn spawn(
    windows: Res<Windows>,
    meshes: Res<Meshes>,
    mut commands: Commands,
    pathmeshes: Res<Assets<PathMesh>>,
) {
    let mut rng = rand::thread_rng();
    let screen = Vec2::new(windows.primary().width(), windows.primary().height());
    let factor = (screen.x / MESH_SIZE.x).min(screen.y / MESH_SIZE.y);

    let in_mesh = Vec2::new(575.0, 410.0);
    let position = (in_mesh - MESH_SIZE / 2.0) * factor;
    if pathmeshes
        .get(&meshes.aurora)
        .map(|mesh| mesh.blocking().is_in_mesh(in_mesh))
        .unwrap_or_default()
    {
        info!("spawning at {}", in_mesh);
        let color = Color::hsl(
            rng.gen_range(0.0..360.0),
            rng.gen_range(0.0..1.0),
            rng.gen_range(0.5..1.0),
        )
        .as_rgba();
        commands
            .spawn_bundle(SpriteBundle {
                sprite: Sprite {
                    color,
                    custom_size: Some(Vec2::ONE),
                    ..default()
                },
                transform: Transform::from_translation(position.extend(1.0))
                    .with_scale(Vec3::splat(5.0)),
                ..default()
            })
            .insert(Navigator {
                speed: rng.gen_range(50.0..100.0),
                color,
            });
    } else {
        info!("clicked outside of mesh");
    }
}

#[derive(Component)]
struct FindingPath(Arc<RwLock<(Option<polyanya::Path>, bool, f32)>>);

fn compute_paths(
    mut commands: Commands,
    with_target: Query<(Entity, &Target, &Transform), Changed<Target>>,
    meshes: Res<Assets<PathMesh>>,
    windows: Res<Windows>,
) {
    for (entity, target, transform) in &with_target {
        let window = windows.primary();
        let factor = (window.width() / MESH_SIZE.x).min(window.height() / MESH_SIZE.y);

        let in_mesh = transform.translation.truncate() / factor + MESH_SIZE / 2.0;
        let mesh = meshes.get(&target.pathmesh).unwrap();

        let to = target.target;
        let mesh = mesh.clone();
        let finding = FindingPath(Arc::new(RwLock::new((None, false, 0.0))));
        let writer = finding.0.clone();
        let start = Instant::now();
        AsyncComputeTaskPool::get()
            .spawn(async move {
                let path = mesh.path(in_mesh, to).await;
                *writer.write().unwrap() = (path, true, (Instant::now() - start).as_secs_f32());
            })
            .detach();
        commands.entity(entity).insert(finding);
    }
}

#[derive(Default)]
struct Stats {
    pathfinding_duration: VecDeque<f32>,
}

fn poll_path_tasks(
    mut commands: Commands,
    computing: Query<(Entity, &FindingPath)>,
    mut stats: ResMut<Stats>,
) {
    for (entity, task) in &computing {
        let mut task = task.0.write().unwrap();
        if task.1 {
            stats.pathfinding_duration.push_front(task.2);
            stats.pathfinding_duration.truncate(100);
            if let Some(path) = task.0.take() {
                commands
                    .entity(entity)
                    .insert(Path { path: path.path })
                    .remove::<FindingPath>();
            } else {
                commands
                    .entity(entity)
                    .remove::<FindingPath>()
                    .remove::<Target>();
                info!("no path found");
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
                debug!("reached target");
                commands.entity(entity).remove::<Path>().remove::<Target>();
            } else {
                debug!("reached next step");
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
) {
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
    meshes: Res<Meshes>,
) {
    let mut rng = rand::thread_rng();
    for navigator in &query {
        let target = Vec2::new(
            rng.gen_range(0.0..MESH_SIZE.x),
            rng.gen_range(0.0..MESH_SIZE.y),
        );
        info!("going to {}", target);
        commands.entity(navigator).insert(Target {
            target: target,
            pathmesh: meshes.aurora.clone_weak(),
        });
    }
}

fn update_ui(
    mut ui_query: Query<&mut Text>,
    agents: Query<&Navigator>,
    mut count: Local<usize>,
    stats: Res<Stats>,
) {
    let new_count = agents.iter().len();
    if *count != new_count {
        let mut text = ui_query.single_mut();
        text.sections[1].value = format!("{}\n", new_count);
        text.sections[3].value = format!(
            "{:.2}",
            stats.pathfinding_duration.iter().sum::<f32>()
                / (stats.pathfinding_duration.len() as f32)
        );
        *count = new_count;
    }
}
