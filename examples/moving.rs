use std::{
    sync::{Arc, RwLock},
    time::Duration,
};

use bevy::{
    color::palettes,
    math::Vec3Swizzles,
    prelude::*,
    tasks::AsyncComputeTaskPool,
    window::{PrimaryWindow, WindowResized},
};

use vleue_navigator::{NavMesh, VleueNavigatorPlugin};

fn main() {
    App::new()
        .insert_resource(ClearColor(palettes::css::BLACK.into()))
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Navmesh with Polyanya".to_string(),
                    fit_canvas_to_parent: true,
                    ..default()
                }),
                ..default()
            }),
            VleueNavigatorPlugin,
        ))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                on_mesh_change,
                mesh_change,
                on_click,
                compute_paths,
                poll_path_tasks,
                move_navigator,
                display_path,
            ),
        )
        .run();
}

#[derive(Resource)]
struct Meshes {
    simple: Handle<NavMesh>,
    arena: Handle<NavMesh>,
    aurora: Handle<NavMesh>,
}

enum CurrentMesh {
    Simple,
    Arena,
    Aurora,
}

#[derive(Resource)]
struct MeshDetails {
    mesh: CurrentMesh,
    size: Vec2,
}

const SIMPLE: MeshDetails = MeshDetails {
    mesh: CurrentMesh::Simple,
    size: Vec2::new(13.0, 8.0),
};

const ARENA: MeshDetails = MeshDetails {
    mesh: CurrentMesh::Arena,
    size: Vec2::new(49.0, 49.0),
};

const AURORA: MeshDetails = MeshDetails {
    mesh: CurrentMesh::Aurora,
    size: Vec2::new(1024.0, 768.0),
};

fn setup(
    mut commands: Commands,
    mut navmeshes: ResMut<Assets<NavMesh>>,
    asset_server: Res<AssetServer>,
) {
    commands.spawn(Camera2d);
    commands.insert_resource(Meshes {
        simple: navmeshes.add(NavMesh::from_polyanya_mesh(
            polyanya::Mesh::new(
                vec![
                    polyanya::Vertex::new(Vec2::new(0., 6.), vec![0, u32::MAX]),
                    polyanya::Vertex::new(Vec2::new(2., 5.), vec![0, u32::MAX, 2]),
                    polyanya::Vertex::new(Vec2::new(5., 7.), vec![0, 2, u32::MAX]),
                    polyanya::Vertex::new(Vec2::new(5., 8.), vec![0, u32::MAX]),
                    polyanya::Vertex::new(Vec2::new(0., 8.), vec![0, u32::MAX]),
                    polyanya::Vertex::new(Vec2::new(1., 4.), vec![1, u32::MAX]),
                    polyanya::Vertex::new(Vec2::new(2., 1.), vec![1, u32::MAX]),
                    polyanya::Vertex::new(Vec2::new(4., 1.), vec![1, u32::MAX]),
                    polyanya::Vertex::new(Vec2::new(4., 2.), vec![1, u32::MAX, 2]),
                    polyanya::Vertex::new(Vec2::new(2., 4.), vec![1, 2, u32::MAX]),
                    polyanya::Vertex::new(Vec2::new(7., 4.), vec![2, u32::MAX, 4]),
                    polyanya::Vertex::new(Vec2::new(10., 7.), vec![2, 4, 6, u32::MAX, 3]),
                    polyanya::Vertex::new(Vec2::new(7., 7.), vec![2, 3, u32::MAX]),
                    polyanya::Vertex::new(Vec2::new(11., 8.), vec![3, u32::MAX]),
                    polyanya::Vertex::new(Vec2::new(7., 8.), vec![3, u32::MAX]),
                    polyanya::Vertex::new(Vec2::new(7., 0.), vec![5, 4, u32::MAX]),
                    polyanya::Vertex::new(Vec2::new(11., 3.), vec![4, 5, u32::MAX]),
                    polyanya::Vertex::new(Vec2::new(11., 5.), vec![4, u32::MAX, 6]),
                    polyanya::Vertex::new(Vec2::new(12., 0.), vec![5, u32::MAX]),
                    polyanya::Vertex::new(Vec2::new(12., 3.), vec![5, u32::MAX]),
                    polyanya::Vertex::new(Vec2::new(13., 5.), vec![6, u32::MAX]),
                    polyanya::Vertex::new(Vec2::new(13., 7.), vec![6, u32::MAX]),
                    polyanya::Vertex::new(Vec2::new(1., 3.), vec![1, u32::MAX]),
                ],
                vec![
                    polyanya::Polygon::new(vec![0, 1, 2, 3, 4], true),
                    polyanya::Polygon::new(vec![5, 22, 6, 7, 8, 9], true),
                    polyanya::Polygon::new(vec![1, 9, 8, 10, 11, 12, 2], false),
                    polyanya::Polygon::new(vec![12, 11, 13, 14], true),
                    polyanya::Polygon::new(vec![10, 15, 16, 17, 11], false),
                    polyanya::Polygon::new(vec![15, 18, 19, 16], true),
                    polyanya::Polygon::new(vec![11, 17, 20, 21], true),
                ],
            )
            .unwrap(),
        )),
        arena: asset_server.load("arena-merged.polyanya.mesh"),
        aurora: asset_server.load("aurora-merged.polyanya.mesh"),
    });
    commands.insert_resource(AURORA);
}

fn on_mesh_change(
    mesh: Res<MeshDetails>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    navmeshes: Res<Assets<NavMesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    known_meshes: Res<Meshes>,
    mut current_mesh_entity: Local<Option<Entity>>,
    primary_window: Query<&Window, With<PrimaryWindow>>,
    navigator: Query<Entity, With<Navigator>>,
    window_resized: EventReader<WindowResized>,
    text: Query<Entity, With<Text>>,
    mut wait_for_mesh: Local<bool>,
) {
    if mesh.is_changed() || !window_resized.is_empty() || *wait_for_mesh {
        let handle = match mesh.mesh {
            CurrentMesh::Simple => &known_meshes.simple,
            CurrentMesh::Arena => &known_meshes.arena,
            CurrentMesh::Aurora => &known_meshes.aurora,
        };
        if let Some(navmesh) = navmeshes.get(handle) {
            *wait_for_mesh = false;
            if let Some(entity) = *current_mesh_entity {
                commands.entity(entity).despawn();
            }
            if let Ok(entity) = navigator.single() {
                commands.entity(entity).despawn();
            }
            let window = primary_window.single().unwrap();
            let factor = (window.width() / mesh.size.x).min(window.height() / mesh.size.y);
            *current_mesh_entity = Some(
                commands
                    .spawn((
                        Mesh2d(meshes.add(navmesh.to_mesh()).into()),
                        Transform::from_translation(Vec3::new(
                            -mesh.size.x / 2.0 * factor,
                            -mesh.size.y / 2.0 * factor,
                            0.0,
                        ))
                        .with_scale(Vec3::splat(factor)),
                        MeshMaterial2d(
                            materials.add(ColorMaterial::from(Color::Srgba(palettes::css::BLUE))),
                        ),
                    ))
                    .id(),
            );
            if let Ok(entity) = text.single() {
                commands.entity(entity).despawn();
            }

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
                    p.spawn((
                        TextSpan::new(
                            match mesh.mesh {
                                CurrentMesh::Simple => "Simple\n",
                                CurrentMesh::Arena => "Arena\n",
                                CurrentMesh::Aurora => "Aurora\n",
                            }
                            .to_string(),
                        ),
                        TextFont {
                            font_size: 25.0,
                            ..default()
                        },
                    ));
                    p.spawn((
                        TextSpan::new("Press spacebar to switch mesh\n".to_string()),
                        TextFont {
                            font_size: 15.0,
                            ..default()
                        },
                    ));
                    p.spawn((
                        TextSpan::new("Click to find a path".to_string()),
                        TextFont {
                            font_size: 15.0,
                            ..default()
                        },
                    ));
                });
        } else {
            *wait_for_mesh = true;
        }
    }
}

fn mesh_change(
    mut mesh: ResMut<MeshDetails>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    time: Res<Time>,
    mut pressed_since: Local<Option<Duration>>,
) {
    let mut touch_triggered = false;
    if mouse_input.just_pressed(MouseButton::Left) {
        *pressed_since = Some(time.elapsed());
    }
    if mouse_input.just_released(MouseButton::Left) {
        *pressed_since = None;
    }
    if let Some(started) = *pressed_since {
        if (time.elapsed() - started).as_secs() > 1 {
            touch_triggered = true;
            *pressed_since = None;
        }
    }
    if keyboard_input.just_pressed(KeyCode::Space) || touch_triggered {
        match mesh.mesh {
            CurrentMesh::Simple => *mesh = ARENA,
            CurrentMesh::Arena => *mesh = AURORA,
            CurrentMesh::Aurora => *mesh = SIMPLE,
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
    navmesh: Handle<NavMesh>,
}

#[derive(Component)]
struct Path {
    path: Vec<Vec2>,
}

fn on_click(
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    primary_window: Query<&Window, With<PrimaryWindow>>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    mesh: Res<MeshDetails>,
    meshes: Res<Meshes>,
    mut commands: Commands,
    query: Query<Entity, With<Navigator>>,
    navmeshes: Res<Assets<NavMesh>>,
) {
    if mouse_button_input.just_pressed(MouseButton::Left) {
        let (camera, camera_transform) = camera_q.single().unwrap();
        let window = primary_window.single().unwrap();
        if let Some(position) = window
            .cursor_position()
            .and_then(|cursor| camera.viewport_to_world(camera_transform, cursor).ok())
            .map(|ray| ray.origin.truncate())
        {
            let screen = Vec2::new(window.width(), window.height());
            let factor = (screen.x / mesh.size.x).min(screen.y / mesh.size.y);
            let in_mesh = position / factor + mesh.size / 2.0;
            if navmeshes
                .get(match mesh.mesh {
                    CurrentMesh::Simple => &meshes.simple,
                    CurrentMesh::Arena => &meshes.arena,
                    CurrentMesh::Aurora => &meshes.aurora,
                })
                .map(|mesh| mesh.is_in_mesh(in_mesh))
                .unwrap_or_default()
            {
                if let Ok(navigator) = query.single() {
                    info!("going to {}", in_mesh);
                    commands.entity(navigator).insert(Target {
                        target: in_mesh,
                        navmesh: match mesh.mesh {
                            CurrentMesh::Simple => meshes.simple.clone_weak(),
                            CurrentMesh::Arena => meshes.arena.clone_weak(),
                            CurrentMesh::Aurora => meshes.aurora.clone_weak(),
                        },
                    });
                } else {
                    info!("spawning at {}", in_mesh);
                    commands.spawn((
                        Sprite {
                            color: palettes::css::RED.into(),
                            custom_size: Some(Vec2::ONE),
                            ..default()
                        },
                        Transform::from_translation(position.extend(1.0))
                            .with_scale(Vec3::splat(5.0)),
                        Navigator { speed: 100.0 },
                    ));
                }
            } else {
                info!("clicked outside of mesh");
            }
        }
    }
}

#[derive(Component)]
struct FindingPath(Arc<RwLock<(Option<polyanya::Path>, bool)>>);

fn compute_paths(
    mut commands: Commands,
    with_target: Query<(Entity, &Target, &Transform), Changed<Target>>,
    meshes: Res<Assets<NavMesh>>,
    mesh: Res<MeshDetails>,
    primary_window: Query<&Window, With<PrimaryWindow>>,
) {
    let window = primary_window.single().unwrap();
    let factor = (window.width() / mesh.size.x).min(window.height() / mesh.size.y);

    for (entity, target, transform) in &with_target {
        let in_mesh = transform.translation.truncate() / factor + mesh.size / 2.0;
        let mesh = meshes.get(&target.navmesh).unwrap();

        let to = target.target;
        let mesh = mesh.clone();
        let finding = FindingPath(Arc::new(RwLock::new((None, false))));
        let writer = finding.0.clone();
        AsyncComputeTaskPool::get()
            .spawn(async move {
                let path = mesh.path(in_mesh, to);
                *writer.write().unwrap() = (path, true);
            })
            .detach();
        commands.entity(entity).insert(finding);
    }
}

fn poll_path_tasks(mut commands: Commands, computing: Query<(Entity, &FindingPath)>) {
    for (entity, task) in &computing {
        let mut task = task.0.write().unwrap();
        if task.1 {
            if let Some(path) = task.0.take() {
                commands
                    .entity(entity)
                    .insert(Path { path: path.path })
                    .remove::<FindingPath>();
            } else {
                info!("no path found");
            }
        }
    }
}

fn move_navigator(
    mut query: Query<(Entity, &mut Transform, &mut Path, &Navigator)>,
    mesh: Res<MeshDetails>,
    primary_window: Query<&Window, With<PrimaryWindow>>,
    time: Res<Time>,
    mut commands: Commands,
) {
    let window = primary_window.single().unwrap();
    let factor = (window.width() / mesh.size.x).min(window.height() / mesh.size.y);
    for (entity, mut transform, mut path, navigator) in &mut query {
        let next = (path.path[0] - mesh.size / 2.0) * factor;
        let toward = next - transform.translation.xy();
        // TODO: compare this in mesh dimensions, not in display dimensions
        if toward.length() < time.delta_secs() * navigator.speed {
            path.path.remove(0);
            if path.path.is_empty() {
                debug!("reached target");
                commands.entity(entity).remove::<Path>();
            } else {
                debug!("reached next step");
            }
        }
        transform.translation +=
            (toward.normalize() * time.delta_secs() * navigator.speed).extend(0.0);
    }
}

fn display_path(
    query: Query<(&Transform, &Path)>,
    mut gizmos: Gizmos,
    mesh: Res<MeshDetails>,
    primary_window: Query<&Window, With<PrimaryWindow>>,
) {
    let window = primary_window.single().unwrap();
    let factor = (window.width() / mesh.size.x).min(window.height() / mesh.size.y);

    for (transform, path) in &query {
        if path.path.is_empty() {
            continue;
        }
        gizmos.linestrip_2d(
            path.path.iter().map(|p| (*p - mesh.size / 2.0) * factor),
            palettes::css::ORANGE,
        );

        if let Some(next) = path.path.first() {
            gizmos.line_2d(
                transform.translation.truncate(),
                (*next - mesh.size / 2.0) * factor,
                palettes::css::YELLOW,
            );
        }
    }
}
