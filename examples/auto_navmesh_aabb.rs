use bevy::{
    camera::primitives::Aabb,
    color::palettes,
    math::vec2,
    prelude::*,
    sprite_render::ColorMaterial,
    window::{PrimaryWindow, WindowResized},
};
use polyanya::Triangulation;
use rand::Rng;
use std::f32::consts::PI;
use std::ops::Deref;
use vleue_navigator::prelude::*;

#[path = "helpers/agent2d.rs"]
mod agent;
#[path = "helpers/ui.rs"]
mod ui;

const MESH_WIDTH: u32 = 15;
const MESH_HEIGHT: u32 = 10;

#[derive(Component, Debug)]
struct Obstacle;

const FACTOR: f32 = 70.0;

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
            // Auto update the navmesh.
            // Obstacles will be entities with the `Obstacle` marker component,
            // and use the `Aabb` component as the obstacle data source.
            NavmeshUpdaterPlugin::<Aabb, Obstacle>::default(),
        ))
        .add_systems(
            Startup,
            (
                setup,
                ui::setup_stats::<true>,
                ui::setup_settings::<false>,
                agent::setup_agent::<10>,
            ),
        )
        .add_systems(
            Update,
            (
                display_mesh,
                spawn_obstacle_on_click.after(ui::update_settings::<50>),
                ui::update_stats::<Obstacle>,
                remove_obstacles,
                ui::display_settings,
                ui::update_settings::<50>,
                agent::give_target_to_navigator::<10, MESH_WIDTH, MESH_HEIGHT>,
                agent::move_navigator,
                agent::display_navigator_path,
                agent::refresh_path::<10, MESH_WIDTH, MESH_HEIGHT>,
            ),
        )
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    // Spawn a new navmesh that will be automatically updated.
    commands.spawn((
        NavMeshSettings {
            // Define the outer borders of the navmesh.
            fixed: Triangulation::from_outer_edges(&[
                vec2(0.0, 0.0),
                vec2(MESH_WIDTH as f32, 0.0),
                vec2(MESH_WIDTH as f32, MESH_HEIGHT as f32),
                vec2(0.0, MESH_HEIGHT as f32),
            ]),
            ..default()
        },
        // Mark it for update as soon as obstacles are changed.
        // Other modes can be debounced or manually triggered.
        NavMeshUpdateMode::Direct,
        Transform::from_translation(Vec3::new(
            -(MESH_WIDTH as f32) / 2.0 * FACTOR,
            -(MESH_HEIGHT as f32) / 2.0 * FACTOR,
            0.0,
        ))
        .with_scale(Vec3::splat(FACTOR)),
    ));

    // Spawn a few obstacles to start with.
    // They need
    // - the `Obstacle` marker component
    // - the `Aabb` component to define the obstacle's shape
    // - the `Transform` component to define the obstacle's position
    // - the `GlobalTransform` so that it's correctly handled by Bevy
    let mut rng = rand::thread_rng();
    for _ in 0..100 {
        commands.spawn((
            Obstacle,
            Aabb::from_min_max(
                Vec3::ZERO,
                Vec3::new(rng.gen_range(10.0..50.0), rng.gen_range(10.0..50.0), 0.0),
            ),
            Transform::from_translation(
                Vec3::new(
                    rng.gen_range((-(MESH_WIDTH as f32) / 2.0)..(MESH_WIDTH as f32 / 2.0)),
                    rng.gen_range((-(MESH_HEIGHT as f32) / 2.0)..(MESH_HEIGHT as f32 / 2.0)),
                    0.0,
                ) * FACTOR,
            )
            .with_rotation(Quat::from_rotation_z(rng.gen_range(0.0..PI))),
        ));
    }
}

fn display_mesh(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    navmeshes: Res<Assets<NavMesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut current_mesh_entity: Local<Option<Entity>>,
    window_resized: MessageReader<WindowResized>,
    navmesh: Single<(&ManagedNavMesh, Ref<NavMeshStatus>)>,
) {
    let (navmesh_handle, status) = navmesh.deref();
    if (!status.is_changed() || **status != NavMeshStatus::Built) && window_resized.is_empty() {
        return;
    }

    let Some(navmesh) = navmeshes.get(*navmesh_handle) else {
        return;
    };
    if let Some(entity) = *current_mesh_entity {
        commands.entity(entity).despawn();
    }

    *current_mesh_entity = Some(
        commands
            .spawn((
                Mesh2d(meshes.add(navmesh.to_mesh())),
                MeshMaterial2d(materials.add(ColorMaterial::from(Color::Srgba(
                    palettes::tailwind::BLUE_800,
                )))),
            ))
            .with_children(|main_mesh| {
                main_mesh.spawn((
                    Mesh2d(meshes.add(navmesh.to_wireframe_mesh())),
                    Transform::from_translation(Vec3::new(0.0, 0.0, 0.1)),
                    MeshMaterial2d(materials.add(ColorMaterial::from(Color::Srgba(
                        palettes::tailwind::TEAL_300,
                    )))),
                ));
            })
            .id(),
    );
}

fn spawn_obstacle_on_click(
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    primary_window: Single<&Window, With<PrimaryWindow>>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    mut commands: Commands,
    settings: Single<Ref<NavMeshSettings>>,
) {
    // Click was on a UI button that triggered a settings change, ignore it.
    if settings.is_changed() {
        return;
    }
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
            let mut rng = rand::thread_rng();
            commands.spawn((
                Obstacle,
                Aabb::from_min_max(
                    Vec3::ZERO,
                    Vec3::new(rng.gen_range(10.0..50.), rng.gen_range(10.0..50.0), 0.0),
                ),
                Transform::from_translation(position.extend(0.0))
                    .with_rotation(Quat::from_rotation_z(rng.gen_range(0.0..PI))),
                GlobalTransform::default(),
            ));
            info!("spawning an obstacle at {}", position);
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
