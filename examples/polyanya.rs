use bevy::{math::Vec3Swizzles, prelude::*, sprite::MaterialMesh2dBundle};
use bevy_pathmesh::{PathMesh, PathmeshPlugin};

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .add_plugins(DefaultPlugins)
        .add_plugin(PathmeshPlugin)
        .add_startup_system(setup)
        .add_system(on_mesh_change)
        .add_system(mesh_change)
        .add_system(on_click)
        .add_system(compute_paths)
        .add_system(move_navigator)
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

fn on_mesh_change(
    mesh: Res<MeshDetails>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    pathmeshes: Res<Assets<PathMesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    path_meshes: Res<Meshes>,
    mut current_mesh_entity: Local<Option<Entity>>,
    windows: Res<Windows>,
    navigator: Query<Entity, With<Navigator>>,
) {
    if mesh.is_changed() {
        let handle = match mesh.mesh {
            CurrentMesh::Simple => &path_meshes.simple,
            CurrentMesh::Arena => &path_meshes.arena,
            CurrentMesh::Aurora => &path_meshes.aurora,
        };
        let pathmesh = pathmeshes.get(handle).unwrap();
        if let Some(entity) = *current_mesh_entity {
            commands.entity(entity).despawn();
        }
        if let Ok(entity) = navigator.get_single() {
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
    }
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

#[derive(Component)]
struct Navigator {
    speed: f32,
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

fn on_click(
    mouse_button_input: Res<Input<MouseButton>>,
    windows: Res<Windows>,
    mesh: Res<MeshDetails>,
    meshes: Res<Meshes>,
    mut commands: Commands,
    query: Query<Entity, With<Navigator>>,
) {
    if mouse_button_input.just_pressed(MouseButton::Left) {
        if let Some(position) = windows.primary().cursor_position() {
            let screen = Vec2::new(windows.primary().width(), windows.primary().height());
            let factor = (screen.x / mesh.size.x).min(screen.y / mesh.size.y);

            let in_mesh = (position - screen / 2.0) / factor + mesh.size / 2.0;
            if (0.0..mesh.size.x).contains(&in_mesh.x) && (0.0..mesh.size.y).contains(&in_mesh.y) {
                if let Ok(navigator) = query.get_single() {
                    info!("going to {}", in_mesh);
                    commands.entity(navigator).insert(Target {
                        target: in_mesh,
                        pathmesh: match mesh.mesh {
                            CurrentMesh::Simple => meshes.simple.clone_weak(),
                            CurrentMesh::Arena => meshes.arena.clone_weak(),
                            CurrentMesh::Aurora => meshes.aurora.clone_weak(),
                        },
                    });
                } else {
                    info!("spawning at {}", in_mesh);
                    commands
                        .spawn_bundle(SpriteBundle {
                            sprite: Sprite {
                                color: Color::RED,
                                custom_size: Some(Vec2::ONE),
                                ..default()
                            },
                            transform: Transform::from_translation(
                                (position - screen / 2.0).extend(1.0),
                            )
                            .with_scale(Vec3::splat(5.0)),
                            ..default()
                        })
                        .insert(Navigator { speed: 100.0 });
                }
            } else {
                info!("clicked outside of mesh");
            }
        }
    }
}

fn compute_paths(
    mut commands: Commands,
    with_target: Query<(Entity, &Target, &Transform), Changed<Target>>,
    meshes: Res<Assets<PathMesh>>,
    mesh: Res<MeshDetails>,
    windows: Res<Windows>,
) {
    for (entity, target, transform) in &with_target {
        let window = windows.primary();
        let factor = (window.width() / mesh.size.x).min(window.height() / mesh.size.y);

        let in_mesh = transform.translation.truncate() / factor + mesh.size / 2.0;
        let mesh = meshes.get(&target.pathmesh).unwrap();

        if let Some(path) = mesh.path(in_mesh, target.target) {
            commands.entity(entity).insert(Path { path: path.path });
        } else {
            info!("no path found");
        }
    }
}

fn move_navigator(
    mut query: Query<(Entity, &mut Transform, &mut Path, &Navigator)>,
    mesh: Res<MeshDetails>,
    windows: Res<Windows>,
    time: Res<Time>,
    mut commands: Commands,
) {
    let window = windows.primary();
    let factor = (window.width() / mesh.size.x).min(window.height() / mesh.size.y);
    for (entity, mut transform, mut path, navigator) in &mut query {
        let next = (path.path[0] - mesh.size / 2.0) * factor;
        let toward = next - transform.translation.xy();
        // TODO: compare this in mesh dimensions, not in display dimensions
        if toward.length() < 1.0 {
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
