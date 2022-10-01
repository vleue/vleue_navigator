use bevy::{prelude::*, sprite::MaterialMesh2dBundle, window::WindowResized};
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
        .add_plugins(DefaultPlugins)
        .add_plugin(DebugLinesPlugin::default())
        .add_plugin(PathmeshPlugin)
        .add_event::<NewPathStepEvent>()
        .insert_resource(PathToDisplay::default())
        .add_startup_system(setup)
        .add_system(on_mesh_change)
        .add_system(mesh_change)
        .add_system(on_click)
        .add_system(compute_paths)
        .add_system(update_path_display)
        .run();
}

struct Meshes {
    simple: Handle<PathMesh>,
    arena: Handle<PathMesh>,
    aurora: Handle<PathMesh>,
}

enum CurrentMesh {
    Simple,
    Arena,
    Aurora,
}

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
    mut pathmeshes: ResMut<Assets<PathMesh>>,
    asset_server: Res<AssetServer>,
) {
    commands.spawn_bundle(Camera2dBundle::default());
    commands.insert_resource(Meshes {
        simple: pathmeshes.add(PathMesh::from_polyanya_mesh(polyanya::Mesh::new(
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

#[derive(Default)]
struct PathToDisplay {
    steps: Vec<Vec2>,
}

fn on_mesh_change(
    mut path_to_display: ResMut<PathToDisplay>,
    mesh: Res<MeshDetails>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    pathmeshes: Res<Assets<PathMesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    path_meshes: Res<Meshes>,
    mut current_mesh_entity: Local<Option<Entity>>,
    windows: Res<Windows>,
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
    let pathmesh = pathmeshes.get(handle).unwrap();
    if let Some(entity) = *current_mesh_entity {
        commands.entity(entity).despawn();
    }
    let window = windows.primary();
    let factor = (window.width() / mesh.size.x).min(window.height() / mesh.size.y);
    *current_mesh_entity = Some(
        commands
            .spawn_bundle(MaterialMesh2dBundle {
                mesh: meshes.add(pathmesh.to_mesh()).into(),
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
    if let Ok(entity) = text.get_single() {
        commands.entity(entity).despawn();
    }
    let font = asset_server.load("fonts/FiraMono-Medium.ttf");
    commands.spawn_bundle(TextBundle {
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

fn mesh_change(mut mesh: ResMut<MeshDetails>, keyboard_input: Res<Input<KeyCode>>) {
    if keyboard_input.just_pressed(KeyCode::Space) {
        match mesh.mesh {
            CurrentMesh::Simple => *mesh = ARENA,
            CurrentMesh::Arena => *mesh = AURORA,
            CurrentMesh::Aurora => *mesh = SIMPLE,
        }
    }
}

struct NewPathStepEvent(Vec2);

fn on_click(
    mut path_step_event: EventWriter<NewPathStepEvent>,
    mouse_button_input: Res<Input<MouseButton>>,
    windows: Res<Windows>,
    mesh: Res<MeshDetails>,
    meshes: Res<Meshes>,
    pathmeshes: Res<Assets<PathMesh>>,
) {
    if mouse_button_input.just_pressed(MouseButton::Left) {
        if let Some(position) = windows.primary().cursor_position() {
            let screen = Vec2::new(windows.primary().width(), windows.primary().height());
            let factor = (screen.x / mesh.size.x).min(screen.y / mesh.size.y);

            let in_mesh = (position - screen / 2.0) / factor + mesh.size / 2.0;
            if pathmeshes
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
    pathmeshes: Res<Assets<PathMesh>>,
) {
    for ev in event_new_step_path.iter() {
        if path_to_display.steps.is_empty() {
            path_to_display.steps.push(ev.0);
            return;
        }

        let path_mesh = pathmeshes
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
    mut lines: ResMut<DebugLines>,
    mesh: Res<MeshDetails>,
    windows: Res<Windows>,
) {
    let window = windows.primary();
    let factor = (window.width() / mesh.size.x).min(window.height() / mesh.size.y);

    (1..path_to_display.steps.len()).for_each(|i| {
        lines.line(
            ((path_to_display.steps[i - 1] - mesh.size / 2.0) * factor).extend(0f32),
            ((path_to_display.steps[i] - mesh.size / 2.0) * factor).extend(0f32),
            0f32,
        );
    });
}
