use std::time::Duration;

use avian3d::{math::*, prelude::*};
use bevy::{color::palettes, math::vec2, prelude::*, time::common_conditions::on_timer};

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
    .insert_resource(Gravity(Vector::NEG_Y * 9.81 * 50.0))
    .add_systems(Startup, setup)
    .add_systems(
        Update,
        view_navmesh.run_if(on_timer(Duration::from_secs_f32(1.0))),
    )
    .add_systems(Update, cached_material);

    let mut config_store = app
        .world_mut()
        .get_resource_mut::<GizmoConfigStore>()
        .unwrap();
    for (_, config, _) in config_store.iter_mut() {
        config.depth_bias = -1.0;
    }

    app.run();
}

pub const MATERIAL_OBSTACLE_LIVE: Handle<StandardMaterial> = Handle::weak_from_u128(0);
pub const MATERIAL_OBSTACLE_CACHED: Handle<StandardMaterial> = Handle::weak_from_u128(1);

fn setup(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    materials.insert(&MATERIAL_OBSTACLE_LIVE, Color::srgb(0.2, 0.7, 0.9).into());
    materials.insert(&MATERIAL_OBSTACLE_CACHED, Color::srgb(0.2, 0.9, 0.7).into());
    let arena_mesh = meshes.add(Cuboid::default());
    let arena_material = materials.add(Color::srgb(0.7, 0.7, 0.8));
    // Ground
    commands.spawn((
        Mesh3d(arena_mesh.clone()),
        MeshMaterial3d(arena_material.clone()),
        Transform::from_xyz(0.0, -5.0, 0.0).with_scale(Vec3::new(50.0, 10.0, 50.0)),
        RigidBody::Static,
        Collider::cuboid(1.0, 1.0, 1.0),
    ));
    commands.spawn((
        Mesh3d(arena_mesh.clone()),
        MeshMaterial3d(arena_material.clone()),
        Transform::from_xyz(25.5, 0.0, 0.0).with_scale(Vec3::new(1.0, 10.0, 50.0)),
        RigidBody::Static,
        Collider::cuboid(1.0, 1.0, 1.0),
    ));
    commands.spawn((
        Mesh3d(arena_mesh.clone()),
        MeshMaterial3d(arena_material.clone()),
        Transform::from_xyz(-25.5, 0.0, 0.0).with_scale(Vec3::new(1.0, 10.0, 50.0)),
        RigidBody::Static,
        Collider::cuboid(1.0, 1.0, 1.0),
    ));
    commands.spawn((
        Mesh3d(arena_mesh.clone()),
        MeshMaterial3d(arena_material.clone()),
        Transform::from_xyz(0.0, 0.0, 25.5).with_scale(Vec3::new(50.0, 10.0, 1.0)),
        RigidBody::Static,
        Collider::cuboid(1.0, 1.0, 1.0),
    ));
    commands.spawn((
        Mesh3d(arena_mesh.clone()),
        MeshMaterial3d(arena_material.clone()),
        Transform::from_xyz(0.0, 0.0, -25.5).with_scale(Vec3::new(50.0, 10.0, 1.0)),
        RigidBody::Static,
        Collider::cuboid(1.0, 1.0, 1.0),
    ));

    let obstacle_size = 2.0;
    let spacing = 1.0;

    use rand::seq::SliceRandom;

    let types = [
        (
            meshes.add(Cuboid::from_length(obstacle_size)),
            Collider::cuboid(obstacle_size, obstacle_size, obstacle_size),
        ),
        (
            meshes.add(Cone {
                radius: 2.0,
                height: 2.0,
            }),
            Collider::cone(2.0, 2.0),
        ),
        (
            meshes.add(Sphere::new(obstacle_size / 2.0)),
            Collider::sphere(obstacle_size / 2.0),
        ),
        (
            meshes.add(Capsule3d::new(1.0, 2.0)),
            Collider::capsule(1.0, 2.0),
        ),
    ];
    // Spawn some obstacles
    for x in -3..3 {
        for z in -3..3 {
            let (mesh, collider) = types.choose(&mut rand::thread_rng()).unwrap();
            let position = Vec3::new((x as f32 - 0.5) * spacing, 25.0, (z as f32 - 0.5) * spacing)
                * obstacle_size;
            commands.spawn((
                Mesh3d(mesh.clone()),
                MeshMaterial3d(MATERIAL_OBSTACLE_LIVE.clone()),
                Transform::from_translation(position),
                Friction::new(0.1),
                RigidBody::Dynamic,
                collider.clone(),
                Obstacle,
            ));
        }
    }

    // Directional light
    commands.spawn((
        DirectionalLight {
            illuminance: 5000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::default().looking_at(Vec3::new(-1.0, -2.5, -1.5), Vec3::Y),
    ));

    // Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(Vec3::new(0.0, 40.0, 30.0)).looking_at(Vec3::Z * 5.0, Vec3::Y),
    ));

    let nb_navmeshes = 3;
    let height_step = obstacle_size / (nb_navmeshes as f32);
    for idx in 0..nb_navmeshes {
        commands.spawn((
            ManagedNavMesh::from_id(idx as u128),
            NavMeshSettings {
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
                ..default()
            },
            NavMeshUpdateMode::Direct,
            Transform::from_xyz(0.0, idx as f32 * height_step + 0.1, 0.0)
                .with_rotation(Quat::from_rotation_x(FRAC_PI_2)),
        ));
    }
}

fn view_navmesh(
    mut commands: Commands,
    navmeshes: Query<Entity, With<ManagedNavMesh>>,
    mut current: Local<usize>,
) {
    for (i, entity) in navmeshes.iter().sort::<Entity>().enumerate() {
        commands.entity(entity).remove::<NavMeshDebug>();
        if i == *current {
            commands
                .entity(entity)
                .insert(NavMeshDebug(palettes::tailwind::RED_800.into()));
        }
    }
    *current = (*current + 1) % navmeshes.iter().len();
}

fn cached_material(
    mut obstacles: Query<(
        &mut MeshMaterial3d<StandardMaterial>,
        Option<Ref<CachableObstacle>>,
    )>,
    mut removed: RemovedComponents<CachableObstacle>,
) {
    for (mut material, cachable) in &mut obstacles {
        if cachable.is_some() {
            *material = MeshMaterial3d(MATERIAL_OBSTACLE_CACHED.clone());
        }
    }
    for removed in removed.read() {
        let (mut material, _) = obstacles.get_mut(removed).unwrap();
        *material = MeshMaterial3d(MATERIAL_OBSTACLE_LIVE.clone());
    }
}
