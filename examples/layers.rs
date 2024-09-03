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
    let mut app = App::new();
    app.insert_resource(ClearColor(palettes::css::BLACK.into()))
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
        .add_systems(Update, (rotate_camera, despawn_obstacles, display_path))
        .add_systems(Update, rotate_obstacle);
    // .add_systems(
    //     Update,
    //     spawn_obstacles.run_if(on_timer(Duration::from_secs_f32(0.5))),
    // );

    // let mut config_store = app
    //     .world_mut()
    //     .get_resource_mut::<GizmoConfigStore>()
    //     .unwrap();
    // for (_, config, _) in config_store.iter_mut() {
    //     config.depth_bias = -1.0;
    // }

    app.run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let stitches = vec![
        (
            (0, 2),
            [
                vec2(MESH_UNIT as f32 / 2.0, MESH_UNIT as f32 / 2.0),
                vec2(MESH_UNIT as f32 / 2.0, -(MESH_UNIT as f32 / 2.0)),
            ],
        ),
        (
            (0, 3),
            [
                vec2(
                    MESH_UNIT as f32 * 2.5,
                    MESH_UNIT as f32 * 3.0 + MESH_UNIT as f32 / 2.0,
                ),
                vec2(
                    MESH_UNIT as f32 * 2.5,
                    MESH_UNIT as f32 * 3.0 - (MESH_UNIT as f32 / 2.0),
                ),
            ],
        ),
        (
            (1, 2),
            [
                vec2(MESH_UNIT as f32, MESH_UNIT as f32 / 2.0),
                vec2(MESH_UNIT as f32, -(MESH_UNIT as f32 / 2.0)),
            ],
        ),
        (
            (1, 3),
            [
                vec2(
                    MESH_UNIT as f32 * 2.0,
                    MESH_UNIT as f32 * 3.0 + MESH_UNIT as f32 / 2.0,
                ),
                vec2(
                    MESH_UNIT as f32 * 2.0,
                    MESH_UNIT as f32 * 3.0 - (MESH_UNIT as f32 / 2.0),
                ),
            ],
        ),
    ];

    // commands.spawn((
    //     PbrBundle {
    //         mesh: meshes.add(Capsule3d::new(2.0, MESH_UNIT as f32).mesh()),
    //         material: materials.add(StandardMaterial::from(Color::Srgba(
    //             palettes::tailwind::RED_600,
    //         ))),
    //         transform: Transform::from_translation(vec3(
    //             MESH_UNIT as f32 * 2.0,
    //             0.0,
    //             MESH_UNIT as f32 * 3.0 + MESH_UNIT as f32 / 2.0,
    //         )),
    //         ..default()
    //     },
    //     RigidBody::Static,
    //     Collider::capsule(2.0, MESH_UNIT as f32 * 2.5),
    // ));
    // commands.spawn((
    //     PbrBundle {
    //         mesh: meshes.add(Capsule3d::new(2.0, MESH_UNIT as f32).mesh()),
    //         material: materials.add(StandardMaterial::from(Color::Srgba(
    //             palettes::tailwind::RED_600,
    //         ))),
    //         transform: Transform::from_translation(vec3(
    //             MESH_UNIT as f32 * 2.0,
    //             0.0,
    //             MESH_UNIT as f32 * 3.0 - (MESH_UNIT as f32 / 2.0),
    //         )),
    //         ..default()
    //     },
    //     RigidBody::Static,
    //     Collider::capsule(2.0, MESH_UNIT as f32 * 2.5),
    // ));

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

    // ground level
    commands
        .spawn(SpatialBundle::from_transform(Transform::from_rotation(
            Quat::from_rotation_x(FRAC_PI_2),
        )))
        .with_children(|p| {
            p.spawn((
                NavMeshBundle {
                    settings: NavMeshSettings {
                        fixed: Triangulation::from_outer_edges(&[
                            vec2(-(MESH_UNIT as f32 / 2.0), MESH_UNIT as f32 * 2.0),
                            vec2(MESH_UNIT as f32 * 2.5, MESH_UNIT as f32 * 2.0),
                            vec2(MESH_UNIT as f32 * 2.5, MESH_UNIT as f32 * 2.5),
                            vec2(MESH_UNIT as f32 * 2.5, MESH_UNIT as f32 * 3.5),
                            vec2(MESH_UNIT as f32 * 3.5, MESH_UNIT as f32 * 3.5),
                            vec2(MESH_UNIT as f32 * 3.5, MESH_UNIT as f32 * 1.0),
                            vec2(MESH_UNIT as f32 * 2.5, MESH_UNIT as f32 * 1.0),
                            vec2(MESH_UNIT as f32 / 2.0, MESH_UNIT as f32 * 1.0),
                            vec2(MESH_UNIT as f32 / 2.0, MESH_UNIT as f32 / 2.0),
                            vec2(MESH_UNIT as f32 / 2.0, -(MESH_UNIT as f32 / 2.0)),
                            vec2(-(MESH_UNIT as f32) / 2.0, -(MESH_UNIT as f32 / 2.0)),
                        ]),
                        simplify: 0.001,
                        merge_steps: 0,
                        upward_shift: 1.0,
                        layer: Some(0),
                        stitches: stitches.clone(),
                        ..default()
                    },
                    update_mode: NavMeshUpdateMode::Direct,
                    handle: Handle::<NavMesh>::weak_from_u128(0),
                    ..default()
                },
                NavMeshDebug(palettes::tailwind::FUCHSIA_950.into()),
            ));
            p.spawn((
                PbrBundle {
                    mesh: meshes.add(Plane3d::new(
                        -Vec3::Z,
                        Vec2::new(MESH_UNIT as f32 / 2.0, MESH_UNIT as f32 * 1.25),
                    )),
                    material: materials.add(StandardMaterial::from(Color::Srgba(
                        palettes::tailwind::BLUE_800,
                    ))),
                    transform: Transform::from_translation(vec3(0.0, MESH_UNIT as f32 * 0.75, 0.0)),
                    ..default()
                },
                RigidBody::Static,
                Collider::cuboid(MESH_UNIT as f32, MESH_UNIT as f32 * 2.5, 0.01),
            ));
            p.spawn((
                PbrBundle {
                    mesh: meshes.add(Plane3d::new(
                        -Vec3::Z,
                        Vec2::new(MESH_UNIT as f32, MESH_UNIT as f32 / 2.0),
                    )),
                    material: materials.add(StandardMaterial::from(Color::Srgba(
                        palettes::tailwind::BLUE_800,
                    ))),
                    transform: Transform::from_translation(vec3(
                        MESH_UNIT as f32 * 3.0 / 2.0,
                        MESH_UNIT as f32 * 3.0 / 2.0,
                        0.0,
                    )),
                    ..default()
                },
                RigidBody::Static,
                Collider::cuboid(MESH_UNIT as f32 * 2.0, MESH_UNIT as f32, 0.1),
            ));
            p.spawn((
                PbrBundle {
                    mesh: meshes.add(Plane3d::new(
                        -Vec3::Z,
                        Vec2::new(MESH_UNIT as f32 / 2.0, MESH_UNIT as f32 * 1.25),
                    )),
                    material: materials.add(StandardMaterial::from(Color::Srgba(
                        palettes::tailwind::BLUE_800,
                    ))),
                    transform: Transform::from_translation(vec3(
                        MESH_UNIT as f32 * 3.0,
                        MESH_UNIT as f32 * 2.25,
                        0.0,
                    )),
                    ..default()
                },
                RigidBody::Static,
                Collider::cuboid(MESH_UNIT as f32, MESH_UNIT as f32 * 2.5, 0.1),
            ));
        });

    // upper level
    commands
        .spawn(SpatialBundle::from_transform(
            Transform::from_translation(vec3(
                MESH_UNIT as f32 * 3.0 / 2.0,
                MESH_UNIT as f32 / 4.0,
                MESH_UNIT as f32 * 3.0 / 2.0,
            ))
            .with_rotation(Quat::from_rotation_x(FRAC_PI_2)),
        ))
        .with_children(|p| {
            p.spawn((
                NavMeshBundle {
                    settings: NavMeshSettings {
                        fixed: Triangulation::from_outer_edges(&[
                            vec2(-(MESH_UNIT as f32 / 2.0), -(MESH_UNIT as f32 * 1.0)),
                            vec2(-(MESH_UNIT as f32 / 2.0), MESH_UNIT as f32 * 2.0),
                            vec2(MESH_UNIT as f32 / 2.0, MESH_UNIT as f32 * 2.0),
                            vec2(MESH_UNIT as f32 / 2.0, MESH_UNIT as f32),
                            vec2(MESH_UNIT as f32 / 2.0, -(MESH_UNIT as f32 * 2.0)),
                            vec2(-(MESH_UNIT as f32 / 2.0), -(MESH_UNIT as f32 * 2.0)),
                        ]),
                        simplify: 0.001,
                        merge_steps: 0,
                        upward_shift: 1.0,
                        // up: Some((Dir3::Y, 1.0)),
                        layer: Some(1),
                        stitches: stitches.clone(),
                        ..default()
                    },
                    update_mode: NavMeshUpdateMode::Direct,
                    handle: Handle::<NavMesh>::weak_from_u128(0),
                    ..default()
                },
                NavMeshDebug(palettes::tailwind::YELLOW_950.into()),
            ));
            p.spawn((
                PbrBundle {
                    mesh: meshes.add(Plane3d::new(
                        -Vec3::Z,
                        Vec2::new(MESH_UNIT as f32 / 2.0, MESH_UNIT as f32 * 2.0),
                    )),
                    material: materials.add(StandardMaterial::from(Color::Srgba(
                        palettes::tailwind::BLUE_800.with_alpha(1.0),
                    ))),
                    ..default()
                },
                RigidBody::Static,
                Collider::cuboid(MESH_UNIT as f32, MESH_UNIT as f32 * 4.0, 0.01),
            ));
        });

    // RAMPS
    commands
        .spawn(SpatialBundle::from_transform(
            Transform::from_translation(vec3(
                MESH_UNIT as f32 / 2.0 + MESH_UNIT as f32 / 4.0,
                MESH_UNIT as f32 / 8.0,
                0.0,
            ))
            .with_rotation(
                Quat::from_rotation_x(FRAC_PI_2) * Quat::from_rotation_y(0.5_f32.atan()),
            ),
        ))
        .with_children(|p| {
            p.spawn((
                NavMeshBundle {
                    settings: NavMeshSettings {
                        fixed: Triangulation::from_outer_edges(&[
                            vec2(-(MESH_UNIT as f32 / 4.0), -(MESH_UNIT as f32 / 2.0)),
                            vec2(MESH_UNIT as f32 / 4.0, -(MESH_UNIT as f32 / 2.0)),
                            vec2(MESH_UNIT as f32 / 4.0, MESH_UNIT as f32 / 2.0),
                            vec2(-(MESH_UNIT as f32 / 4.0), MESH_UNIT as f32 / 2.0),
                        ]),
                        simplify: 0.001,
                        merge_steps: 0,
                        upward_shift: 0.5,
                        layer: Some(2),
                        stitches: stitches.clone(),
                        scale: vec2(1.0 / (0.5_f32).atan().cos(), 1.0),
                        ..default()
                    },
                    update_mode: NavMeshUpdateMode::Direct,
                    handle: Handle::<NavMesh>::weak_from_u128(0),
                    ..default()
                },
                NavMeshDebug(palettes::tailwind::TEAL_950.into()),
            ));
            p.spawn((
                PbrBundle {
                    mesh: meshes.add(Plane3d::new(
                        -Vec3::Z,
                        Vec2::new(
                            MESH_UNIT as f32 / 4.0 / (0.5_f32).atan().cos(),
                            MESH_UNIT as f32 / 2.0,
                        ),
                    )),
                    material: materials.add(StandardMaterial::from(Color::Srgba(
                        palettes::tailwind::BLUE_800,
                    ))),
                    ..default()
                },
                RigidBody::Static,
                Collider::cuboid(
                    MESH_UNIT as f32 / 2.0 / (0.5_f32).atan().cos(),
                    MESH_UNIT as f32,
                    0.01,
                ),
            ));
        });

    commands
        .spawn(SpatialBundle::from_transform(
            Transform::from_translation(vec3(
                MESH_UNIT as f32 / 2.0 + MESH_UNIT as f32 / 4.0 + MESH_UNIT as f32 * 3.0 / 2.0,
                MESH_UNIT as f32 / 8.0,
                MESH_UNIT as f32 * 3.0,
            ))
            .with_rotation(
                Quat::from_rotation_x(FRAC_PI_2) * Quat::from_rotation_y(-0.5_f32.atan()),
            ),
        ))
        .with_children(|p| {
            p.spawn((
                NavMeshBundle {
                    settings: NavMeshSettings {
                        fixed: Triangulation::from_outer_edges(&[
                            vec2(-(MESH_UNIT as f32 / 4.0), -(MESH_UNIT as f32 / 2.0)),
                            vec2(MESH_UNIT as f32 / 4.0, -(MESH_UNIT as f32 / 2.0)),
                            vec2(MESH_UNIT as f32 / 4.0, MESH_UNIT as f32 / 2.0),
                            vec2(-(MESH_UNIT as f32 / 4.0), MESH_UNIT as f32 / 2.0),
                        ]),
                        simplify: 0.001,
                        merge_steps: 0,
                        // up: Some((Dir3::new(vec3(1.0, 2.0, 0.0)).unwrap(), 0.5)),
                        upward_shift: 0.5,
                        layer: Some(3),
                        scale: vec2(1.0 / (0.5_f32).atan().cos(), 1.0),
                        stitches: stitches.clone(),
                        ..default()
                    },
                    update_mode: NavMeshUpdateMode::Direct,
                    handle: Handle::<NavMesh>::weak_from_u128(0),
                    ..default()
                },
                NavMeshDebug(palettes::tailwind::TEAL_950.into()),
            ));
            p.spawn((
                PbrBundle {
                    mesh: meshes.add(Plane3d::new(
                        -Vec3::Z,
                        Vec2::new(
                            MESH_UNIT as f32 / 4.0 / (0.5_f32).atan().cos(),
                            MESH_UNIT as f32 / 2.0,
                        ),
                    )),
                    material: materials.add(StandardMaterial::from(Color::Srgba(
                        palettes::tailwind::BLUE_800,
                    ))),
                    ..default()
                },
                RigidBody::Static,
                Collider::cuboid(
                    MESH_UNIT as f32 / 2.0 / (0.5_f32).atan().cos(),
                    MESH_UNIT as f32,
                    0.01,
                ),
            ));
        });

    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Cuboid::new(5.0, 10.0, 125.0)),
            material: materials.add(Color::srgb(0.2, 0.7, 0.9)),
            transform: Transform::from_xyz(MESH_UNIT as f32 * 1.5, 35.0, MESH_UNIT as f32 * 1.5),
            ..default()
        },
        RigidBody::Dynamic,
        Collider::cuboid(5.0, 10.0, 125.0),
        Obstacle(Timer::from_seconds(30000.0, TimerMode::Once)),
    ));
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
    let cube_size = 5.0;
    loop {
        // let x = rand::thread_rng().gen_range(0.0..(MESH_UNIT as f32 * 3.0));
        let x = rand::thread_rng().gen_range((MESH_UNIT as f32 * 1.2)..(MESH_UNIT as f32 * 1.8));
        // let z = rand::thread_rng().gen_range(0.0..(MESH_UNIT as f32 * 3.0));
        let z = rand::thread_rng().gen_range((MESH_UNIT as f32 * 0.8)..(MESH_UNIT as f32 * 2.2));
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

fn rotate_obstacle(mut query: Query<&mut Transform, With<Obstacle>>, time: Res<Time>) {
    for mut transform in query.iter_mut() {
        transform.rotate(Quat::from_rotation_y(time.delta_seconds() / 2.0))
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

fn display_path(navmeshes: Res<Assets<NavMesh>>, mut gizmos: Gizmos) {
    let Some(navmesh) = navmeshes.get(Handle::<NavMesh>::weak_from_u128(0).id()) else {
        return;
    };
    for points in [
        // (
        //     vec2(0.0, 0.0),
        //     vec2(MESH_UNIT as f32 * 3.0, MESH_UNIT as f32 * 3.0),
        // ),
        (
            vec2((MESH_UNIT as f32) * 1.5, -(MESH_UNIT as f32) / 4.0),
            vec2(MESH_UNIT as f32 * 1.5, MESH_UNIT as f32 * 3.25),
        ),
        (
            vec2(MESH_UNIT as f32 * 1.5, MESH_UNIT as f32 * 3.25),
            vec2((MESH_UNIT as f32) * 1.5, -(MESH_UNIT as f32) / 4.0),
        ),
    ] {
        let start = navmesh.get().get_point_layer(points.0)[0];
        let Some(path) = navmesh.path(points.0, points.1) else {
            println!("zut");
            continue;
        };
        let mut path = path
            .path_with_layers
            .iter()
            .map(|(v, layer)| vec3(v.x, point_to_height(v.xy(), *layer), v.y))
            .collect::<Vec<_>>();
        path.insert(
            0,
            vec3(
                points.0.x,
                point_to_height(points.0, start.layer.unwrap()),
                points.0.y,
            ),
        );
        gizmos.linestrip(path, palettes::tailwind::RED_600);
    }
}

fn point_to_height(point: Vec2, layer: u8) -> f32 {
    let top = MESH_UNIT as f32 / 4.0;
    match layer {
        0 => 0.5,
        1 => top + 0.5,
        2 => (point.x - (MESH_UNIT as f32 / 2.0)) / (MESH_UNIT as f32 / 2.0) * top + 0.5,
        3 => {
            (MESH_UNIT as f32 / 2.0 - (point.x - (MESH_UNIT as f32 * 2.0)))
                / (MESH_UNIT as f32 / 2.0)
                * top
                + 0.5
        }
        x => unreachable!("layer {:?}", x),
    }
}
