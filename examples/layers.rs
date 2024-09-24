use std::f32::consts::FRAC_PI_2;

use avian3d::{math::Vector, prelude::*};
use bevy::{
    color::palettes,
    math::{vec2, vec3},
    prelude::*,
};
use polyanya::Triangulation;
use vleue_navigator::prelude::*;

const MESH_UNIT: u32 = 100;

#[derive(Component)]
enum Obstacle {
    Rotating(f32),
    Sliding(f32),
}

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
        .add_systems(Startup, setup)
        .add_systems(Update, (rotate_camera, display_path))
        .add_systems(Update, move_obstacles)
        .run();
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
                        merge_steps: 2,
                        upward_shift: 1.0,
                        layer: Some(0),
                        stitches: stitches.clone(),
                        ..default()
                    },
                    update_mode: NavMeshUpdateMode::Direct,
                    ..NavMeshBundle::with_unique_id(0)
                },
                NavMeshDebug(palettes::tailwind::FUCHSIA_800.into()),
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
                        merge_steps: 2,
                        upward_shift: 1.0,
                        layer: Some(1),
                        stitches: stitches.clone(),
                        ..default()
                    },
                    update_mode: NavMeshUpdateMode::Direct,
                    ..NavMeshBundle::with_unique_id(0)
                },
                NavMeshDebug(palettes::tailwind::YELLOW_800.into()),
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

    // Ramps
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
                        merge_steps: 2,
                        upward_shift: 0.5,
                        layer: Some(2),
                        stitches: stitches.clone(),
                        scale: vec2(1.0 / (0.5_f32).atan().cos(), 1.0),
                        ..default()
                    },
                    update_mode: NavMeshUpdateMode::Direct,
                    ..NavMeshBundle::with_unique_id(0)
                },
                NavMeshDebug(palettes::tailwind::TEAL_800.into()),
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
                        merge_steps: 2,
                        upward_shift: 0.5,
                        layer: Some(3),
                        scale: vec2(1.0 / (0.5_f32).atan().cos(), 1.0),
                        stitches: stitches.clone(),
                        ..default()
                    },
                    update_mode: NavMeshUpdateMode::Direct,
                    ..NavMeshBundle::with_unique_id(0)
                },
                NavMeshDebug(palettes::tailwind::TEAL_800.into()),
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

    // Obstacles
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Cuboid::new(5.0, 10.0, 125.0)),
            material: materials.add(Color::srgb(0.2, 0.7, 0.9)),
            transform: Transform::from_xyz(MESH_UNIT as f32 * 1.5, 25.0, MESH_UNIT as f32 * 1.5),
            ..default()
        },
        RigidBody::Static,
        Collider::cuboid(5.0, 10.0, 125.0),
        Obstacle::Rotating(-2.0),
    ));
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Cuboid::new(75.0, 10.0, 5.0)),
            material: materials.add(Color::srgb(0.2, 0.7, 0.9)),
            transform: Transform::from_xyz(MESH_UNIT as f32 * 0.0, 0.0, MESH_UNIT as f32 * 0.75),
            ..default()
        },
        RigidBody::Static,
        Collider::cuboid(75.0, 10.0, 5.0),
        Obstacle::Sliding(0.0),
    ));
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Cuboid::new(75.0, 10.0, 5.0)),
            material: materials.add(Color::srgb(0.2, 0.7, 0.9)),
            transform: Transform::from_xyz(MESH_UNIT as f32 * 3.0, 0.0, MESH_UNIT as f32 * 2.25),
            ..default()
        },
        RigidBody::Static,
        Collider::cuboid(75.0, 10.0, 5.0),
        Obstacle::Sliding(MESH_UNIT as f32 * 3.0),
    ));
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Cuboid::new(75.0, 10.0, 5.0)),
            material: materials.add(Color::srgb(0.2, 0.7, 0.9)),
            transform: Transform::from_xyz(MESH_UNIT as f32 * 1.75, 25.0, MESH_UNIT as f32 * 0.75),
            ..default()
        },
        RigidBody::Static,
        Collider::cuboid(75.0, 10.0, 5.0),
        Obstacle::Rotating(0.7),
    ));
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Cuboid::new(75.0, 10.0, 5.0)),
            material: materials.add(Color::srgb(0.2, 0.7, 0.9)),
            transform: Transform::from_xyz(MESH_UNIT as f32 * 1.25, 25.0, MESH_UNIT as f32 * 2.25),
            ..default()
        },
        RigidBody::Static,
        Collider::cuboid(75.0, 10.0, 5.0),
        Obstacle::Rotating(1.0),
    ));

    // Endpoints
    commands.spawn(PbrBundle {
        mesh: meshes.add(Capsule3d::new(2.0, 2.0).mesh()),
        material: materials.add(StandardMaterial::from(Color::Srgba(
            palettes::tailwind::RED_600,
        ))),
        transform: Transform::from_translation(vec3(
            MESH_UNIT as f32 * 1.5,
            25.0,
            -(MESH_UNIT as f32) / 4.0,
        )),
        ..default()
    });
    commands.spawn(PbrBundle {
        mesh: meshes.add(Capsule3d::new(2.0, 2.0).mesh()),
        material: materials.add(StandardMaterial::from(Color::Srgba(
            palettes::tailwind::RED_600,
        ))),
        transform: Transform::from_translation(vec3(
            MESH_UNIT as f32 * 1.5,
            25.0,
            MESH_UNIT as f32 * 3.25,
        )),
        ..default()
    });
}

fn rotate_camera(time: Res<Time>, mut query: Query<&mut Transform, With<Camera3d>>) {
    for mut transform in query.iter_mut() {
        transform.rotate_around(
            vec3(MESH_UNIT as f32 * 1.5, 0.0, MESH_UNIT as f32 * 1.5),
            Quat::from_rotation_y(time.delta_seconds() / 6.0),
        )
    }
}

fn move_obstacles(mut query: Query<(&mut Transform, &Obstacle)>, time: Res<Time>) {
    for (mut transform, obstacle) in query.iter_mut() {
        match obstacle {
            Obstacle::Rotating(speed) => {
                transform.rotate(Quat::from_rotation_y(time.delta_seconds() / speed))
            }
            Obstacle::Sliding(center_x) => {
                transform.translation.x =
                    center_x + (time.elapsed_seconds() as f32 * 4.0).sin() * 35.0;
            }
        }
    }
}

fn display_path(navmeshes: Res<Assets<NavMesh>>, mut gizmos: Gizmos) {
    let Some(navmesh) = navmeshes.get(Handle::<NavMesh>::weak_from_u128(0).id()) else {
        return;
    };
    for points in [(
        vec2(MESH_UNIT as f32 * 1.5, -(MESH_UNIT as f32) / 4.0),
        vec2(MESH_UNIT as f32 * 1.5, MESH_UNIT as f32 * 3.25),
    )] {
        let Some(start) = navmesh.get().get_point_layer(points.0).get(0).cloned() else {
            continue;
        };
        let Some(path) = navmesh.path(points.0, points.1) else {
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
                point_to_height(points.0, start.layer().unwrap()),
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
