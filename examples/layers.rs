use std::{f32::consts::FRAC_PI_2, time::Duration};

use avian3d::{math::Vector, prelude::*};
use bevy::{
    color::palettes,
    math::{vec2, vec3},
    prelude::*,
    time::common_conditions::on_timer,
};
use polyanya::Triangulation;
use rand::Rng;
use vleue_navigator::prelude::*;

const MESH_UNIT: u32 = 100;

#[derive(Component)]
struct Obstacle(Timer);

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
            PhysicsPlugins::default().with_length_unit(1.0),
            VleueNavigatorPlugin,
            NavmeshUpdaterPlugin::<Collider, Obstacle>::default(),
        ))
        .insert_resource(Gravity(Vector::NEG_Y * 9.81 * 10.0))
        .add_systems(Startup, (setup,))
        .add_systems(Update, (rotate_camera, despawn_obstacles))
        .add_systems(
            Update,
            spawn_obstacles.run_if(on_timer(Duration::from_secs_f32(0.5))),
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
            -(MESH_UNIT as f32) * 1.5,
            MESH_UNIT as f32 * 5.0,
            -(MESH_UNIT as f32) * 1.5,
        )
        .looking_at(
            vec3(MESH_UNIT as f32 * 1.0, 0.0, MESH_UNIT as f32 * 1.0),
            Vec3::Y,
        ),
        ..Default::default()
    });
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: 3000.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::default().looking_at(Vec3::new(-1.0, -2.5, -1.5), Vec3::Y),
        ..default()
    });

    // GROUND LEVEL
    {
        commands.spawn((
            NavMeshBundle {
                settings: NavMeshSettings {
                    fixed: Triangulation::from_outer_edges(&[
                        vec2(-(MESH_UNIT as f32 / 2.0), -(MESH_UNIT as f32 * 2.0)),
                        vec2(MESH_UNIT as f32 / 2.0, -(MESH_UNIT as f32 * 2.0)),
                        vec2(MESH_UNIT as f32 * 2.5, -(MESH_UNIT as f32 * 2.0)),
                        vec2(MESH_UNIT as f32 * 2.5, -(MESH_UNIT as f32 * 2.5)),
                        vec2(MESH_UNIT as f32 * 2.5, -(MESH_UNIT as f32 * 3.5)),
                        vec2(MESH_UNIT as f32 * 3.5, -(MESH_UNIT as f32 * 3.5)),
                        vec2(MESH_UNIT as f32 * 3.5, -(MESH_UNIT as f32 * 1.0)),
                        vec2(MESH_UNIT as f32 * 2.5, -(MESH_UNIT as f32 * 1.0)),
                        vec2(MESH_UNIT as f32 / 2.0, -(MESH_UNIT as f32 * 1.0)),
                        vec2(MESH_UNIT as f32 / 2.0, -(MESH_UNIT as f32 / 2.0)),
                        vec2(MESH_UNIT as f32 / 2.0, MESH_UNIT as f32 / 2.0),
                        vec2(-(MESH_UNIT as f32) / 2.0, MESH_UNIT as f32 / 2.0),
                    ]),
                    simplify: 0.001,
                    merge_steps: 0,
                    up: Some((Dir3::Y, 1.0)),
                    ..default()
                },
                transform: Transform::from_rotation(Quat::from_rotation_x(-FRAC_PI_2)),
                update_mode: NavMeshUpdateMode::Direct,
                handle: Handle::<NavMesh>::weak_from_u128(0),
                ..default()
            },
            NavMeshDebug(palettes::tailwind::FUCHSIA_600.into()),
        ));
        commands.spawn((
            PbrBundle {
                mesh: meshes.add(Plane3d::new(
                    Vec3::Z,
                    Vec2::new(MESH_UNIT as f32 / 2.0, MESH_UNIT as f32 * 1.25),
                )),
                material: materials.add(StandardMaterial::from(Color::Srgba(
                    palettes::tailwind::BLUE_800,
                ))),
                transform: Transform::from_translation(vec3(0.0, 0.0, MESH_UNIT as f32 * 0.75))
                    .with_rotation(Quat::from_rotation_x(-FRAC_PI_2)),

                ..default()
            },
            RigidBody::Static,
            Collider::cuboid(MESH_UNIT as f32, MESH_UNIT as f32 * 2.5, 0.01),
        ));
        commands.spawn((
            PbrBundle {
                mesh: meshes.add(Plane3d::new(
                    Vec3::Z,
                    Vec2::new(MESH_UNIT as f32, MESH_UNIT as f32 / 2.0),
                )),
                material: materials.add(StandardMaterial::from(Color::Srgba(
                    palettes::tailwind::BLUE_800,
                ))),
                transform: Transform::from_translation(vec3(
                    MESH_UNIT as f32 * 3.0 / 2.0,
                    0.0,
                    MESH_UNIT as f32 * 3.0 / 2.0,
                ))
                .with_rotation(Quat::from_rotation_x(-FRAC_PI_2)),
                ..default()
            },
            RigidBody::Static,
            Collider::cuboid(MESH_UNIT as f32 * 2.0, MESH_UNIT as f32, 0.1),
        ));
        commands.spawn((
            PbrBundle {
                mesh: meshes.add(Plane3d::new(
                    Vec3::Z,
                    Vec2::new(MESH_UNIT as f32 / 2.0, MESH_UNIT as f32 * 1.25),
                )),
                material: materials.add(StandardMaterial::from(Color::Srgba(
                    palettes::tailwind::BLUE_800,
                ))),
                transform: Transform::from_translation(vec3(
                    MESH_UNIT as f32 * 3.0,
                    0.0,
                    MESH_UNIT as f32 * 2.25,
                ))
                .with_rotation(Quat::from_rotation_x(-FRAC_PI_2)),
                ..default()
            },
            RigidBody::Static,
            Collider::cuboid(MESH_UNIT as f32, MESH_UNIT as f32 * 2.5, 0.1),
        ));
    }

    // UPPER LEVEL
    {
        commands.spawn((
            NavMeshBundle {
                settings: NavMeshSettings {
                    fixed: Triangulation::from_outer_edges(&[
                        vec2(-(MESH_UNIT as f32 / 2.0), MESH_UNIT as f32 * 1.0),
                        vec2(-(MESH_UNIT as f32 / 2.0), -(MESH_UNIT as f32 * 2.0)),
                        vec2(MESH_UNIT as f32 / 2.0, -(MESH_UNIT as f32 * 2.0)),
                        vec2(MESH_UNIT as f32 / 2.0, -(MESH_UNIT as f32)),
                        vec2(MESH_UNIT as f32 / 2.0, MESH_UNIT as f32 * 2.0),
                        vec2(-(MESH_UNIT as f32 / 2.0), MESH_UNIT as f32 * 2.0),
                    ]),
                    simplify: 0.001,
                    merge_steps: 0,
                    up: Some((Dir3::Y, 1.0)),
                    ..default()
                },
                transform: Transform::from_translation(vec3(
                    MESH_UNIT as f32 * 3.0 / 2.0,
                    MESH_UNIT as f32 / 4.0,
                    MESH_UNIT as f32 * 3.0 / 2.0,
                ))
                .with_rotation(Quat::from_rotation_x(-FRAC_PI_2)),
                update_mode: NavMeshUpdateMode::Direct,
                handle: Handle::<NavMesh>::weak_from_u128(1),
                ..default()
            },
            NavMeshDebug(palettes::tailwind::YELLOW_600.into()),
        ));
        commands.spawn((
            PbrBundle {
                mesh: meshes.add(Plane3d::new(
                    Vec3::Z,
                    Vec2::new(MESH_UNIT as f32 / 2.0, MESH_UNIT as f32 * 2.0),
                )),
                material: materials.add(StandardMaterial::from(Color::Srgba(
                    palettes::tailwind::BLUE_800.with_alpha(1.0),
                ))),
                transform: Transform::from_translation(vec3(
                    MESH_UNIT as f32 * 3.0 / 2.0,
                    MESH_UNIT as f32 / 4.0,
                    MESH_UNIT as f32 * 3.0 / 2.0,
                ))
                .with_rotation(Quat::from_rotation_x(-FRAC_PI_2)),
                ..default()
            },
            RigidBody::Static,
            Collider::cuboid(MESH_UNIT as f32, MESH_UNIT as f32 * 4.0, 0.01),
        ));
    }

    // RAMPS
    {
        commands.spawn((
            NavMeshBundle {
                settings: NavMeshSettings {
                    fixed: Triangulation::from_outer_edges(&[
                        vec2(
                            -(MESH_UNIT as f32 / 4.0 / (0.5_f32).atan().cos()),
                            -(MESH_UNIT as f32 / 2.0),
                        ),
                        vec2(
                            MESH_UNIT as f32 / 4.0 / (0.5_f32).atan().cos(),
                            -(MESH_UNIT as f32 / 2.0),
                        ),
                        vec2(
                            MESH_UNIT as f32 / 4.0 / (0.5_f32).atan().cos(),
                            MESH_UNIT as f32 / 2.0,
                        ),
                        vec2(
                            -(MESH_UNIT as f32 / 4.0 / (0.5_f32).atan().cos()),
                            MESH_UNIT as f32 / 2.0,
                        ),
                    ]),
                    simplify: 0.001,
                    merge_steps: 0,
                    up: Some((Dir3::new(vec3(-1.0, 2.0, 0.0)).unwrap(), 0.1)),
                    ..default()
                },
                transform: Transform::from_translation(vec3(
                    MESH_UNIT as f32 / 2.0 + MESH_UNIT as f32 / 4.0,
                    MESH_UNIT as f32 / 8.0,
                    0.0,
                ))
                .with_rotation(
                    Quat::from_rotation_x(-FRAC_PI_2) * Quat::from_rotation_y((-0.5_f32).atan()),
                ),
                update_mode: NavMeshUpdateMode::Direct,
                handle: Handle::<NavMesh>::weak_from_u128(2),
                ..default()
            },
            NavMeshDebug(palettes::tailwind::RED_600.into()),
        ));
        commands.spawn((
            PbrBundle {
                mesh: meshes.add(Plane3d::new(
                    Vec3::Z,
                    Vec2::new(
                        MESH_UNIT as f32 / 4.0 / (0.5_f32).atan().cos(),
                        MESH_UNIT as f32 / 2.0,
                    ),
                )),
                material: materials.add(StandardMaterial::from(Color::Srgba(
                    palettes::tailwind::BLUE_800,
                ))),
                transform: Transform::from_translation(vec3(
                    MESH_UNIT as f32 / 2.0 + MESH_UNIT as f32 / 4.0,
                    MESH_UNIT as f32 / 8.0,
                    0.0,
                ))
                .with_rotation(
                    Quat::from_rotation_x(-FRAC_PI_2) * Quat::from_rotation_y((-0.5_f32).atan()),
                ),
                ..default()
            },
            RigidBody::Static,
            Collider::cuboid(
                MESH_UNIT as f32 / 2.0 / (0.5_f32).atan().cos(),
                MESH_UNIT as f32,
                0.01,
            ),
        ));

        commands.spawn((
            NavMeshBundle {
                settings: NavMeshSettings {
                    fixed: Triangulation::from_outer_edges(&[
                        vec2(
                            -(MESH_UNIT as f32 / 4.0 / (0.5_f32).atan().cos()),
                            -(MESH_UNIT as f32 / 2.0),
                        ),
                        vec2(
                            MESH_UNIT as f32 / 4.0 / (0.5_f32).atan().cos(),
                            -(MESH_UNIT as f32 / 2.0),
                        ),
                        vec2(
                            MESH_UNIT as f32 / 4.0 / (0.5_f32).atan().cos(),
                            MESH_UNIT as f32 / 2.0,
                        ),
                        vec2(
                            -(MESH_UNIT as f32 / 4.0 / (0.5_f32).atan().cos()),
                            MESH_UNIT as f32 / 2.0,
                        ),
                    ]),
                    simplify: 0.001,
                    merge_steps: 0,
                    up: Some((Dir3::new(vec3(1.0, 2.0, 0.0)).unwrap(), 0.1)),
                    ..default()
                },
                transform: Transform::from_translation(vec3(
                    MESH_UNIT as f32 / 2.0 + MESH_UNIT as f32 / 4.0 + MESH_UNIT as f32 * 3.0 / 2.0,
                    MESH_UNIT as f32 / 8.0,
                    MESH_UNIT as f32 * 3.0,
                ))
                .with_rotation(
                    Quat::from_rotation_x(-FRAC_PI_2) * Quat::from_rotation_y(0.5_f32.atan()),
                ),
                update_mode: NavMeshUpdateMode::Direct,
                handle: Handle::<NavMesh>::weak_from_u128(3),
                ..default()
            },
            NavMeshDebug(palettes::tailwind::RED_600.into()),
        ));
        commands.spawn((
            PbrBundle {
                mesh: meshes.add(Plane3d::new(
                    Vec3::Z,
                    Vec2::new(
                        MESH_UNIT as f32 / 4.0 / (0.5_f32).atan().cos(),
                        MESH_UNIT as f32 / 2.0,
                    ),
                )),
                material: materials.add(StandardMaterial::from(Color::Srgba(
                    palettes::tailwind::BLUE_800,
                ))),
                transform: Transform::from_translation(vec3(
                    MESH_UNIT as f32 / 2.0 + MESH_UNIT as f32 / 4.0 + MESH_UNIT as f32 * 3.0 / 2.0,
                    MESH_UNIT as f32 / 8.0,
                    MESH_UNIT as f32 * 3.0,
                ))
                .with_rotation(
                    Quat::from_rotation_x(-FRAC_PI_2) * Quat::from_rotation_y(0.5_f32.atan()),
                ),
                ..default()
            },
            RigidBody::Static,
            Collider::cuboid(
                MESH_UNIT as f32 / 2.0 / (0.5_f32).atan().cos(),
                MESH_UNIT as f32,
                0.01,
            ),
        ));
    }
}

fn rotate_camera(time: Res<Time>, mut query: Query<&mut Transform, With<Camera3d>>) {
    for mut transform in query.iter_mut() {
        transform.rotate_around(
            vec3(MESH_UNIT as f32 * 1.5, 0.0, MESH_UNIT as f32 * 1.5),
            Quat::from_rotation_y(time.delta_seconds() / 10.0),
        )
    }
}

fn spawn_obstacles(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    navmeshes: Res<Assets<NavMesh>>,
) {
    let cube_size = 10.0;
    loop {
        let x = rand::thread_rng().gen_range(0.0..(MESH_UNIT as f32 * 3.0));
        let z = rand::thread_rng().gen_range(0.0..(MESH_UNIT as f32 * 3.0));
        if navmeshes
            .iter()
            .any(|(_, nm)| nm.transformed_is_in_mesh(vec3(x, 0.0, z)))
        {
            commands.spawn((
                PbrBundle {
                    mesh: meshes.add(Cuboid::new(cube_size, cube_size, cube_size)),
                    material: materials.add(Color::srgb(0.2, 0.7, 0.9)),
                    transform: Transform::from_xyz(x, 50.0, z).looking_to(
                        Vec3::new(
                            rand::thread_rng().gen_range(-1.0..1.0),
                            rand::thread_rng().gen_range(-1.0..1.0),
                            rand::thread_rng().gen_range(-1.0..1.0),
                        )
                        .normalize(),
                        Vec3::Y,
                    ),
                    ..default()
                },
                RigidBody::Dynamic,
                Collider::cuboid(cube_size, cube_size, cube_size),
                Obstacle(Timer::from_seconds(30.0, TimerMode::Once)),
            ));
            return;
        }
    }
}

fn despawn_obstacles(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Obstacle)>,
) {
    for (entity, mut obstacle) in &mut query {
        if obstacle.0.tick(time.delta()).just_finished() {
            commands.entity(entity).despawn();
        }
    }
}
