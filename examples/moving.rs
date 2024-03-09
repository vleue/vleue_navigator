use std::{
    sync::{Arc, RwLock},
    time::Duration,
};

use bevy::{
    math::{vec2, Vec3Swizzles},
    prelude::*,
    sprite::MaterialMesh2dBundle,
    tasks::AsyncComputeTaskPool,
    window::{PrimaryWindow, WindowResized},
};

use vleue_navigator::{NavMesh, VleueNavigatorPlugin};

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Navmesh with Polyanya".to_string(),
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
    simple_triangulation: Handle<NavMesh>,
    arena: Handle<NavMesh>,
    arena_triangulation: Handle<NavMesh>,
    aurora: Handle<NavMesh>,
}

enum CurrentMesh {
    Simple,
    SimpleTriangulation,
    Arena,
    ArenaTriangulation,
    Aurora,
}

#[derive(Resource)]
struct MeshDetails {
    mesh: CurrentMesh,
    size: Vec2,
    with_wireframe: bool,
}

#[derive(Component)]
pub struct WireframeMesh;

const SIMPLE: MeshDetails = MeshDetails {
    mesh: CurrentMesh::Simple,
    size: Vec2::new(13.0, 8.0),
    with_wireframe: false,
};

const SIMPLE_TRIANGULATION: MeshDetails = MeshDetails {
    mesh: CurrentMesh::SimpleTriangulation,
    size: Vec2::new(13.0, 8.0),
    with_wireframe: false,
};

const ARENA: MeshDetails = MeshDetails {
    mesh: CurrentMesh::Arena,
    size: Vec2::new(49.0, 49.0),
    with_wireframe: false,
};

const ARENA_TRIANGULATION: MeshDetails = MeshDetails {
    mesh: CurrentMesh::ArenaTriangulation,
    size: Vec2::new(49.0, 49.0),
    with_wireframe: false,
};

const AURORA: MeshDetails = MeshDetails {
    mesh: CurrentMesh::Aurora,
    size: Vec2::new(1024.0, 768.0),
    with_wireframe: false,
};

fn setup(
    mut commands: Commands,
    mut navmeshes: ResMut<Assets<NavMesh>>,
    asset_server: Res<AssetServer>,
) {
    commands.spawn(Camera2dBundle::default());
    commands.insert_resource(Meshes {
        simple: navmeshes.add(NavMesh::from_polyanya_mesh(polyanya::Mesh::new(
            vec![
                polyanya::Vertex::new(Vec2::new(0., 6.), vec![0, -1]),
                polyanya::Vertex::new(Vec2::new(2., 5.), vec![0, -1, 2]),
                polyanya::Vertex::new(Vec2::new(5., 7.), vec![0, 2, -1]),
                polyanya::Vertex::new(Vec2::new(5., 8.), vec![0, -1]),
                polyanya::Vertex::new(Vec2::new(0., 8.), vec![0, -1]),
                polyanya::Vertex::new(Vec2::new(1., 4.), vec![1, -1]),
                polyanya::Vertex::new(Vec2::new(2., 1.), vec![1, -1]),
                polyanya::Vertex::new(Vec2::new(4., 1.), vec![1, -1]),
                polyanya::Vertex::new(Vec2::new(4., 2.), vec![1, -1, 2]),
                polyanya::Vertex::new(Vec2::new(2., 4.), vec![1, 2, -1]),
                polyanya::Vertex::new(Vec2::new(7., 4.), vec![2, -1, 4]),
                polyanya::Vertex::new(Vec2::new(10., 7.), vec![2, 4, 6, -1, 3]),
                polyanya::Vertex::new(Vec2::new(7., 7.), vec![2, 3, -1]),
                polyanya::Vertex::new(Vec2::new(11., 8.), vec![3, -1]),
                polyanya::Vertex::new(Vec2::new(7., 8.), vec![3, -1]),
                polyanya::Vertex::new(Vec2::new(7., 0.), vec![5, 4, -1]),
                polyanya::Vertex::new(Vec2::new(11., 3.), vec![4, 5, -1]),
                polyanya::Vertex::new(Vec2::new(11., 5.), vec![4, -1, 6]),
                polyanya::Vertex::new(Vec2::new(12., 0.), vec![5, -1]),
                polyanya::Vertex::new(Vec2::new(12., 3.), vec![5, -1]),
                polyanya::Vertex::new(Vec2::new(13., 5.), vec![6, -1]),
                polyanya::Vertex::new(Vec2::new(13., 7.), vec![6, -1]),
                polyanya::Vertex::new(Vec2::new(1., 3.), vec![1, -1]),
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
        ))),
        simple_triangulation: navmeshes.add({
            vleue_navigator::NavMesh::from_edge(vec![
                vec2(0., 6.),
                vec2(2., 5.),
                vec2(2., 4.),
                vec2(1., 4.),
                vec2(1., 3.),
                vec2(2., 1.),
                vec2(4., 1.),
                vec2(4., 2.),
                vec2(7., 4.),
                vec2(7., 0.),
                vec2(12., 0.),
                vec2(12., 3.),
                vec2(11., 3.),
                vec2(11., 5.),
                vec2(13., 5.),
                vec2(13., 7.),
                vec2(10., 7.),
                vec2(11., 8.),
                vec2(7., 8.),
                vec2(7., 7.),
                vec2(5., 7.),
                vec2(5., 8.),
                vec2(0., 8.),
            ])
        }),
        arena: asset_server.load("arena-merged.polyanya.mesh"),
        arena_triangulation: navmeshes.add({
            vleue_navigator::NavMesh::from_edge_and_obstacles(
                vec![
                    vec2(1., 3.),
                    vec2(2., 3.),
                    vec2(2., 2.),
                    vec2(3., 2.),
                    vec2(3., 1.),
                    vec2(15., 1.),
                    vec2(15., 3.),
                    vec2(18., 3.),
                    vec2(18., 2.),
                    vec2(19., 2.),
                    vec2(19., 1.),
                    vec2(20., 1.),
                    vec2(20., 2.),
                    vec2(23., 2.),
                    vec2(23., 1.),
                    vec2(26., 1.),
                    vec2(26., 3.),
                    vec2(29., 3.),
                    vec2(29., 2.),
                    vec2(30., 2.),
                    vec2(30., 1.),
                    vec2(31., 1.),
                    vec2(31., 3.),
                    vec2(34., 3.),
                    vec2(34., 2.),
                    vec2(35., 2.),
                    vec2(35., 1.),
                    vec2(47., 1.),
                    vec2(47., 3.),
                    vec2(48., 3.),
                    vec2(48., 15.),
                    vec2(47., 15.),
                    vec2(47., 19.),
                    vec2(48., 19.),
                    vec2(48., 31.),
                    vec2(47., 31.),
                    vec2(47., 35.),
                    vec2(48., 35.),
                    vec2(48., 47.),
                    vec2(47., 47.),
                    vec2(47., 48.),
                    vec2(35., 48.),
                    vec2(35., 47.),
                    vec2(31., 47.),
                    vec2(31., 48.),
                    vec2(30., 48.),
                    vec2(30., 47.),
                    vec2(29., 47.),
                    vec2(29., 46.),
                    vec2(26., 46.),
                    vec2(26., 48.),
                    vec2(24., 48.),
                    vec2(24., 47.),
                    vec2(23., 47.),
                    vec2(23., 46.),
                    vec2(20., 46.),
                    vec2(20., 48.),
                    vec2(19., 48.),
                    vec2(19., 47.),
                    vec2(15., 47.),
                    vec2(15., 48.),
                    vec2(3., 48.),
                    vec2(3., 47.),
                    vec2(1., 47.),
                    vec2(1., 35.),
                    vec2(2., 35.),
                    vec2(2., 34.),
                    vec2(3., 34.),
                    vec2(3., 31.),
                    vec2(1., 31.),
                    vec2(1., 30.),
                    vec2(3., 30.),
                    vec2(3., 27.),
                    vec2(2., 27.),
                    vec2(2., 26.),
                    vec2(1., 26.),
                    vec2(1., 23.),
                    vec2(2., 23.),
                    vec2(2., 18.),
                    vec2(3., 18.),
                    vec2(3., 15.),
                    vec2(1., 15.),
                ],
                vec![
                    vec![
                        vec2(15., 15.),
                        vec2(19., 15.),
                        vec2(19., 18.),
                        vec2(18., 18.),
                        vec2(18., 19.),
                        vec2(15., 19.),
                    ],
                    vec![
                        vec2(31., 15.),
                        vec2(35., 15.),
                        vec2(35., 18.),
                        vec2(34., 18.),
                        vec2(34., 19.),
                        vec2(31., 19.),
                    ],
                    vec![
                        vec2(15., 31.),
                        vec2(19., 31.),
                        vec2(19., 34.),
                        vec2(18., 34.),
                        vec2(18., 35.),
                        vec2(15., 35.),
                    ],
                    vec![
                        vec2(31., 31.),
                        vec2(35., 31.),
                        vec2(35., 34.),
                        vec2(34., 34.),
                        vec2(34., 35.),
                        vec2(31., 35.),
                    ],
                    vec![
                        vec2(23., 10.),
                        vec2(23., 8.),
                        vec2(24., 8.),
                        vec2(24., 7.),
                        vec2(26., 7.),
                        vec2(26., 10.),
                    ],
                ],
            )
        }),
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
    asset_server: Res<AssetServer>,
    text: Query<Entity, With<Text>>,
    wireframe: Query<Entity, With<WireframeMesh>>,
    mut wait_for_mesh: Local<bool>,
) {
    if mesh.is_changed() || !window_resized.is_empty() || *wait_for_mesh {
        let handle = match mesh.mesh {
            CurrentMesh::Simple => &known_meshes.simple,
            CurrentMesh::SimpleTriangulation => &known_meshes.simple_triangulation,
            CurrentMesh::Arena => &known_meshes.arena,
            CurrentMesh::ArenaTriangulation => &known_meshes.arena_triangulation,
            CurrentMesh::Aurora => &known_meshes.aurora,
        };
        if let Some(navmesh) = navmeshes.get(handle) {
            *wait_for_mesh = false;
            if let Some(entity) = *current_mesh_entity {
                commands.entity(entity).despawn();
            }
            if let Ok(entity) = navigator.get_single() {
                commands.entity(entity).despawn();
            }
            let window = primary_window.single();
            let factor = (window.width() / mesh.size.x).min(window.height() / mesh.size.y);
            *current_mesh_entity = Some(
                commands
                    .spawn(MaterialMesh2dBundle {
                        mesh: meshes.add(navmesh.to_mesh()).into(),
                        transform: Transform::from_translation(Vec3::new(
                            -mesh.size.x / 2.0 * factor,
                            -mesh.size.y / 2.0 * factor,
                            0.0,
                        ))
                        .with_scale(Vec3::splat(factor)),
                        material: materials.add(ColorMaterial::from(Color::BLUE)),
                        ..default()
                    })
                    .id(),
            );
            if mesh.with_wireframe {
                commands.spawn((
                    MaterialMesh2dBundle {
                        mesh: meshes.add(navmesh.to_wireframe_mesh()).into(),
                        transform: Transform::from_translation(Vec3::new(
                            -mesh.size.x / 2.0 * factor,
                            -mesh.size.y / 2.0 * factor,
                            1.0,
                        ))
                        .with_scale(Vec3::splat(factor)),
                        material: materials.add(ColorMaterial::from(Color::WHITE)),
                        ..default()
                    },
                    WireframeMesh,
                ));
            }
            if let Ok(wireframe_entity) = wireframe.get_single() {
                commands.entity(wireframe_entity).despawn();
            }
            if let Ok(entity) = text.get_single() {
                commands.entity(entity).despawn();
            }
            let font = asset_server.load("fonts/FiraMono-Medium.ttf");
            commands.spawn(TextBundle {
                text: Text::from_sections([
                    TextSection::new(
                        match mesh.mesh {
                            CurrentMesh::Simple => "Simple\n",
                            CurrentMesh::SimpleTriangulation => "Triangulation from outer edge\n",
                            CurrentMesh::Arena => "Arena\n",
                            CurrentMesh::ArenaTriangulation => {
                                "Triangulation from outer edge and obstacles\n"
                            }
                            CurrentMesh::Aurora => "Aurora\n",
                        },
                        TextStyle {
                            font: font.clone_weak(),
                            font_size: 30.0,
                            color: Color::WHITE,
                        },
                    ),
                    TextSection::new(
                        match mesh.mesh {
                            CurrentMesh::Simple => {
                                "This mesh is built by providing all the needed data for it\n"
                            }
                            CurrentMesh::SimpleTriangulation => {
                                "This mesh is built from the list of points on the outer edge\n"
                            }
                            CurrentMesh::Arena => "This mesh is loaded from a file\n",
                            CurrentMesh::ArenaTriangulation => {
                                "This mesh is built from the list of points on the outer edge and on obstacles\n"
                            }
                            CurrentMesh::Aurora => "This mesh is loaded from a file\n",
                        },
                        TextStyle {
                            font: font.clone_weak(),
                            font_size: 20.0,
                            color: Color::WHITE,
                        },
                    ),
                    TextSection::new(
                        "Press spacebar or long touch to switch mesh\n",
                        TextStyle {
                            font: font.clone_weak(),
                            font_size: 15.0,
                            color: Color::WHITE,
                        },
                    ),
                    TextSection::new(
                        "Click to find a path",
                        TextStyle {
                            font: font.clone_weak(),
                            font_size: 15.0,
                            color: Color::WHITE,
                        },
                    ),
                ]),
                style: Style {
                    position_type: PositionType::Absolute,
                    margin: UiRect {
                        top: Val::Px(5.0),
                        left: Val::Px(5.0),
                        ..default()
                    },
                    ..default()
                },
                ..default()
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
            CurrentMesh::Simple => *mesh = SIMPLE_TRIANGULATION,
            CurrentMesh::SimpleTriangulation => *mesh = ARENA,
            CurrentMesh::Arena => *mesh = ARENA_TRIANGULATION,
            CurrentMesh::ArenaTriangulation => *mesh = AURORA,
            CurrentMesh::Aurora => *mesh = SIMPLE,
        }
    }
    if keyboard_input.just_pressed(KeyCode::Enter) {
        mesh.with_wireframe = !mesh.with_wireframe;
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
        let (camera, camera_transform) = camera_q.single();
        let window = primary_window.single();
        if let Some(position) = window
            .cursor_position()
            .and_then(|cursor| camera.viewport_to_world(camera_transform, cursor))
            .map(|ray| ray.origin.truncate())
        {
            let screen = Vec2::new(window.width(), window.height());
            let factor = (screen.x / mesh.size.x).min(screen.y / mesh.size.y);
            let in_mesh = position / factor + mesh.size / 2.0;
            if navmeshes
                .get(match mesh.mesh {
                    CurrentMesh::Simple => &meshes.simple,
                    CurrentMesh::SimpleTriangulation => &meshes.simple_triangulation,
                    CurrentMesh::Arena => &meshes.arena,
                    CurrentMesh::ArenaTriangulation => &meshes.arena_triangulation,
                    CurrentMesh::Aurora => &meshes.aurora,
                })
                .map(|mesh| mesh.is_in_mesh(in_mesh))
                .unwrap_or_default()
            {
                if let Ok(navigator) = query.get_single() {
                    info!("going to {}", in_mesh);
                    commands.entity(navigator).insert(Target {
                        target: in_mesh,
                        navmesh: match mesh.mesh {
                            CurrentMesh::Simple => meshes.simple.clone_weak(),
                            CurrentMesh::SimpleTriangulation => {
                                meshes.simple_triangulation.clone_weak()
                            }
                            CurrentMesh::Arena => meshes.arena.clone_weak(),
                            CurrentMesh::ArenaTriangulation => {
                                meshes.arena_triangulation.clone_weak()
                            }
                            CurrentMesh::Aurora => meshes.aurora.clone_weak(),
                        },
                    });
                } else {
                    info!("spawning at {}", in_mesh);
                    commands.spawn((
                        SpriteBundle {
                            sprite: Sprite {
                                color: Color::RED,
                                custom_size: Some(Vec2::ONE),
                                ..default()
                            },
                            transform: Transform::from_translation(
                                position.extend(1.0),
                                // (position - screen / 2.0).extend(1.0),
                            )
                            .with_scale(Vec3::splat(5.0)),
                            ..default()
                        },
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
    let window = primary_window.single();
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
    let window = primary_window.single();
    let factor = (window.width() / mesh.size.x).min(window.height() / mesh.size.y);
    for (entity, mut transform, mut path, navigator) in &mut query {
        let next = (path.path[0] - mesh.size / 2.0) * factor;
        let toward = next - transform.translation.xy();
        // TODO: compare this in mesh dimensions, not in display dimensions
        if toward.length() < time.delta_seconds() * navigator.speed {
            path.path.remove(0);
            if path.path.is_empty() {
                debug!("reached target");
                commands.entity(entity).remove::<Path>();
            } else {
                debug!("reached next step");
            }
        }
        transform.translation +=
            (toward.normalize() * time.delta_seconds() * navigator.speed).extend(0.0);
    }
}

fn display_path(
    query: Query<(&Transform, &Path)>,
    mut gizmos: Gizmos,
    mesh: Res<MeshDetails>,
    primary_window: Query<&Window, With<PrimaryWindow>>,
) {
    let window = primary_window.single();
    let factor = (window.width() / mesh.size.x).min(window.height() / mesh.size.y);

    for (transform, path) in &query {
        if path.path.is_empty() {
            continue;
        }
        gizmos.linestrip_2d(
            path.path.iter().map(|p| (*p - mesh.size / 2.0) * factor),
            Color::ORANGE,
        );

        if let Some(next) = path.path.first() {
            gizmos.line_2d(
                transform.translation.truncate(),
                (*next - mesh.size / 2.0) * factor,
                Color::YELLOW,
            );
        }
    }
}
