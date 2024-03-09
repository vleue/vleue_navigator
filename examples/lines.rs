use bevy::{
    prelude::*,
    sprite::MaterialMesh2dBundle,
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
        .add_event::<NewPathStepEvent>()
        .insert_resource(PathToDisplay::default())
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                on_mesh_change,
                mesh_change,
                on_click,
                compute_paths,
                update_path_display,
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
        arena: asset_server.load("arena-merged.polyanya.mesh"),
        aurora: asset_server.load("aurora-merged.polyanya.mesh"),
    });
    commands.insert_resource(SIMPLE);
}

#[derive(Default, Resource)]
struct PathToDisplay {
    steps: Vec<Vec2>,
}

fn on_mesh_change(
    mut path_to_display: ResMut<PathToDisplay>,
    mesh: Res<MeshDetails>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    navmeshes: Res<Assets<NavMesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    path_meshes: Res<Meshes>,
    mut current_mesh_entity: Local<Option<Entity>>,
    primary_window: Query<&Window, With<PrimaryWindow>>,
    window_resized: EventReader<WindowResized>,
    asset_server: Res<AssetServer>,
    text: Query<Entity, With<Text>>,
) {
    if !mesh.is_changed() && window_resized.is_empty() {
        return;
    }
    path_to_display.steps.clear();
    let handle = match mesh.mesh {
        CurrentMesh::Simple => &path_meshes.simple,
        CurrentMesh::Arena => &path_meshes.arena,
        CurrentMesh::Aurora => &path_meshes.aurora,
    };
    let navmesh = navmeshes.get(handle).unwrap();
    if let Some(entity) = *current_mesh_entity {
        commands.entity(entity).despawn_recursive();
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
            .with_children(|main_mesh| {
                main_mesh.spawn(MaterialMesh2dBundle {
                    mesh: meshes.add(navmesh.to_wireframe_mesh()).into(),
                    transform: Transform::from_translation(Vec3::new(0.0, 0.0, 0.1)),
                    material: materials.add(ColorMaterial::from(Color::rgb(0.5, 0.5, 1.0))),
                    ..default()
                });
            })
            .id(),
    );
    if let Ok(entity) = text.get_single() {
        commands.entity(entity).despawn_recursive();
    }
    let font = asset_server.load("fonts/FiraMono-Medium.ttf");
    commands.spawn(TextBundle {
        text: Text::from_sections([
            TextSection::new(
                match mesh.mesh {
                    CurrentMesh::Simple => "Simple\n",
                    CurrentMesh::Arena => "Arena\n",
                    CurrentMesh::Aurora => "Aurora\n",
                },
                TextStyle {
                    font: font.clone_weak(),
                    font_size: 30.0,
                    color: Color::WHITE,
                },
            ),
            TextSection::new(
                "Press spacebar to switch mesh\n",
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
}

fn mesh_change(mut mesh: ResMut<MeshDetails>, keyboard_input: Res<ButtonInput<KeyCode>>) {
    if keyboard_input.just_pressed(KeyCode::Space) {
        match mesh.mesh {
            CurrentMesh::Simple => *mesh = ARENA,
            CurrentMesh::Arena => *mesh = AURORA,
            CurrentMesh::Aurora => *mesh = SIMPLE,
        }
    }
}

#[derive(Event)]
struct NewPathStepEvent(Vec2);

fn on_click(
    mut path_step_event: EventWriter<NewPathStepEvent>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    primary_window: Query<&Window, With<PrimaryWindow>>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    mesh: Res<MeshDetails>,
    meshes: Res<Meshes>,
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
                    CurrentMesh::Arena => &meshes.arena,
                    CurrentMesh::Aurora => &meshes.aurora,
                })
                .map(|mesh| mesh.is_in_mesh(in_mesh))
                .unwrap_or_default()
            {
                info!("going to {}", in_mesh);
                path_step_event.send(NewPathStepEvent(in_mesh));
            } else {
                info!("clicked outside of mesh");
            }
        }
    }
}

fn compute_paths(
    mut event_new_step_path: EventReader<NewPathStepEvent>,
    mut path_to_display: ResMut<PathToDisplay>,
    mesh: Res<MeshDetails>,
    meshes: Res<Meshes>,
    navmeshes: Res<Assets<NavMesh>>,
) {
    for ev in event_new_step_path.read() {
        if path_to_display.steps.is_empty() {
            path_to_display.steps.push(ev.0);
            return;
        }

        let path_mesh = navmeshes
            .get(match mesh.mesh {
                CurrentMesh::Simple => &meshes.simple,
                CurrentMesh::Arena => &meshes.arena,
                CurrentMesh::Aurora => &meshes.aurora,
            })
            .unwrap();
        if let Some(path) = path_mesh.path(*path_to_display.steps.last().unwrap(), ev.0) {
            for p in path.path {
                path_to_display.steps.push(p);
            }
        } else {
            info!("no path found");
        }
    }
}

fn update_path_display(
    path_to_display: Res<PathToDisplay>,
    mut gizmos: Gizmos,
    mesh: Res<MeshDetails>,
    primary_window: Query<&Window, With<PrimaryWindow>>,
) {
    let window = primary_window.single();
    let factor = (window.width() / mesh.size.x).min(window.height() / mesh.size.y);

    let path = path_to_display
        .steps
        .iter()
        .map(|p| (*p - mesh.size / 2.0) * factor);

    if path.len() >= 1 {
        gizmos.linestrip_2d(path, Color::YELLOW);
    }
}
