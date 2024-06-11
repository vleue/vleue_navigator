use std::f32::consts::PI;

use bevy::{
    color::palettes,
    math::vec2,
    prelude::*,
    render::primitives::Aabb,
    sprite::MaterialMesh2dBundle,
    window::{PrimaryWindow, WindowResized},
};
use polyanya::Triangulation;
use rand::Rng;
use vleue_navigator::{
    updater::{
        NavMeshBundle, NavMeshSettings, NavMeshStatus, NavMeshUpdateMode, NavmeshUpdaterPlugin,
    },
    NavMesh, VleueNavigatorPlugin,
};

const MESH_WIDTH: f32 = 15.0;
const MESH_HEIGHT: f32 = 10.0;

#[derive(Component, Debug)]
struct Obstacle;

fn main() {
    App::new()
        .insert_resource(ClearColor(palettes::css::BLACK.into()))
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Navmesh with Polyanya".to_string(),
                    ..default()
                }),
                ..default()
            }),
            VleueNavigatorPlugin,
            // Auto update the navmesh.
            // Obstacles will be entities with the `Obstacle` marker component,
            // and use the `Aabb` component as the obstacle data source.
            NavmeshUpdaterPlugin::<Obstacle, Aabb>::default(),
        ))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                display_mesh,
                spawn_obstacle_on_click,
                update_stats,
                remove_obstacles,
            ),
        )
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());

    // Spawn a new navmesh that will be automatically updated.
    commands.spawn(NavMeshBundle {
        settings: NavMeshSettings {
            // Define the outer borders of the navmesh.
            fixed: Triangulation::from_outer_edges(&vec![
                vec2(0.0, 0.0),
                vec2(MESH_WIDTH, 0.0),
                vec2(MESH_WIDTH, MESH_HEIGHT),
                vec2(0.0, MESH_HEIGHT),
            ]),
            ..default()
        },
        // Mark it for update as soon as obstacles are changed.
        // Other modes can be debounced or manually triggered.
        update_mode: NavMeshUpdateMode::Direct,
        ..default()
    });

    // Spawn a few obstacles to start with.
    // They need
    // - the `Obstacle` marker component
    // - the `Aabb` component to define the obstacle's shape
    // - the `Transform` component to define the obstacle's position
    // - the `GlobalTransform` so that it's correctly handled by Bevy
    let mut rng = rand::thread_rng();
    for _ in 0..10 {
        commands.spawn((
            Obstacle,
            Aabb::from_min_max(
                Vec3::ZERO,
                Vec3::new(rng.gen_range(0.1..0.5), rng.gen_range(0.1..0.5), 0.0),
            ),
            Transform::from_translation(Vec3::new(
                rng.gen_range(0.5..MESH_WIDTH - 0.5),
                rng.gen_range(0.5..MESH_HEIGHT - 0.5),
                0.0,
            ))
            .with_rotation(Quat::from_rotation_z(rng.gen_range(0.0..PI))),
            GlobalTransform::default(),
        ));
    }

    commands.spawn(TextBundle {
        text: Text::from_sections([
            TextSection::new(
                "Status: ",
                TextStyle {
                    font_size: 30.0,
                    ..default()
                },
            ),
            TextSection::new(
                "{}",
                TextStyle {
                    font_size: 30.0,
                    ..default()
                },
            ),
            TextSection::new(
                "\nObstacles: ",
                TextStyle {
                    font_size: 30.0,
                    ..default()
                },
            ),
            TextSection::new(
                "{}",
                TextStyle {
                    font_size: 30.0,
                    ..default()
                },
            ),
            TextSection::new(
                "\nPolygons: ",
                TextStyle {
                    font_size: 30.0,
                    ..default()
                },
            ),
            TextSection::new(
                "{}",
                TextStyle {
                    font_size: 30.0,
                    ..default()
                },
            ),
            TextSection::new(
                "\n\nClick to add an obstacle",
                TextStyle {
                    font_size: 25.0,
                    ..default()
                },
            ),
            TextSection::new(
                "\nobstacles must not overlap the border, it would result in a build failure",
                TextStyle {
                    font_size: 15.0,
                    ..default()
                },
            ),
            TextSection::new(
                "\nPress spacebar to reset",
                TextStyle {
                    font_size: 25.0,
                    ..default()
                },
            ),
        ]),
        ..default()
    });
}

fn display_mesh(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    navmeshes: Res<Assets<NavMesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut current_mesh_entity: Local<Option<Entity>>,
    primary_window: Query<&Window, With<PrimaryWindow>>,
    window_resized: EventReader<WindowResized>,
    navmesh: Query<(&Handle<NavMesh>, Ref<NavMeshStatus>)>,
) {
    let (navmesh_handle, status) = navmesh.single();
    if !status.is_changed() && window_resized.is_empty() {
        return;
    }

    let Some(navmesh) = navmeshes.get(navmesh_handle) else {
        return;
    };
    if let Some(entity) = *current_mesh_entity {
        commands.entity(entity).despawn_recursive();
    }
    let window = primary_window.single();
    let factor = (window.width() / MESH_WIDTH).min(window.height() / MESH_HEIGHT);

    *current_mesh_entity = Some(
        commands
            .spawn(MaterialMesh2dBundle {
                mesh: meshes.add(navmesh.to_mesh()).into(),
                transform: Transform::from_translation(Vec3::new(
                    -MESH_WIDTH / 2.0 * factor,
                    -MESH_HEIGHT / 2.0 * factor,
                    0.0,
                ))
                .with_scale(Vec3::splat(factor)),
                material: materials.add(ColorMaterial::from(Color::Srgba(palettes::css::BLUE))),
                ..default()
            })
            .with_children(|main_mesh| {
                main_mesh.spawn(MaterialMesh2dBundle {
                    mesh: meshes.add(navmesh.to_wireframe_mesh()).into(),
                    transform: Transform::from_translation(Vec3::new(0.0, 0.0, 0.1)),
                    material: materials.add(ColorMaterial::from(Color::srgb(0.5, 0.5, 1.0))),
                    ..default()
                });
            })
            .id(),
    );
}

fn spawn_obstacle_on_click(
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    primary_window: Query<&Window, With<PrimaryWindow>>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    mut commands: Commands,
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
            let factor = (screen.x / 15.0).min(screen.y / 10.0);

            let in_mesh = position / factor + vec2(15.0, 10.0) / 2.0;
            let mut rng = rand::thread_rng();
            commands.spawn((
                Obstacle,
                Aabb::from_min_max(
                    Vec3::ZERO,
                    Vec3::new(rng.gen_range(0.1..0.5), rng.gen_range(0.1..0.5), 0.0),
                ),
                Transform::from_translation(in_mesh.extend(0.0))
                    .with_rotation(Quat::from_rotation_z(rng.gen_range(0.0..PI))),
                GlobalTransform::default(),
            ));
            info!("spawning an obstacle at {}", in_mesh);
        }
    }
}

fn remove_obstacles(
    obstacles: Query<Entity, With<Obstacle>>,
    mut commands: Commands,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    if keyboard_input.just_pressed(KeyCode::Space) {
        for entity in obstacles.iter() {
            commands.entity(entity).despawn();
        }
    }
}

fn update_stats(
    mut text: Query<&mut Text>,
    obstacles: Query<&Obstacle>,
    navmesh: Query<(Ref<NavMeshStatus>, &Handle<NavMesh>)>,
    navmeshes: Res<Assets<NavMesh>>,
) {
    let (status, handle) = navmesh.single();

    if !status.is_changed() && !status.is_added() {
        return;
    }

    let mut text = text.single_mut();
    text.sections[1].value = format!("{:?}", *status);
    text.sections[1].style.color = match *status {
        NavMeshStatus::Building => palettes::tailwind::AMBER_500.into(),
        NavMeshStatus::Built => palettes::tailwind::GREEN_400.into(),
        NavMeshStatus::Failed => palettes::tailwind::RED_600.into(),
    };
    text.sections[3].value = format!("{}", obstacles.iter().len());
    text.sections[5].value = format!(
        "{}",
        navmeshes
            .get(handle)
            .map(|nm| nm.get().polygons.len())
            .unwrap_or_default()
    );
}
