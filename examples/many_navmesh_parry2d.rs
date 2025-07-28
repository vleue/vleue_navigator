use bevy::{
    color::palettes,
    math::vec2,
    prelude::*,
    window::{PrimaryWindow, WindowResized},
};
use parry2d::shape::TypedShape;
use polyanya::Triangulation;
use rand::{Rng, rngs::ThreadRng};
use std::{f32::consts::PI, ops::Deref};
use vleue_navigator::prelude::*;

use crate::{
    agent::{Navigator, SpecialNavmeshId},
    ui::ShowingNavMesh,
};
#[path = "helpers/agent2d.rs"]
mod agent;
#[path = "helpers/ui.rs"]
mod ui;

const MESH_WIDTH: u32 = 150;
const MESH_HEIGHT: u32 = 100;

#[derive(Debug, Component)]
pub struct LandNavMesh;

#[derive(Debug, Component)]
pub struct AirNavMesh;

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
            // and use the `SharedShape` component as the obstacle data source.
            NavmeshUpdaterPlugin::<SharedShapeStorage>::default(),
        ))
        .add_systems(
            Startup,
            (
                ui::setup_stats::<true>,
                ui::setup_settings::<false>,
                agent::setup_agent::<10, 10, 2>,
                setup,
            )
                .chain(),
        )
        .add_systems(
            Update,
            (
                display_obstacle,
                display_mesh,
                spawn_obstacle_on_click,
                ui::update_stats::<SharedShapeStorage>,
                remove_obstacles,
                ui::display_settings,
                ui::update_settings::<10>,
                agent::give_target_to_navigator::<10, MESH_WIDTH, MESH_HEIGHT>,
                agent::move_navigator,
                agent::refresh_path::<10, MESH_WIDTH, MESH_HEIGHT>,
                agent::display_navigator_path,
            ),
        )
        .run();
}

const FACTOR: f32 = 7.0;

fn setup(
    mut commands: Commands,
    mut showing_navmesh: ResMut<ShowingNavMesh>,
    mut navigators: Query<(Entity, &mut Sprite), With<Navigator>>,
) {
    commands.spawn(Camera2d);
    // Spawn a new navmesh that will be automatically updated.
    let land_navmesh = commands
        .spawn((
            LandNavMesh,
            ManagedNavMesh::from_id(0),
            NavMeshSettings {
                // Define the outer borders of the navmesh.
                // This will be in navmesh coordinates
                fixed: Triangulation::from_outer_edges(&[
                    vec2(0.0, 0.0),
                    vec2(MESH_WIDTH as f32, 0.0),
                    vec2(MESH_WIDTH as f32, MESH_HEIGHT as f32),
                    vec2(0.0, MESH_HEIGHT as f32),
                ]),
                // Starting with a small mesh simplification factor to avoid very small geometry.
                // Small geometry can make navmesh generation fail due to rounding errors.
                // This example has round obstacles which can create small details.
                simplify: 0.05,
                ..default()
            },
            // Mark it for update as soon as obstacles are changed.
            // Other modes can be debounced or manually triggered.
            NavMeshUpdateMode::Direct,
            // This transform places the (0, 0) point of the navmesh, and is used to transform coordinates from the world to the navmesh.
            Transform::from_translation(Vec3::new(
                -(MESH_WIDTH as f32) / 2.0 * FACTOR,
                -(MESH_HEIGHT as f32) / 2.0 * FACTOR,
                0.0,
            ))
            .with_scale(Vec3::splat(FACTOR)),
        ))
        .id();

    // Set the currently displayed navmesh to the land navmesh entity.
    showing_navmesh.0 = Some(land_navmesh);

    let air_navmesh = commands
        .spawn((
            AirNavMesh,
            ManagedNavMesh::from_id(1),
            NavMeshSettings {
                fixed: Triangulation::from_outer_edges(&[
                    vec2(0.0, 0.0),
                    vec2(MESH_WIDTH as f32, 0.0),
                    vec2(MESH_WIDTH as f32, MESH_HEIGHT as f32),
                    vec2(0.0, MESH_HEIGHT as f32),
                ]),
                simplify: 0.05,
                filter_obstacles_mode: FilterObstaclesMode::Ignore,
                ..default()
            },
            NavMeshUpdateMode::Direct,
            Transform::from_translation(Vec3::new(
                -(MESH_WIDTH as f32) / 2.0 * FACTOR,
                -(MESH_HEIGHT as f32) / 2.0 * FACTOR,
                0.0,
            ))
            .with_scale(Vec3::splat(FACTOR)),
        ))
        .id();

    let navmeshs = [land_navmesh, air_navmesh];
    let colors = [palettes::css::RED, palettes::css::FUCHSIA];
    for (index, (entity, mut sprite)) in navigators.iter_mut().enumerate() {
        sprite.color = colors[index].into();
        commands
            .entity(entity)
            .insert(SpecialNavmeshId(navmeshs[index]));
        info!(
            "Navigator entity: {:?} use the navmesh id is {:?}",
            entity, navmeshs[index]
        );
    }
}

fn display_obstacle(mut gizmos: Gizmos, query: Query<(&SharedShapeStorage, &Transform)>) {
    for (shape, transform) in &query {
        match shape.shape_scaled().as_typed_shape() {
            TypedShape::Ball(ball) => {
                gizmos.circle_2d(
                    Isometry2d::new(
                        transform.translation.xy(),
                        Rot2::radians(transform.rotation.to_axis_angle().1),
                    ),
                    ball.radius,
                    Color::WHITE,
                );
            }
            TypedShape::Cuboid(cuboid) => {
                gizmos.rect_2d(
                    Isometry2d::new(
                        transform.translation.xy(),
                        Rot2::radians(transform.rotation.to_axis_angle().1),
                    ),
                    (cuboid.half_extents.xy() * 2.0).into(),
                    Color::WHITE,
                );
            }
            TypedShape::Capsule(capsule) => {
                gizmos.primitive_2d(
                    &Capsule2d::new(capsule.radius, capsule.height()),
                    Isometry2d::new(
                        transform.translation.xy(),
                        Rot2::radians(transform.rotation.to_axis_angle().1),
                    ),
                    Color::WHITE,
                );
            }
            _ => {}
        }
    }
}

fn new_obstacle(commands: &mut Commands, rng: &mut ThreadRng, transform: Transform) -> Entity {
    commands
        .spawn((
            match rng.random_range(0..6) {
                0 => SharedShapeStorage::rectangle(
                    rng.random_range(1.0..5.0) * FACTOR,
                    rng.random_range(1.0..5.0) * FACTOR,
                ),
                1 => SharedShapeStorage::circle(rng.random_range(1.0..5.0) * FACTOR),
                2 => SharedShapeStorage::ellipse(
                    rng.random_range(1.0..5.0) * FACTOR,
                    rng.random_range(1.0..5.0) * FACTOR,
                ),
                3 => SharedShapeStorage::capsule(
                    rng.random_range(1.0..3.0) * FACTOR,
                    rng.random_range(1.5..5.0) * FACTOR,
                ),
                4 => SharedShapeStorage::round_rectangle(
                    rng.random_range(1.0..3.0) * FACTOR,
                    rng.random_range(1.5..5.0) * FACTOR,
                    rng.random_range(1.0..2.0) * FACTOR,
                ),
                5 => SharedShapeStorage::regular_polygon(
                    rng.random_range(1.0..5.0) * FACTOR,
                    rng.random_range(3..8),
                ),
                _ => unreachable!(),
            },
            transform,
        ))
        .id()
}

fn display_mesh(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    navmeshes: Res<Assets<NavMesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut current_mesh_entity: Local<Option<Entity>>,
    window_resized: EventReader<WindowResized>,
    navmesh: Single<(&ManagedNavMesh, Ref<NavMeshStatus>), With<LandNavMesh>>,
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
                    MeshMaterial2d(materials.add(ColorMaterial::from(Color::Srgba(
                        palettes::tailwind::TEAL_300,
                    )))),
                    Transform::from_translation(Vec3::new(0.0, 0.0, 0.1)),
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
    mut settings: Single<&mut NavMeshSettings, With<AirNavMesh>>,
) -> Result {
    if mouse_button_input.just_pressed(MouseButton::Right) {
        let Ok((camera, camera_transform)) = camera_q.single() else {
            return Ok(());
        };
        let window = *primary_window;
        if let Some(position) = window
            .cursor_position()
            .and_then(|cursor| camera.viewport_to_world(camera_transform, cursor).ok())
            .map(|ray| ray.origin.truncate())
        {
            let mut rng = rand::rng();
            let transform = Transform::from_translation(position.extend(0.0))
                .with_rotation(Quat::from_rotation_z(rng.random_range(0.0..(2.0 * PI))));
            settings
                .filter_obstacles
                .insert(new_obstacle(&mut commands, &mut rng, transform));
        }
    }
    Ok(())
}

fn remove_obstacles(
    obstacles: Query<Entity, With<SharedShapeStorage>>,
    mut commands: Commands,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    if keyboard_input.just_pressed(KeyCode::Space) {
        for entity in obstacles.iter() {
            commands.entity(entity).despawn();
        }
    }
}
