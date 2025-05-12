use bevy::{
    color::palettes,
    math::vec2,
    prelude::*,
    window::{PrimaryWindow, WindowResized},
};
use rand::Rng;
use vleue_navigator::{NavMesh, VleueNavigatorPlugin};

const MESH_WIDTH: f32 = 15.0;
const MESH_HEIGHT: f32 = 10.0;

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
        .init_resource::<MyNavMesh>()
        .run();
}

#[derive(Resource, Default)]
struct MyNavMesh(Handle<NavMesh>);

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
}

#[derive(Default, Resource)]
struct PathToDisplay {
    steps: Vec<Vec2>,
}

fn on_mesh_change(
    mut path_to_display: ResMut<PathToDisplay>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    navmeshes: Res<Assets<NavMesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    known_meshes: Res<MyNavMesh>,
    mut current_mesh_entity: Local<Option<Entity>>,
    primary_window: Single<&Window, With<PrimaryWindow>>,
    window_resized: EventReader<WindowResized>,
    text: Query<Entity, With<Text>>,
    mut waiting_for_available: Local<bool>,
) {
    if !*waiting_for_available && !known_meshes.is_changed() && window_resized.is_empty() {
        return;
    }
    path_to_display.steps.clear();
    let Some(navmesh) = navmeshes.get(&known_meshes.0) else {
        *waiting_for_available = true;
        return;
    };
    *waiting_for_available = false;
    if let Some(entity) = *current_mesh_entity {
        commands.entity(entity).despawn();
    }
    let window = *primary_window;
    let factor = (window.width() / MESH_WIDTH).min(window.height() / MESH_HEIGHT);

    *current_mesh_entity = Some(
        commands
            .spawn((
                Mesh2d(meshes.add(navmesh.to_mesh()).into()),
                Transform::from_translation(Vec3::new(
                    -MESH_WIDTH / 2.0 * factor,
                    -MESH_HEIGHT / 2.0 * factor,
                    0.0,
                ))
                .with_scale(Vec3::splat(factor)),
                MeshMaterial2d(
                    materials.add(ColorMaterial::from(Color::Srgba(palettes::css::BLUE))),
                ),
            ))
            .with_children(|main_mesh| {
                main_mesh.spawn((
                    Mesh2d(meshes.add(navmesh.to_wireframe_mesh()).into()),
                    Transform::from_translation(Vec3::new(0.0, 0.0, 0.1)),
                    MeshMaterial2d(materials.add(ColorMaterial::from(Color::srgb(0.5, 0.5, 1.0)))),
                ));
            })
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
                TextSpan::new("Random Triangle Obstacles\n".to_string()),
                TextFont {
                    font_size: 30.0,
                    ..default()
                },
            ));
            p.spawn((
                TextSpan::new("Press spacebar to change obstacles\n".to_string()),
                TextFont {
                    font_size: 25.0,
                    ..default()
                },
            ));
            p.spawn((
                TextSpan::new("Click to find a path".to_string()),
                TextFont {
                    font_size: 25.0,
                    ..default()
                },
            ));
        });
}

fn mesh_change(
    mut meshes: ResMut<MyNavMesh>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut navmeshes: ResMut<Assets<NavMesh>>,
) {
    if keyboard_input.just_pressed(KeyCode::Space) || meshes.0 == Handle::default() {
        let mut obstacles = vec![];
        let mut rng = rand::thread_rng();
        for _i in 0..500 {
            let point = vec2(
                rng.gen_range(0.0..MESH_WIDTH),
                rng.gen_range(0.0..MESH_HEIGHT),
            );
            let around = -0.6..0.6;
            obstacles.push(vec![
                point + vec2(rng.gen_range(around.clone()), rng.gen_range(around.clone())),
                point + vec2(rng.gen_range(around.clone()), rng.gen_range(around.clone())),
                point + vec2(rng.gen_range(around.clone()), rng.gen_range(around)),
            ]);
        }

        meshes.0 = navmeshes.add(NavMesh::from_edge_and_obstacles(
            vec![vec2(0., 0.), vec2(15., 0.), vec2(15., 10.), vec2(0., 10.)],
            obstacles,
        ));
    }
}

#[derive(Event)]
struct NewPathStepEvent(Vec2);

fn on_click(
    mut path_step_event: EventWriter<NewPathStepEvent>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    primary_window: Single<&Window, With<PrimaryWindow>>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    meshes: Res<MyNavMesh>,
    navmeshes: Res<Assets<NavMesh>>,
) {
    if mouse_button_input.just_pressed(MouseButton::Left) {
        let Ok((camera, camera_transform)) = camera_q.single() else {
            return;
        };
        let window = *primary_window;
        if let Some(position) = window
            .cursor_position()
            .and_then(|cursor| camera.viewport_to_world(camera_transform, cursor).ok())
            .map(|ray| ray.origin.truncate())
        {
            let screen = Vec2::new(window.width(), window.height());
            let factor = (screen.x / MESH_WIDTH).min(screen.y / MESH_HEIGHT);

            let in_mesh = position / factor + vec2(MESH_WIDTH, MESH_HEIGHT) / 2.0;
            if navmeshes
                .get(&meshes.0)
                .map(|mesh| mesh.is_in_mesh(in_mesh))
                .unwrap_or_default()
            {
                info!("going to {}", in_mesh);
                path_step_event.write(NewPathStepEvent(in_mesh));
            } else {
                info!("clicked outside of mesh");
            }
        }
    }
}

fn compute_paths(
    mut event_new_step_path: EventReader<NewPathStepEvent>,
    mut path_to_display: ResMut<PathToDisplay>,
    meshes: Res<MyNavMesh>,
    navmeshes: Res<Assets<NavMesh>>,
) {
    for ev in event_new_step_path.read() {
        if path_to_display.steps.is_empty() {
            path_to_display.steps.push(ev.0);
            return;
        }

        let navmesh = navmeshes.get(&meshes.0).unwrap();
        if let Some(path) = navmesh.path(*path_to_display.steps.last().unwrap(), ev.0) {
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
    primary_window: Single<&Window, With<PrimaryWindow>>,
) {
    let window = *primary_window;
    let factor = (window.width() / MESH_WIDTH).min(window.height() / MESH_HEIGHT);

    let path = path_to_display
        .steps
        .iter()
        .map(|p| (*p - vec2(MESH_WIDTH, MESH_HEIGHT) / 2.0) * factor);

    if path.len() >= 1 {
        gizmos.linestrip_2d(path, palettes::css::YELLOW);
    }
}
