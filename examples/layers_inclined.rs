use std::{f32::consts::FRAC_PI_2, time::Duration};

use avian3d::{math::Vector, prelude::*};
use bevy::{
    asset::weak_handle,
    color::palettes,
    math::{vec2, vec3},
    prelude::*,
    time::common_conditions::on_timer,
};
use polyanya::Triangulation;
use rand::Rng;
use vleue_navigator::prelude::*;

const MESH_UNIT: u32 = 100;
const UPDATE_MODE: NavMeshUpdateMode = NavMeshUpdateMode::Direct;
const UP_SHIFT: f32 = 0.1;
const RATIO: f32 = 0.8;

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
            PhysicsPlugins::default().with_length_unit(2.0),
            VleueNavigatorPlugin,
            NavmeshUpdaterPlugin::<Collider, Obstacle>::default(),
        ))
        .insert_resource(Gravity(Vector::NEG_Y * 9.81 * 10.0))
        .add_systems(Startup, setup)
        .add_systems(Update, (despawn_obstacles, rotate_camera))
        .add_systems(PostUpdate, display_path)
        .add_systems(
            Update,
            spawn_obstacles.run_if(on_timer(Duration::from_secs_f32(0.5))),
        )
        .run();
}

fn rotate_camera(time: Res<Time>, mut query: Query<&mut Transform, With<Camera3d>>) {
    for mut transform in query.iter_mut() {
        transform.rotate_around(
            vec3(MESH_UNIT as f32 * 1.5, 0.0, MESH_UNIT as f32),
            Quat::from_rotation_y(time.delta_secs() / 10.0),
        )
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, MESH_UNIT as f32 * (3.0 + RATIO), MESH_UNIT as f32)
            .looking_at(vec3(MESH_UNIT as f32 * 1.5, 0.0, MESH_UNIT as f32), Vec3::Y),
    ));
    commands.spawn((
        DirectionalLight {
            illuminance: 3000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::default().looking_at(Vec3::new(-1.0, -2.5, -1.5), Vec3::Y),
    ));

    // side A
    commands
        .spawn((
            Transform::from_rotation(Quat::from_rotation_x(FRAC_PI_2)),
            Visibility::Visible,
        ))
        .with_children(|p| {
            p.spawn((
                NavMeshSettings {
                    fixed: Triangulation::from_outer_edges(&[
                        vec2(0.0, 0.0),
                        vec2(MESH_UNIT as f32, 0.0),
                        vec2(MESH_UNIT as f32, MESH_UNIT as f32 * 2.0),
                        vec2(0.0, MESH_UNIT as f32 * 2.0),
                    ]),
                    simplify: 0.001,
                    merge_steps: 3,
                    upward_shift: UP_SHIFT,
                    layer: Some(0),
                    stitches: vec![
                        (
                            (0, 1),
                            [
                                vec2(MESH_UNIT as f32, 0.0),
                                vec2(MESH_UNIT as f32, MESH_UNIT as f32 * 2.0),
                            ],
                        ),
                        (
                            (1, 2),
                            [
                                vec2(MESH_UNIT as f32 * 2.0, 0.0),
                                vec2(MESH_UNIT as f32 * 2.0, MESH_UNIT as f32 * 2.0),
                            ],
                        ),
                    ],
                    ..default()
                },
                UPDATE_MODE,
                NavMeshDebug(palettes::tailwind::YELLOW_600.into()),
            ));
            p.spawn((
                Mesh3d(meshes.add(Plane3d::new(
                    -Vec3::Z,
                    Vec2::new(MESH_UNIT as f32 / 2.0, MESH_UNIT as f32),
                ))),
                MeshMaterial3d(materials.add(StandardMaterial::from(Color::Srgba(
                    palettes::tailwind::SLATE_900,
                )))),
                Transform::from_xyz(MESH_UNIT as f32 / 2.0, MESH_UNIT as f32, 0.0),
                RigidBody::Static,
                Collider::cuboid(MESH_UNIT as f32, MESH_UNIT as f32 * 2.0, 0.01),
                Restitution::ZERO,
            ));
        });

    // ramp
    let length = MESH_UNIT as f32 / (RATIO.atan().cos());
    commands
        .spawn((
            Transform::from_translation(vec3(MESH_UNIT as f32, 0.0, 0.0)).with_rotation(
                Quat::from_rotation_x(FRAC_PI_2) * Quat::from_rotation_y(RATIO.atan()),
            ),
            Visibility::Visible,
        ))
        .with_children(|p| {
            p.spawn((
                NavMeshSettings {
                    fixed: Triangulation::from_outer_edges(&[
                        vec2(0.0, 0.0),
                        vec2(MESH_UNIT as f32, 0.0),
                        vec2(MESH_UNIT as f32, MESH_UNIT as f32 * 2.0),
                        vec2(0.0, MESH_UNIT as f32 * 2.0),
                    ]),
                    simplify: 0.001,
                    merge_steps: 3,
                    upward_shift: UP_SHIFT,
                    layer: Some(1),
                    scale: vec2(length / MESH_UNIT as f32, 1.0),
                    stitches: vec![
                        (
                            (0, 1),
                            [
                                vec2(MESH_UNIT as f32, 0.0),
                                vec2(MESH_UNIT as f32, MESH_UNIT as f32 * 2.0),
                            ],
                        ),
                        (
                            (1, 2),
                            [
                                vec2(MESH_UNIT as f32 * 2.0, 0.0),
                                vec2(MESH_UNIT as f32 * 2.0, MESH_UNIT as f32 * 2.0),
                            ],
                        ),
                    ],
                    ..default()
                },
                UPDATE_MODE,
                NavMeshDebug(palettes::tailwind::BLUE_600.into()),
            ));
            p.spawn((
                Mesh3d(meshes.add(Plane3d::new(
                    -Vec3::Z,
                    Vec2::new(length / 2.0, MESH_UNIT as f32),
                ))),
                MeshMaterial3d(materials.add(StandardMaterial::from(Color::Srgba(
                    palettes::tailwind::SLATE_900,
                )))),
                Transform::from_xyz(length / 2.0, MESH_UNIT as f32, 0.0),
                RigidBody::Static,
                Collider::cuboid(length, MESH_UNIT as f32 * 2.0, 0.01),
                Restitution::ZERO,
            ));
        });

    // side B
    commands
        .spawn((
            Transform::from_translation(vec3(
                MESH_UNIT as f32 * 2.0,
                MESH_UNIT as f32 * RATIO,
                0.0,
            ))
            .with_rotation(Quat::from_rotation_x(FRAC_PI_2)),
            Visibility::Visible,
        ))
        .with_children(|p| {
            p.spawn((
                NavMeshSettings {
                    fixed: Triangulation::from_outer_edges(&[
                        vec2(0.0, 0.0),
                        vec2(MESH_UNIT as f32, 0.0),
                        vec2(MESH_UNIT as f32, MESH_UNIT as f32 * 2.0),
                        vec2(0.0, MESH_UNIT as f32 * 2.0),
                    ]),
                    simplify: 0.001,
                    merge_steps: 3,
                    upward_shift: UP_SHIFT,
                    layer: Some(2),
                    stitches: vec![
                        (
                            (0, 1),
                            [
                                vec2(MESH_UNIT as f32, 0.0),
                                vec2(MESH_UNIT as f32, MESH_UNIT as f32 * 2.0),
                            ],
                        ),
                        (
                            (1, 2),
                            [
                                vec2(MESH_UNIT as f32 * 2.0, 0.0),
                                vec2(MESH_UNIT as f32 * 2.0, MESH_UNIT as f32 * 2.0),
                            ],
                        ),
                    ],
                    ..default()
                },
                UPDATE_MODE,
                NavMeshDebug(palettes::tailwind::LIME_600.into()),
            ));
            p.spawn((
                Mesh3d(meshes.add(Plane3d::new(
                    -Vec3::Z,
                    Vec2::new(MESH_UNIT as f32 / 2.0, MESH_UNIT as f32),
                ))),
                MeshMaterial3d(materials.add(StandardMaterial::from(Color::Srgba(
                    palettes::tailwind::SLATE_900,
                )))),
                Transform::from_xyz(MESH_UNIT as f32 / 2.0, MESH_UNIT as f32, 0.0),
                RigidBody::Static,
                Collider::cuboid(MESH_UNIT as f32, MESH_UNIT as f32 * 2.0, 0.01),
                Restitution::ZERO,
            ));
        });

    commands.spawn((
        Mesh3d(meshes.add(Capsule3d::new(2.0, 2.0).mesh())),
        MeshMaterial3d(materials.add(StandardMaterial::from(Color::Srgba(
            palettes::tailwind::RED_600,
        )))),
        Transform::from_translation(vec3(MESH_UNIT as f32 / 10.0, 0.0, MESH_UNIT as f32 / 10.0)),
        RigidBody::Static,
        Collider::capsule(2.0, 2.0),
    ));
    commands.spawn((
        Mesh3d(meshes.add(Capsule3d::new(2.0, 2.0).mesh())),
        MeshMaterial3d(materials.add(StandardMaterial::from(Color::Srgba(
            palettes::tailwind::RED_600,
        )))),
        Transform::from_translation(vec3(
            MESH_UNIT as f32 * 3.0 - MESH_UNIT as f32 / 10.0,
            MESH_UNIT as f32 * RATIO,
            MESH_UNIT as f32 * 2.0 - MESH_UNIT as f32 / 10.0,
        )),
        RigidBody::Static,
        Collider::capsule(2.0, 2.0),
    ));
    commands.spawn((
        Mesh3d(meshes.add(Capsule3d::new(2.0, 2.0).mesh())),
        MeshMaterial3d(materials.add(StandardMaterial::from(Color::Srgba(
            palettes::tailwind::RED_600,
        )))),
        Transform::from_translation(vec3(
            MESH_UNIT as f32 / 10.0,
            0.0,
            MESH_UNIT as f32 * 2.0 - MESH_UNIT as f32 / 10.0,
        )),
        RigidBody::Static,
        Collider::capsule(2.0, 2.0),
    ));
    commands.spawn((
        Mesh3d(meshes.add(Capsule3d::new(2.0, 2.0).mesh())),
        MeshMaterial3d(materials.add(StandardMaterial::from(Color::Srgba(
            palettes::tailwind::RED_600,
        )))),
        Transform::from_translation(vec3(
            MESH_UNIT as f32 * 3.0 - MESH_UNIT as f32 / 10.0,
            MESH_UNIT as f32 * RATIO,
            MESH_UNIT as f32 / 10.0,
        )),
        RigidBody::Static,
        Collider::capsule(2.0, 2.0),
    ));
}

fn spawn_obstacles(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    navmeshes: Res<Assets<NavMesh>>,
) {
    let cube_size = rand::thread_rng().gen_range(5.0..10.0);
    loop {
        let x = rand::thread_rng().gen_range(0.0..(MESH_UNIT as f32 * 3.0));
        let z = rand::thread_rng().gen_range(0.0..(MESH_UNIT as f32 * 3.0));

        if navmeshes.iter().any(|(_, nm)| nm.is_in_mesh(vec2(x, z))) {
            commands.spawn((
                Mesh3d(meshes.add(Cuboid::new(cube_size, cube_size, cube_size))),
                MeshMaterial3d(materials.add(Color::srgb(0.2, 0.7, 0.9))),
                Transform::from_xyz(x, 125.0, z).looking_to(
                    Vec3::new(
                        rand::thread_rng().gen_range(-1.0..1.0),
                        rand::thread_rng().gen_range(-1.0..1.0),
                        rand::thread_rng().gen_range(-1.0..1.0),
                    )
                    .normalize(),
                    Vec3::Y,
                ),
                RigidBody::Dynamic,
                Collider::cuboid(cube_size, cube_size, cube_size),
                Restitution::ZERO,
                Obstacle(Timer::from_seconds(4.0, TimerMode::Once)),
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

fn display_path(
    navmeshes: Res<Assets<NavMesh>>,
    mut gizmos: Gizmos,
    statuses: Query<&NavMeshStatus>,
) {
    if statuses
        .iter()
        .any(|status| *status == NavMeshStatus::Invalid)
    {
        return;
    }
    let Some(navmesh) = navmeshes.get(weak_handle!("B7B8516E-6027-44BD-A2AB-BDB8D1320033").id())
    else {
        return;
    };
    for points in [
        (
            vec3(MESH_UNIT as f32 / 10.0, 0.0, MESH_UNIT as f32 / 10.0),
            vec3(
                MESH_UNIT as f32 * 3.0 - MESH_UNIT as f32 / 10.0,
                0.0,
                MESH_UNIT as f32 * 2.0 - MESH_UNIT as f32 / 10.0,
            ),
        ),
        (
            vec3(
                MESH_UNIT as f32 * 3.0 - MESH_UNIT as f32 / 10.0,
                0.0,
                MESH_UNIT as f32 / 10.0,
            ),
            vec3(
                MESH_UNIT as f32 / 10.0,
                0.0,
                MESH_UNIT as f32 * 2.0 - MESH_UNIT as f32 / 10.0,
            ),
        ),
    ] {
        let Some(path) = navmesh.transformed_path(points.0, points.1) else {
            continue;
        };
        let start = navmesh.get().get_point_layer(points.0.xz())[0];

        let mut path = path
            .path_with_layers
            .iter()
            .map(|(v, l)| vec3(v.x, point_to_height(v.xz(), *l), v.z))
            .collect::<Vec<_>>();
        path.insert(
            0,
            vec3(
                points.0.x,
                point_to_height(points.0.xz(), start.layer().unwrap()),
                points.0.z,
            ),
        );
        gizmos.linestrip(path, palettes::tailwind::RED_600);
    }
}

fn point_to_height(point: Vec2, layer: u8) -> f32 {
    let top = MESH_UNIT as f32 * RATIO;
    match layer {
        0 => 0.5,
        1 => (point.x - MESH_UNIT as f32) / MESH_UNIT as f32 * top + 0.5,
        2 => top + 0.5,
        x => unreachable!("layer {:?}", x),
    }
}
