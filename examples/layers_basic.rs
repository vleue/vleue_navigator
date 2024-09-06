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
            PhysicsPlugins::default().with_length_unit(2.0),
            VleueNavigatorPlugin,
            NavmeshUpdaterPlugin::<Collider, Obstacle>::default(),
        ))
        .insert_resource(Gravity(Vector::NEG_Y * 9.81 * 10.0))
        .add_systems(Startup, setup)
        .add_systems(Update, despawn_obstacles)
        .add_systems(PostUpdate, display_path)
        .add_systems(
            Update,
            spawn_obstacles.run_if(on_timer(Duration::from_secs_f32(0.5))),
        );

    let mut config_store = app
        .world_mut()
        .get_resource_mut::<GizmoConfigStore>()
        .unwrap();
    for (_, config, _) in config_store.iter_mut() {
        config.depth_bias = -1.0;
    }

    app.run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.0, MESH_UNIT as f32 * 2.2, MESH_UNIT as f32).looking_at(
            vec3(MESH_UNIT as f32 * 0.85, 0.0, MESH_UNIT as f32),
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

    // side A
    commands
        .spawn(SpatialBundle::from_transform(Transform::from_rotation(
            Quat::from_rotation_x(FRAC_PI_2),
        )))
        .with_children(|p| {
            p.spawn((
                NavMeshBundle {
                    settings: NavMeshSettings {
                        fixed: Triangulation::from_outer_edges(&[
                            vec2(0.0, 0.0),
                            vec2(MESH_UNIT as f32, 0.0),
                            vec2(MESH_UNIT as f32, MESH_UNIT as f32 * 2.0),
                            vec2(0.0, MESH_UNIT as f32 * 2.0),
                        ]),
                        simplify: 0.001,
                        merge_steps: 3,
                        upward_shift: 1.0,
                        layer: Some(0),
                        stitches: vec![(
                            (0, 1),
                            [
                                vec2(MESH_UNIT as f32, 0.0),
                                vec2(MESH_UNIT as f32, MESH_UNIT as f32 * 2.0),
                            ],
                        )],
                        ..default()
                    },
                    update_mode: NavMeshUpdateMode::Direct,
                    ..NavMeshBundle::with_unique_id(0)
                },
                NavMeshDebug(palettes::tailwind::YELLOW_600.into()),
            ));
            p.spawn((
                PbrBundle {
                    mesh: meshes.add(Plane3d::new(
                        -Vec3::Z,
                        Vec2::new(MESH_UNIT as f32 / 2.0, MESH_UNIT as f32),
                    )),
                    material: materials.add(StandardMaterial::from(Color::Srgba(
                        palettes::tailwind::SLATE_900,
                    ))),
                    transform: Transform::from_xyz(MESH_UNIT as f32 / 2.0, MESH_UNIT as f32, 0.0),
                    ..default()
                },
                RigidBody::Static,
                Collider::cuboid(MESH_UNIT as f32, MESH_UNIT as f32 * 2.0, 0.01),
                Restitution::ZERO,
            ));
        });

    // side B
    commands
        .spawn(SpatialBundle::from_transform(
            Transform::from_translation(vec3(MESH_UNIT as f32, 0.0, 0.0))
                .with_rotation(Quat::from_rotation_x(FRAC_PI_2)),
        ))
        .with_children(|p| {
            p.spawn((
                NavMeshBundle {
                    settings: NavMeshSettings {
                        fixed: Triangulation::from_outer_edges(&[
                            vec2(0.0, 0.0),
                            vec2(MESH_UNIT as f32, 0.0),
                            vec2(MESH_UNIT as f32, MESH_UNIT as f32 * 2.0),
                            vec2(0.0, MESH_UNIT as f32 * 2.0),
                        ]),
                        simplify: 0.001,
                        merge_steps: 3,
                        upward_shift: 1.0,
                        layer: Some(1),
                        stitches: vec![(
                            (0, 1),
                            [
                                vec2(MESH_UNIT as f32, 0.0),
                                vec2(MESH_UNIT as f32, MESH_UNIT as f32 * 2.0),
                            ],
                        )],
                        ..default()
                    },
                    update_mode: NavMeshUpdateMode::Direct,
                    ..NavMeshBundle::with_unique_id(0)
                },
                NavMeshDebug(palettes::tailwind::LIME_600.into()),
            ));
            p.spawn((
                PbrBundle {
                    mesh: meshes.add(Plane3d::new(
                        -Vec3::Z,
                        Vec2::new(MESH_UNIT as f32 / 2.0, MESH_UNIT as f32),
                    )),
                    material: materials.add(StandardMaterial::from(Color::Srgba(
                        palettes::tailwind::SLATE_900,
                    ))),
                    transform: Transform::from_xyz(MESH_UNIT as f32 / 2.0, MESH_UNIT as f32, 0.0),
                    ..default()
                },
                RigidBody::Static,
                Collider::cuboid(MESH_UNIT as f32, MESH_UNIT as f32 * 2.0, 0.01),
                Restitution::ZERO,
            ));
        });

    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Capsule3d::new(2.0, 2.0).mesh()),
            material: materials.add(StandardMaterial::from(Color::Srgba(
                palettes::tailwind::RED_600,
            ))),
            transform: Transform::from_translation(vec3(
                MESH_UNIT as f32 / 10.0,
                0.0,
                MESH_UNIT as f32 / 10.0,
            )),
            ..default()
        },
        RigidBody::Static,
        Collider::capsule(2.0, 2.0),
    ));
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Capsule3d::new(2.0, 2.0).mesh()),
            material: materials.add(StandardMaterial::from(Color::Srgba(
                palettes::tailwind::RED_600,
            ))),
            transform: Transform::from_translation(vec3(
                MESH_UNIT as f32 * 2.0 - MESH_UNIT as f32 / 10.0,
                0.0,
                MESH_UNIT as f32 * 2.0 - MESH_UNIT as f32 / 10.0,
            )),
            ..default()
        },
        RigidBody::Static,
        Collider::capsule(2.0, 2.0),
    ));
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Capsule3d::new(2.0, 2.0).mesh()),
            material: materials.add(StandardMaterial::from(Color::Srgba(
                palettes::tailwind::RED_600,
            ))),
            transform: Transform::from_translation(vec3(
                MESH_UNIT as f32 / 10.0,
                0.0,
                MESH_UNIT as f32 * 2.0 - MESH_UNIT as f32 / 10.0,
            )),
            ..default()
        },
        RigidBody::Static,
        Collider::capsule(2.0, 2.0),
    ));
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Capsule3d::new(2.0, 2.0).mesh()),
            material: materials.add(StandardMaterial::from(Color::Srgba(
                palettes::tailwind::RED_600,
            ))),
            transform: Transform::from_translation(vec3(
                MESH_UNIT as f32 * 2.0 - MESH_UNIT as f32 / 10.0,
                0.0,
                MESH_UNIT as f32 / 10.0,
            )),
            ..default()
        },
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
        let x = rand::thread_rng().gen_range(-(MESH_UNIT as f32 * 3.0)..(MESH_UNIT as f32 * 3.0));
        let z = rand::thread_rng().gen_range(-(MESH_UNIT as f32 * 3.0)..(MESH_UNIT as f32 * 3.0));
        if navmeshes.iter().any(|(_, nm)| nm.is_in_mesh(vec2(x, z))) {
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
                Restitution::ZERO,
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

fn display_path(navmeshes: Res<Assets<NavMesh>>, mut gizmos: Gizmos) {
    let Some(navmesh) = navmeshes.get(Handle::<NavMesh>::weak_from_u128(0).id()) else {
        return;
    };
    for points in [
        (
            vec2(MESH_UNIT as f32 / 10.0, MESH_UNIT as f32 / 10.0),
            vec2(
                MESH_UNIT as f32 * 2.0 - MESH_UNIT as f32 / 10.0,
                MESH_UNIT as f32 * 2.0 - MESH_UNIT as f32 / 10.0,
            ),
        ),
        (
            vec2(
                MESH_UNIT as f32 * 2.0 - MESH_UNIT as f32 / 10.0,
                MESH_UNIT as f32 / 10.0,
            ),
            vec2(
                MESH_UNIT as f32 / 10.0,
                MESH_UNIT as f32 * 2.0 - MESH_UNIT as f32 / 10.0,
            ),
        ),
    ] {
        let Some(path) = navmesh.path(points.0, points.1) else {
            continue;
        };
        let mut path = path
            .path
            .iter()
            .map(|v| vec3(v.x, 0.5, v.y))
            .collect::<Vec<_>>();
        path.insert(0, vec3(points.0.x, 0.5, points.0.y));
        gizmos.linestrip(path, palettes::tailwind::RED_600);
    }
}
