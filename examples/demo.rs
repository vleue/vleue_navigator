use std::f32::consts::{FRAC_PI_2, PI};

use bevy::{
    color::palettes, math::vec2, pbr::NotShadowCaster, prelude::*, render::view::RenderLayers,
};
use polyanya::Triangulation;
use rand::Rng;
use vleue_navigator::prelude::*;

#[path = "helpers/agent3d.rs"]
mod agent3d;
#[path = "helpers/ui.rs"]
mod ui;

const MESH_WIDTH: u32 = 150;
const MESH_HEIGHT: u32 = 100;

pub const MATERIAL_OBSTACLE_1: Handle<StandardMaterial> = Handle::weak_from_u128(0);
pub const MATERIAL_OBSTACLE_2: Handle<StandardMaterial> = Handle::weak_from_u128(1);
pub const MATERIAL_OBSTACLE_3: Handle<StandardMaterial> = Handle::weak_from_u128(2);
pub const MATERIAL_NAVMESH: Handle<StandardMaterial> = Handle::weak_from_u128(3);

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
            NavmeshUpdaterPlugin::<PrimitiveObstacle>::default(),
        ))
        .add_systems(
            Startup,
            (
                setup,
                ui::setup_stats::<false>,
                ui::setup_settings,
                agent3d::setup_agent::<100>,
            ),
        )
        .add_systems(
            Update,
            (
                display_mesh,
                ui::update_stats::<PrimitiveObstacle>,
                remove_obstacles,
                ui::display_settings,
                ui::update_settings::<10>,
                agent3d::give_target_to_navigator::<10, MESH_WIDTH, MESH_HEIGHT>,
                agent3d::move_navigator,
                agent3d::display_navigator_path,
                agent3d::refresh_path::<100, MESH_WIDTH, MESH_HEIGHT>,
                life_of_obstacle,
                ui::toggle_ui,
                toggle_ui,
            ),
        )
        .add_systems(FixedUpdate, random_obstacle)
        .insert_resource(Time::<Fixed>::from_seconds(0.25))
        .run();
}

#[derive(Component)]
struct Lifetime(Timer);

fn life_of_obstacle(
    mut commands: Commands,
    time: Res<Time>,
    mut obstacles: Query<(Entity, &mut Lifetime, &mut Transform)>,
) {
    for (entity, mut lifetime, mut transform) in obstacles.iter_mut() {
        lifetime.0.tick(time.delta());
        if lifetime.0.fraction() < 0.2 {
            transform.scale = Vec3::new(
                lifetime.0.fraction() * 5.0,
                1.0,
                lifetime.0.fraction() * 5.0,
            );
        }
        if lifetime.0.fraction() > 0.8 {
            transform.scale = Vec3::new(
                (-lifetime.0.fraction() + 1.0) * 5.0 + 0.01,
                1.0,
                (-lifetime.0.fraction() + 1.0) * 5.0 + 0.01,
            );
        }
        if lifetime.0.finished() {
            commands.entity(entity).despawn_recursive();
        }
    }
}

fn random_obstacle(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    let mut rng = rand::thread_rng();
    let mat = match rand::thread_rng().gen_range(0..3) {
        0 => MATERIAL_OBSTACLE_1,
        1 => MATERIAL_OBSTACLE_2,
        2 => MATERIAL_OBSTACLE_3,
        _ => unreachable!(),
    };
    let transform = Transform::from_translation(Vec3::new(
        rng.gen_range(0.0..(MESH_WIDTH as f32)),
        0.0,
        rng.gen_range(0.0..(MESH_HEIGHT as f32)),
    ))
    .with_rotation(Quat::from_rotation_y(rng.gen_range(0.0..PI)))
    .with_scale(Vec3::splat(0.0));
    new_obstacle(&mut commands, &mut rng, transform, &mut *meshes, &mat);
}

fn setup(mut commands: Commands, mut materials: ResMut<Assets<StandardMaterial>>) {
    commands.spawn((
        Camera3dBundle {
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
        },
        RenderLayers::default().with(1),
    ));
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
            simplify: 0.101,
            merge_steps: 3,
            build_timeout: Some(0.5),
            ..default()
        },
        transform: Transform::from_rotation(Quat::from_rotation_x(-FRAC_PI_2)),
        // Mark it for update as soon as obstacles are changed.
        // Other modes can be debounced or manually triggered.
        update_mode: NavMeshUpdateMode::Debounced(0.2),
        ..default()
    });

    materials.insert(
        &MATERIAL_OBSTACLE_1,
        StandardMaterial::from(Color::Srgba(palettes::tailwind::RED_600)),
    );
    materials.insert(
        &MATERIAL_OBSTACLE_2,
        StandardMaterial::from(Color::Srgba(palettes::tailwind::RED_700)),
    );
    materials.insert(
        &MATERIAL_OBSTACLE_3,
        StandardMaterial::from(Color::Srgba(palettes::tailwind::ORANGE_700)),
    );
    materials.insert(
        &MATERIAL_NAVMESH,
        StandardMaterial::from(Color::Srgba(palettes::tailwind::BLUE_800)),
    );
}

fn new_obstacle(
    commands: &mut Commands,
    rng: &mut impl Rng,
    transform: Transform,
    meshes: &mut Assets<Mesh>,
    mat: &Handle<StandardMaterial>,
) {
    let radius = 1.0;
    match rng.gen_range(0..6) {
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
                    Lifetime(Timer::from_seconds(
                        rng.gen_range(20.0..40.0),
                        TimerMode::Once,
                    )),
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
                    Lifetime(Timer::from_seconds(
                        rng.gen_range(20.0..40.0),
                        TimerMode::Once,
                    )),
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
                    Lifetime(Timer::from_seconds(
                        rng.gen_range(20.0..40.0),
                        TimerMode::Once,
                    )),
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
            let primitive = Capsule2d::new(rng.gen_range(1.0..3.0), rng.gen_range(1.5..5.0));
            let larger_primitive =
                Capsule2d::new(primitive.radius + radius, primitive.half_length * 2.0);
            commands
                .spawn((
                    transform,
                    GlobalTransform::default(),
                    PrimitiveObstacle::Capsule(larger_primitive),
                    Lifetime(Timer::from_seconds(
                        rng.gen_range(20.0..40.0),
                        TimerMode::Once,
                    )),
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
            let primitive = RegularPolygon::new(rng.gen_range(1.0..5.0), rng.gen_range(3..11));
            let larger_primitive =
                RegularPolygon::new(primitive.circumradius() + radius, primitive.sides);
            commands
                .spawn((
                    transform,
                    GlobalTransform::default(),
                    PrimitiveObstacle::RegularPolygon(larger_primitive),
                    Lifetime(Timer::from_seconds(
                        rng.gen_range(20.0..40.0),
                        TimerMode::Once,
                    )),
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
                    Lifetime(Timer::from_seconds(
                        rng.gen_range(20.0..40.0),
                        TimerMode::Once,
                    )),
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
                material: MATERIAL_NAVMESH,
                ..default()
            })
            .with_children(|main_mesh| {
                main_mesh.spawn((
                    PbrBundle {
                        mesh: meshes.add(navmesh.to_wireframe_mesh()).into(),
                        material: MATERIAL_NAVMESH,
                        transform: Transform::from_translation(Vec3::new(
                            -(MESH_WIDTH as f32) / 2.0,
                            0.1,
                            -(MESH_HEIGHT as f32) / 2.0,
                        )),
                        ..default()
                    },
                    NotShadowCaster,
                    RenderLayers::none().with(1),
                ));
            })
            .id(),
    );
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

fn toggle_ui(
    mut layers: Query<&mut RenderLayers, With<Camera>>,
    mut entered: EventReader<CursorEntered>,
    mut left: EventReader<CursorLeft>,
) {
    for _ in entered.read() {
        for mut layers in &mut layers {
            *layers = layers.clone().with(1);
        }
    }
    for _ in left.read() {
        for mut layers in &mut layers {
            *layers = layers.clone().without(1);
        }
    }
}
