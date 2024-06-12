use std::f32::consts::{FRAC_PI_2, PI};

use bevy::{color::palettes, math::vec2, pbr::NotShadowCaster, prelude::*, window::PrimaryWindow};
use polyanya::Triangulation;
use rand::Rng;
use vleue_navigator::prelude::*;

#[path = "helpers/agent3d.rs"]
mod agent2d;
#[path = "helpers/ui.rs"]
mod ui;

const MESH_WIDTH: u32 = 150;
const MESH_HEIGHT: u32 = 100;

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
            // NavmeshUpdaterPlugin::<Obstacle, Aabb>::default(),
            NavmeshUpdaterPlugin::<PrimitiveObstacle, PrimitiveObstacle>::default(),
        ))
        .add_systems(
            Startup,
            (
                setup,
                ui::setup_stats,
                ui::setup_settings,
                agent2d::setup_agent::<100>,
            ),
        )
        .add_systems(
            Update,
            (
                display_mesh,
                spawn_obstacle_on_click.after(ui::update_settings::<10>),
                ui::update_stats::<PrimitiveObstacle>,
                remove_obstacles,
                ui::display_settings,
                ui::update_settings::<10>,
                agent2d::give_target_to_navigator::<10, MESH_WIDTH, MESH_HEIGHT>,
                agent2d::move_navigator,
                agent2d::display_navigator_path,
                agent2d::refresh_path::<100, MESH_WIDTH, MESH_HEIGHT>,
            ),
        )
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(
            MESH_WIDTH as f32 / 2.0,
            MESH_WIDTH.min(MESH_HEIGHT) as f32,
            -1.0,
        )
        .looking_at(
            Vec3::new(
                MESH_WIDTH as f32 / 2.0,
                0.0,
                MESH_HEIGHT as f32 / 2.0 - MESH_HEIGHT as f32 / 12.0,
            ),
            Vec3::Y,
        ),
        ..Default::default()
    });
    // light
    for (x, y) in [(0.25, 0.25), (0.75, 0.25), (0.25, 0.75), (0.75, 0.75)] {
        commands.spawn(PointLightBundle {
            point_light: PointLight {
                shadows_enabled: true,
                intensity: MESH_WIDTH.min(MESH_HEIGHT) as f32 * 3_000_00.0,
                range: MESH_WIDTH.min(MESH_HEIGHT) as f32 * 10.0,
                ..default()
            },
            transform: Transform::from_xyz(
                MESH_WIDTH as f32 * x,
                MESH_WIDTH.min(MESH_HEIGHT) as f32 / 3.0,
                MESH_HEIGHT as f32 * y,
            ),
            ..default()
        });
    }

    // Spawn a new navmesh that will be automatically updated.
    commands.spawn(NavMeshBundle {
        settings: NavMeshSettings {
            // Define the outer borders of the navmesh.
            fixed: Triangulation::from_outer_edges(&vec![
                vec2(0.0, 0.0),
                vec2(MESH_WIDTH as f32, 0.0),
                vec2(MESH_WIDTH as f32, MESH_HEIGHT as f32),
                vec2(0.0, MESH_HEIGHT as f32),
            ]),
            simplify: 0.001,
            merge_steps: 0,

            ..default()
        },
        transform: Transform::from_rotation(Quat::from_rotation_x(-FRAC_PI_2)),
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
    let mat = materials.add(StandardMaterial::from(Color::Srgba(
        palettes::tailwind::RED_700,
    )));
    for _ in 0..40 {
        let transform = Transform::from_translation(Vec3::new(
            rng.gen_range(0.0..(MESH_WIDTH as f32)),
            0.0,
            rng.gen_range(0.0..(MESH_HEIGHT as f32)),
        ))
        .with_rotation(Quat::from_rotation_y(rng.gen_range(0.0..PI)));
        new_obstacle(&mut commands, &mut rng, transform, &mut *meshes, &mat);
    }
}

fn new_obstacle(
    commands: &mut Commands,
    rng: &mut impl Rng,
    transform: Transform,
    meshes: &mut Assets<Mesh>,
    mat: &Handle<StandardMaterial>,
) {
    let radius = 1.0;
    match rng.gen_range(0..8) {
        0 => {
            let primitive = Rectangle {
                half_size: vec2(rng.gen_range(1.0..5.0), rng.gen_range(1.0..5.0)),
            };
            let larger_primitive = Rectangle {
                half_size: primitive.half_size + vec2(radius, radius),
            };
            commands
                .spawn((
                    transform,
                    GlobalTransform::default(),
                    PrimitiveObstacle::Rectangle(larger_primitive),
                ))
                .with_children(|parent| {
                    parent.spawn(PbrBundle {
                        mesh: meshes.add(Extrusion::new(primitive, rng.gen_range(5.0..15.0))),
                        material: mat.clone(),
                        transform: Transform::from_rotation(Quat::from_rotation_x(FRAC_PI_2)),
                        ..default()
                    });
                });
        }
        1 => {
            let primitive = Circle {
                radius: rng.gen_range(1.0..5.0),
            };
            let larger_primitive = Circle {
                radius: primitive.radius + radius,
            };
            commands
                .spawn((
                    transform,
                    GlobalTransform::default(),
                    PrimitiveObstacle::Circle(larger_primitive),
                ))
                .with_children(|parent| {
                    parent.spawn(PbrBundle {
                        mesh: meshes.add(Extrusion::new(primitive, rng.gen_range(5.0..15.0))),
                        material: mat.clone(),
                        transform: Transform::from_rotation(Quat::from_rotation_x(FRAC_PI_2)),
                        ..default()
                    });
                });
        }
        2 => {
            let primitive = Ellipse {
                half_size: vec2(rng.gen_range(1.0..5.0), rng.gen_range(1.0..5.0)),
            };
            let larger_primitive = Ellipse {
                half_size: primitive.half_size + vec2(radius, radius),
            };
            commands
                .spawn((
                    transform,
                    GlobalTransform::default(),
                    PrimitiveObstacle::Ellipse(larger_primitive),
                ))
                .with_children(|parent| {
                    parent.spawn(PbrBundle {
                        mesh: meshes.add(Extrusion::new(primitive, rng.gen_range(5.0..15.0))),
                        material: mat.clone(),
                        transform: Transform::from_rotation(Quat::from_rotation_x(FRAC_PI_2)),
                        ..default()
                    });
                });
        }
        3 => {
            let primitive =
                CircularSector::new(rng.gen_range(1.5..5.0), rng.gen_range(0.5..FRAC_PI_2));
            let larger_primitive = CircularSector::from(Arc2d {
                radius: primitive.radius() + radius,
                half_angle: primitive.half_angle(),
            });
            commands
                .spawn((
                    transform,
                    GlobalTransform::default(),
                    PrimitiveObstacle::CircularSector(larger_primitive),
                ))
                .with_children(|parent| {
                    parent.spawn(PbrBundle {
                        mesh: meshes.add(Extrusion::new(primitive, rng.gen_range(5.0..15.0))),
                        material: mat.clone(),
                        transform: Transform::from_rotation(Quat::from_rotation_x(FRAC_PI_2)),
                        ..default()
                    });
                });
        }
        4 => {
            let primitive = CircularSegment::new(rng.gen_range(1.5..5.0), rng.gen_range(1.0..PI));
            let larger_primitive = CircularSegment::from(Arc2d {
                radius: primitive.radius() + radius,
                half_angle: primitive.half_angle(),
            });
            commands
                .spawn((
                    transform,
                    GlobalTransform::default(),
                    PrimitiveObstacle::CircularSegment(larger_primitive),
                ))
                .with_children(|parent| {
                    parent.spawn(PbrBundle {
                        mesh: meshes.add(Extrusion::new(primitive, rng.gen_range(5.0..15.0))),
                        material: mat.clone(),
                        transform: Transform::from_rotation(Quat::from_rotation_x(FRAC_PI_2)),
                        ..default()
                    });
                });
        }
        5 => {
            let primitive = Capsule2d::new(rng.gen_range(1.0..3.0), rng.gen_range(1.5..5.0));
            let larger_primitive =
                Capsule2d::new(primitive.radius + radius, primitive.half_length * 2.0);
            commands
                .spawn((
                    transform,
                    GlobalTransform::default(),
                    PrimitiveObstacle::Capsule(larger_primitive),
                ))
                .with_children(|parent| {
                    parent.spawn(PbrBundle {
                        mesh: meshes.add(Extrusion::new(primitive, rng.gen_range(5.0..15.0))),
                        material: mat.clone(),
                        transform: Transform::from_rotation(Quat::from_rotation_x(FRAC_PI_2)),
                        ..default()
                    });
                });
        }
        6 => {
            let primitive = RegularPolygon::new(rng.gen_range(1.0..5.0), rng.gen_range(3..8));
            let larger_primitive =
                RegularPolygon::new(primitive.circumradius() + radius, primitive.sides);
            commands
                .spawn((
                    transform,
                    GlobalTransform::default(),
                    PrimitiveObstacle::RegularPolygon(larger_primitive),
                ))
                .with_children(|parent| {
                    parent.spawn(PbrBundle {
                        mesh: meshes.add(Extrusion::new(primitive, rng.gen_range(5.0..15.0))),
                        material: mat.clone(),
                        transform: Transform::from_rotation(Quat::from_rotation_x(FRAC_PI_2)),
                        ..default()
                    });
                });
        }
        7 => {
            let primitive = Rhombus::new(rng.gen_range(3.0..6.0), rng.gen_range(2.0..3.0));
            let larger_primitive = Rhombus::new(
                (primitive.half_diagonals.x + radius) * 2.0,
                (primitive.half_diagonals.y + radius) * 2.0,
            );
            commands
                .spawn((
                    transform,
                    GlobalTransform::default(),
                    PrimitiveObstacle::Rhombus(larger_primitive),
                ))
                .with_children(|parent| {
                    parent.spawn(PbrBundle {
                        mesh: meshes.add(Extrusion::new(primitive, rng.gen_range(5.0..15.0))),
                        material: mat.clone(),
                        transform: Transform::from_rotation(Quat::from_rotation_x(FRAC_PI_2)),
                        ..default()
                    });
                });
        }
        _ => unreachable!(),
    }
}

fn display_mesh(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    navmeshes: Res<Assets<NavMesh>>,
    mut current_mesh_entity: Local<Option<Entity>>,
    navmesh: Query<(&Handle<NavMesh>, Ref<NavMeshStatus>)>,
) {
    let (navmesh_handle, status) = navmesh.single();
    if !status.is_changed() {
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
            .spawn(PbrBundle {
                mesh: meshes
                    .add(Plane3d::new(
                        Vec3::Y,
                        Vec2::new(MESH_WIDTH as f32 / 2.0, MESH_HEIGHT as f32 / 2.0),
                    ))
                    .into(),
                transform: Transform::from_translation(Vec3::new(
                    (MESH_WIDTH as f32) / 2.0,
                    0.0,
                    MESH_HEIGHT as f32 / 2.0,
                )),
                material: materials.add(StandardMaterial::from(Color::Srgba(
                    palettes::tailwind::BLUE_800,
                ))),
                ..default()
            })
            .with_children(|main_mesh| {
                main_mesh.spawn((
                    PbrBundle {
                        mesh: meshes.add(navmesh.to_wireframe_mesh()).into(),
                        material: materials.add(StandardMaterial::from(Color::Srgba(
                            palettes::tailwind::BLUE_400,
                        ))),
                        transform: Transform::from_translation(Vec3::new(
                            -(MESH_WIDTH as f32) / 2.0,
                            0.1,
                            -(MESH_HEIGHT as f32) / 2.0,
                        )),
                        ..default()
                    },
                    NotShadowCaster,
                ));
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
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
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
            .and_then(|ray| {
                ray.intersect_plane(Vec3::ZERO, InfinitePlane3d::new(Vec3::Y))
                    .map(|d| (ray, d))
            })
            .map(|(ray, d)| ray.get_point(d))
        {
            let mut rng = rand::thread_rng();
            let mat = materials.add(StandardMaterial::from(Color::Srgba(
                palettes::tailwind::RED_700,
            )));
            let transform = Transform::from_translation(position)
                .with_rotation(Quat::from_rotation_y(rng.gen_range(0.0..PI)));
            new_obstacle(&mut commands, &mut rng, transform, &mut *meshes, &mat);
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
