use std::f32::consts::PI;

use bevy::{
    color::palettes,
    math::vec2,
    prelude::*,
    sprite::MaterialMesh2dBundle,
    window::{PrimaryWindow, WindowResized},
};
use polyanya::Triangulation;
use rand::{rngs::ThreadRng, Rng};
use vleue_navigator::prelude::*;

#[path = "helpers/agent2d.rs"]
mod agent;
#[path = "helpers/ui.rs"]
mod ui;

const MESH_WIDTH: u32 = 150;
const MESH_HEIGHT: u32 = 100;

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
            NavmeshUpdaterPlugin::<PrimitiveObstacle>::default(),
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
                display_obstacle,
                display_mesh,
                spawn_obstacle_on_click.after(ui::update_settings::<10>),
                ui::update_stats::<PrimitiveObstacle>,
                remove_obstacles,
                ui::display_settings,
                ui::update_settings::<10>,
                agent::give_target_to_navigator::<10, MESH_WIDTH, MESH_HEIGHT>,
                agent::move_navigator,
                agent::display_navigator_path,
                agent::refresh_path::<10, MESH_WIDTH, MESH_HEIGHT>,
            ),
        )
        .run();
}

const FACTOR: f32 = 7.0;

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());

    // Spawn a new navmesh that will be automatically updated.
    commands.spawn(NavMeshBundle {
        settings: NavMeshSettings {
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
        update_mode: NavMeshUpdateMode::Direct,
        // This transform places the (0, 0) point of the navmesh, and is used to transform coordinates from the world to the navmesh.
        transform: Transform::from_translation(Vec3::new(
            -(MESH_WIDTH as f32) / 2.0 * FACTOR,
            -(MESH_HEIGHT as f32) / 2.0 * FACTOR,
            0.0,
        ))
        .with_scale(Vec3::splat(FACTOR)),
        ..default()
    });

    let mut rng = rand::thread_rng();
    for _ in 0..50 {
        // Obstacles are spawn in world coordinates.
        let transform = Transform::from_translation(
            Vec3::new(
                rng.gen_range((-(MESH_WIDTH as f32) / 2.0)..(MESH_WIDTH as f32 / 2.0)),
                rng.gen_range((-(MESH_HEIGHT as f32) / 2.0)..(MESH_HEIGHT as f32 / 2.0)),
                0.0,
            ) * FACTOR,
        )
        .with_rotation(Quat::from_rotation_z(rng.gen_range(0.0..(2.0 * PI))));
        new_obstacle(&mut commands, &mut rng, transform);
    }
}

fn display_obstacle(mut gizmos: Gizmos, query: Query<(&PrimitiveObstacle, &Transform)>) {
    for (prim, transform) in &query {
        match prim {
            PrimitiveObstacle::Rectangle(prim) => {
                gizmos.primitive_2d(
                    prim,
                    transform.translation.xy(),
                    transform.rotation.to_axis_angle().1,
                    palettes::tailwind::RED_600,
                );
            }
            PrimitiveObstacle::Circle(prim) => {
                gizmos.primitive_2d(
                    prim,
                    transform.translation.xy(),
                    transform.rotation.to_axis_angle().1,
                    palettes::tailwind::RED_600,
                );
            }
            PrimitiveObstacle::Ellipse(prim) => {
                gizmos.primitive_2d(
                    prim,
                    transform.translation.xy(),
                    transform.rotation.to_axis_angle().1,
                    palettes::tailwind::RED_600,
                );
            }
            PrimitiveObstacle::CircularSector(prim) => {
                gizmos.primitive_2d(
                    prim,
                    transform.translation.xy(),
                    transform.rotation.to_axis_angle().1,
                    palettes::tailwind::RED_600,
                );
            }
            PrimitiveObstacle::CircularSegment(prim) => {
                gizmos.primitive_2d(
                    prim,
                    transform.translation.xy(),
                    transform.rotation.to_axis_angle().1,
                    palettes::tailwind::RED_600,
                );
            }
            PrimitiveObstacle::Capsule(prim) => {
                gizmos.primitive_2d(
                    prim,
                    transform.translation.xy(),
                    transform.rotation.to_axis_angle().1,
                    palettes::tailwind::RED_600,
                );
            }
            PrimitiveObstacle::RegularPolygon(prim) => {
                gizmos.primitive_2d(
                    prim,
                    transform.translation.xy(),
                    transform.rotation.to_axis_angle().1,
                    palettes::tailwind::RED_600,
                );
            }
            PrimitiveObstacle::Rhombus(prim) => {
                gizmos.primitive_2d(
                    prim,
                    transform.translation.xy(),
                    transform.rotation.to_axis_angle().1,
                    palettes::tailwind::RED_600,
                );
            }
        }
    }
}

fn new_obstacle(commands: &mut Commands, rng: &mut ThreadRng, transform: Transform) {
    commands.spawn((
        match rng.gen_range(0..8) {
            0 => PrimitiveObstacle::Rectangle(Rectangle {
                half_size: vec2(rng.gen_range(1.0..5.0), rng.gen_range(1.0..5.0)) * FACTOR,
            }),
            1 => PrimitiveObstacle::Circle(Circle {
                radius: rng.gen_range(1.0..5.0) * FACTOR,
            }),
            2 => PrimitiveObstacle::Ellipse(Ellipse {
                half_size: vec2(rng.gen_range(1.0..5.0), rng.gen_range(1.0..5.0)) * FACTOR,
            }),
            3 => PrimitiveObstacle::CircularSector(CircularSector::new(
                rng.gen_range(1.5..5.0) * FACTOR,
                rng.gen_range(0.5..PI),
            )),
            4 => PrimitiveObstacle::CircularSegment(CircularSegment::new(
                rng.gen_range(1.5..5.0) * FACTOR,
                rng.gen_range(1.0..PI),
            )),
            5 => PrimitiveObstacle::Capsule(Capsule2d::new(
                rng.gen_range(1.0..3.0) * FACTOR,
                rng.gen_range(1.5..5.0) * FACTOR,
            )),
            6 => PrimitiveObstacle::RegularPolygon(RegularPolygon::new(
                rng.gen_range(1.0..5.0) * FACTOR,
                rng.gen_range(3..8),
            )),
            7 => PrimitiveObstacle::Rhombus(Rhombus::new(
                rng.gen_range(3.0..6.0) * FACTOR,
                rng.gen_range(2.0..3.0) * FACTOR,
            )),
            _ => unreachable!(),
        },
        SpatialBundle::from_transform(transform),
    ));
}

fn display_mesh(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    navmeshes: Res<Assets<NavMesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut current_mesh_entity: Local<Option<Entity>>,
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

    *current_mesh_entity = Some(
        commands
            .spawn(MaterialMesh2dBundle {
                mesh: meshes.add(navmesh.to_mesh()).into(),
                material: materials.add(ColorMaterial::from(Color::Srgba(
                    palettes::tailwind::BLUE_800,
                ))),
                ..default()
            })
            .with_children(|main_mesh| {
                main_mesh.spawn(MaterialMesh2dBundle {
                    mesh: meshes.add(navmesh.to_wireframe_mesh()).into(),
                    transform: Transform::from_translation(Vec3::new(0.0, 0.0, 0.1)),
                    material: materials.add(ColorMaterial::from(Color::Srgba(
                        palettes::tailwind::TEAL_300,
                    ))),
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
    settings: Query<Ref<NavMeshSettings>>,
) {
    // Click was on a UI button that triggered a settings change, ignore it.
    if settings.single().is_changed() {
        return;
    }
    if mouse_button_input.just_pressed(MouseButton::Left) {
        let (camera, camera_transform) = camera_q.single();
        let window = primary_window.single();
        if let Some(position) = window
            .cursor_position()
            .and_then(|cursor| camera.viewport_to_world(camera_transform, cursor))
            .map(|ray| ray.origin.truncate())
        {
            let mut rng = rand::thread_rng();
            let transform = Transform::from_translation(position.extend(0.0))
                .with_rotation(Quat::from_rotation_z(rng.gen_range(0.0..(2.0 * PI))));
            new_obstacle(&mut commands, &mut rng, transform);
            info!("spawning an obstacle at {}", position);
        }
    }
}

fn remove_obstacles(
    obstacles: Query<Entity, With<PrimitiveObstacle>>,
    mut commands: Commands,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    if keyboard_input.just_pressed(KeyCode::Space) {
        for entity in obstacles.iter() {
            commands.entity(entity).despawn();
        }
    }
}
