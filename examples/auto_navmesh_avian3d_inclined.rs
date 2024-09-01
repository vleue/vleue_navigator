use std::{f32::consts::FRAC_PI_8, time::Duration};

use avian3d::{math::*, prelude::*};
use bevy::{color::palettes, math::vec2, prelude::*, time::common_conditions::on_timer};

use rand::Rng;
use vleue_navigator::prelude::*;

#[derive(Component)]
struct Obstacle;

fn main() {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Navmesh with Polyanya".to_string(),
                fit_canvas_to_parent: true,
                ..default()
            }),
            ..default()
        }),
        PhysicsPlugins::default().with_length_unit(20.0),
        VleueNavigatorPlugin,
        NavmeshUpdaterPlugin::<Collider, Obstacle>::default(),
    ))
    .insert_resource(ClearColor(Color::srgb(0.05, 0.05, 0.1)))
    .insert_resource(Gravity(Vector::NEG_Y * 9.81 * 0.8))
    .add_systems(Startup, setup)
    .add_systems(Update, (despawn, rotate_camera))
    .add_systems(
        Update,
        spawn_obstacles.run_if(on_timer(Duration::from_secs_f32(2.0))),
    );

    // let mut config_store = app
    //     .world_mut()
    //     .get_resource_mut::<GizmoConfigStore>()
    //     .unwrap();
    // for (_, config, _) in config_store.iter_mut() {
    //     config.depth_bias = -1.0;
    // }

    app.run();
}

pub const MATERIAL_OBSTACLE_LIVE: Handle<StandardMaterial> = Handle::weak_from_u128(0);
const ANGLE: f32 = FRAC_PI_8;

fn setup(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    materials.insert(&MATERIAL_OBSTACLE_LIVE, Color::srgb(0.2, 0.7, 0.9).into());

    let arena_mesh = meshes.add(Cuboid::default());
    let arena_material = materials.add(Color::srgb(0.7, 0.7, 0.8));

    // Ground
    commands.spawn((
        PbrBundle {
            mesh: arena_mesh.clone(),
            material: arena_material.clone(),
            transform: Transform::from_xyz(0.0, 0.0, 5.0)
                .with_scale(Vec3::new(50.0, 0.1, 50.0))
                .with_rotation(Quat::from_rotation_x(ANGLE)),
            ..default()
        },
        RigidBody::Static,
        Collider::cuboid(1.0, 1.0, 1.0),
    ));

    // Directional light
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: 5000.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::default().looking_at(Vec3::new(-1.0, -2.5, -1.5), Vec3::Y),
        ..default()
    });

    // Camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_translation(Vec3::new(0.0, 40.0, 30.0))
            .looking_at(Vec3::Z * 5.0, Vec3::Y),
        ..default()
    });

    commands.spawn((
        NavMeshBundle {
            settings: NavMeshSettings {
                // Define the outer borders of the navmesh.
                fixed: Triangulation::from_outer_edges(&[
                    vec2(-25.0, -25.0),
                    vec2(25.0, -25.0),
                    vec2(25.0, 25.0),
                    vec2(-25.0, 25.0),
                ]),
                build_timeout: Some(1.0),
                simplify: 0.005,
                merge_steps: 0,
                upward_shift: 1.0,
                // up: Some((Dir3::new(vec3(0.0, ANGLE.cos(), ANGLE.sin())).unwrap(), 1.0)),
                ..default()
            },
            update_mode: NavMeshUpdateMode::Direct,
            transform: Transform::from_xyz(0.0, 0.1, 5.0)
                .with_rotation(Quat::from_rotation_x(ANGLE) * Quat::from_rotation_x(FRAC_PI_2)),
            handle: Handle::<NavMesh>::weak_from_u128(0),

            ..default()
        },
        NavMeshDebug(palettes::tailwind::RED_600.into()),
    ));
}

fn despawn(mut commands: Commands, query: Query<(Entity, &Transform)>) {
    for (entity, transform) in &query {
        if transform.translation.y < -50.0 {
            commands.entity(entity).despawn();
        }
    }
}

fn spawn_obstacles(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let cube_size = 2.0;
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Cuboid::new(cube_size, cube_size, cube_size)),
            material: materials.add(Color::srgb(0.2, 0.7, 0.9)),
            transform: Transform::from_xyz(
                rand::thread_rng().gen_range(-25.0..25.0),
                50.0,
                // 10.0 * ANGLE.signum(),
                rand::thread_rng().gen_range(-25.0..-10.0),
            )
            .looking_to(
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
        Obstacle,
    ));
}

fn rotate_camera(time: Res<Time>, mut query: Query<&mut Transform, With<Camera3d>>) {
    for mut transform in query.iter_mut() {
        transform.rotate_around(
            Vec3::Z * 5.0,
            Quat::from_rotation_y(time.delta_seconds() / 10.0),
        )
    }
}
