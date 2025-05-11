use avian3d::prelude::*;
use bevy::{
    asset::LoadState,
    color::palettes,
    gltf::{Gltf, GltfMesh},
    math::Vec3Swizzles,
    pbr::NotShadowCaster,
    prelude::*,
    time::common_conditions::on_timer,
};
use polyanya::Triangulation;
use rand::Rng;
use std::{f32::consts::FRAC_PI_2, time::Duration};
use vleue_navigator::{
    NavMesh, NavMeshDebug, VleueNavigatorPlugin,
    prelude::{ManagedNavMesh, NavMeshSettings, NavMeshUpdateMode, NavmeshUpdaterPlugin},
};

#[derive(Component)]
struct Obstacle(Timer);

#[derive(Default, Reflect, GizmoConfigGroup)]
struct PathGizmo {}

fn main() {
    let mut app = App::new();
    app.insert_resource(ClearColor(Color::srgb(0., 0., 0.01)))
        .init_gizmo_group::<PathGizmo>()
        .add_plugins((
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
        .init_state::<AppState>()
        .add_systems(OnEnter(AppState::Setup), setup)
        .add_systems(Update, check_textures.run_if(in_state(AppState::Setup)))
        .add_systems(OnExit(AppState::Setup), setup_scene)
        .add_systems(
            Update,
            (
                give_target_auto,
                move_object,
                move_hover,
                target_activity,
                display_navigator_path,
                despawn_obstacles,
            )
                .run_if(in_state(AppState::Playing)),
        )
        .add_systems(
            Update,
            spawn_obstacles.run_if(on_timer(Duration::from_secs_f32(0.5))),
        )
        .add_systems(
            Update,
            refresh_path.run_if(on_timer(Duration::from_secs_f32(0.1))),
        );

    let mut config_store = app
        .world_mut()
        .get_resource_mut::<GizmoConfigStore>()
        .unwrap();
    for (_, config, _) in config_store.iter_mut() {
        config.depth_bias = -1.0;
    }
    let (config, _) = config_store.config_mut::<PathGizmo>();
    config.line.width = 10.0;
    config.line.joints = GizmoLineJoint::Bevel;

    app.run();
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, States, Default)]
enum AppState {
    #[default]
    Setup,
    Playing,
}

#[derive(Resource, Default, Deref)]
struct GltfHandle(Handle<Gltf>);

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(GltfHandle(asset_server.load("meshes/navmesh.glb")));

    commands.spawn((
        DirectionalLight {
            illuminance: 3000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::default().looking_at(Vec3::new(-1.0, -2.5, -1.5), Vec3::Y),
    ));

    commands.spawn((
        Camera3d::default(),
        Camera {
            #[cfg(not(target_arch = "wasm32"))]
            hdr: true,
            ..default()
        },
        Transform::from_xyz(0.0, 70.0, 5.0).looking_at(Vec3::new(0.0, 0.3, 0.0), Vec3::Y),
    ));
}

fn check_textures(
    mut next_state: ResMut<NextState<AppState>>,
    gltf: ResMut<GltfHandle>,
    asset_server: Res<AssetServer>,
) {
    if let Some(LoadState::Loaded) = asset_server.get_load_state(gltf.id()) {
        next_state.set(AppState::Playing);
    }
}

#[derive(Component)]
struct Path {
    current: Vec3,
    next: Vec<Vec3>,
}

#[derive(Component)]
struct Object(Option<Entity>);

#[derive(Component)]
struct Target;

#[derive(Component)]
struct Hover(Vec2);

fn setup_scene(
    mut commands: Commands,
    gltf: Res<GltfHandle>,
    gltfs: Res<Assets<Gltf>>,
    gltf_meshes: Res<Assets<GltfMesh>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut material: StandardMaterial = Color::Srgba(palettes::css::ALICE_BLUE).into();
    material.perceptual_roughness = 1.0;
    let ground_material = materials.add(material);
    if let Some(gltf) = gltfs.get(gltf.id()) {
        let mesh = gltf_meshes.get(&gltf.named_meshes["obstacles"]).unwrap();
        let mut material: StandardMaterial = Color::Srgba(palettes::css::GRAY).into();
        material.perceptual_roughness = 1.0;
        commands.spawn((
            Mesh3d(mesh.primitives[0].mesh.clone()),
            MeshMaterial3d(materials.add(material)),
            RigidBody::Static,
            ColliderConstructor::TrimeshFromMesh,
        ));

        let mesh = gltf_meshes.get(&gltf.named_meshes["plane"]).unwrap();
        commands.spawn((
            Mesh3d(mesh.primitives[0].mesh.clone()),
            MeshMaterial3d(ground_material.clone()),
            Transform::from_xyz(0.0, 0.01, 0.0),
        ));
    }

    if let Some(gltf) = gltfs.get(gltf.id()) {
        {
            let navmesh = vleue_navigator::NavMesh::from_bevy_mesh(
                meshes
                    .get(
                        &gltf_meshes
                            .get(&gltf.named_meshes["navmesh"])
                            .unwrap()
                            .primitives[0]
                            .mesh,
                    )
                    .unwrap(),
            )
            .unwrap();

            let mut material: StandardMaterial = Color::Srgba(palettes::css::ANTIQUE_WHITE).into();
            material.unlit = true;

            commands.spawn((
                NavMeshSettings {
                    fixed: Triangulation::from_mesh(navmesh.get().as_ref(), 0),
                    build_timeout: Some(5.0),
                    upward_shift: 0.5,
                    merge_steps: 2,
                    ..default()
                },
                Transform::from_rotation(Quat::from_rotation_x(FRAC_PI_2)),
                NavMeshUpdateMode::Direct,
                NavMeshDebug(palettes::tailwind::RED_400.into()),
            ));
        }

        commands
            .spawn((
                Mesh3d(meshes.add(Mesh::from(Capsule3d { ..default() }))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: palettes::css::BLUE.into(),
                    emissive: (palettes::css::BLUE * 5.0).into(),
                    ..default()
                })),
                Transform::from_xyz(-1.0, 0.0, -2.0),
                Object(None),
                NotShadowCaster,
            ))
            .with_children(|object| {
                object.spawn((
                    PointLight {
                        color: palettes::css::BLUE.into(),
                        range: 500.0,
                        intensity: 100000.0,
                        shadows_enabled: true,
                        ..default()
                    },
                    Transform::from_xyz(0.0, 1.2, 0.0),
                ));
            });
    }
}

fn give_target_auto(
    mut commands: Commands,
    mut object_query: Query<(Entity, &Transform, &mut Object), Without<Path>>,
    navmeshes: Res<Assets<NavMesh>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    navmesh: Query<&ManagedNavMesh>,
) {
    for (entity, transform, mut object) in object_query.iter_mut() {
        let Some(navmesh) = navmeshes.get(navmesh.single().unwrap()) else {
            continue;
        };
        let mut x = 0.0;
        let mut z = 0.0;
        for _ in 0..50 {
            x = rand::thread_rng().gen_range(-50.0..50.0);
            z = rand::thread_rng().gen_range(-25.0..25.0);

            if navmesh.transformed_is_in_mesh(Vec3::new(x, 0.0, z)) {
                break;
            }
        }

        let Some(path) = navmesh.transformed_path(transform.translation, Vec3::new(x, 0.0, z))
        else {
            break;
        };
        if let Some((first, remaining)) = path.path.split_first() {
            let mut remaining = remaining.to_vec();
            remaining.reverse();
            let target_id = commands
                .spawn((
                    Mesh3d(meshes.add(Mesh::from(Sphere { radius: 0.5 }))),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: palettes::css::RED.into(),
                        emissive: (palettes::css::RED * 5.0).into(),
                        ..default()
                    })),
                    Transform::from_xyz(x, 0.0, z),
                    NotShadowCaster,
                    Target,
                ))
                .with_children(|target| {
                    target.spawn((
                        PointLight {
                            color: palettes::css::RED.into(),
                            shadows_enabled: true,
                            range: 10.0,
                            ..default()
                        },
                        Transform::from_xyz(0.0, 1.5, 0.0),
                    ));
                })
                .id();
            commands.entity(entity).insert(Path {
                current: *first,
                next: remaining,
            });
            object.0 = Some(target_id);
        }
    }
}

fn refresh_path(
    mut object_query: Query<(&Transform, &mut Path)>,
    target: Query<&Transform, With<Target>>,
    navmeshes: Res<Assets<NavMesh>>,
    navmesh: Query<&ManagedNavMesh>,
) {
    for (transform, mut path) in &mut object_query {
        let navmesh = navmeshes.get(navmesh.single().unwrap()).unwrap();
        let Some(new_path) =
            navmesh.transformed_path(transform.translation, target.single().unwrap().translation)
        else {
            break;
        };
        if let Some((first, remaining)) = new_path.path.split_first() {
            let mut remaining = remaining.to_vec();
            remaining.reverse();
            *path = Path {
                current: *first,
                next: remaining,
            };
        }
    }
}

fn move_object(
    mut commands: Commands,
    mut object_query: Query<(&mut Transform, &mut Path, Entity, &mut Object)>,
    time: Res<Time>,
) {
    for (mut transform, mut target, entity, mut object) in object_query.iter_mut() {
        let move_direction = target.current - transform.translation;
        transform.translation += move_direction.normalize() * time.delta_secs() * 6.0;
        if transform.translation.distance(target.current) < 0.1 {
            if let Some(next) = target.next.pop() {
                target.current = next;
            } else {
                commands.entity(entity).remove::<Path>();
                let target_entity = object.0.take().unwrap();
                commands.entity(target_entity).despawn_recursive();
            }
        }
    }
}

fn target_activity(
    target: Query<&Children, With<Target>>,
    mut point_light: Query<&mut PointLight>,
    time: Res<Time>,
) {
    for children in &target {
        point_light.get_mut(children[0]).unwrap().intensity =
            (time.elapsed_secs() * 10.0).sin().abs() * 100000.0;
    }
}

fn move_hover(mut hovers: Query<(&mut Transform, &mut Hover)>, time: Res<Time>) {
    for (mut transform, mut hover) in &mut hovers {
        let current = transform.translation.xz();
        if hover.0.distance_squared(current) < 1.0 {
            hover.0 = Vec2::new(
                rand::thread_rng().gen_range(-50.0..50.0),
                rand::thread_rng().gen_range(-25.0..25.0),
            );
        }
        transform.translation += ((hover.0 - current).normalize() * time.delta_secs() * 5.0)
            .extend(0.0)
            .xzy();
    }
}

fn spawn_obstacles(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let size = rand::thread_rng().gen_range(1.5..2.0);
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(size, size, size))),
        MeshMaterial3d(materials.add(Color::srgb(0.2, 0.7, 0.9))),
        Transform::from_xyz(
            rand::thread_rng().gen_range(-50.0..50.0),
            10.0,
            rand::thread_rng().gen_range(-25.0..25.0),
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
        RigidBody::Dynamic,
        Collider::cuboid(size, size, size),
        Obstacle(Timer::from_seconds(30.0, TimerMode::Once)),
    ));
}

fn display_navigator_path(navigator: Query<(&Transform, &Path)>, mut gizmos: Gizmos<PathGizmo>) {
    for (transform, path) in &navigator {
        let mut to_display = path.next.clone();
        to_display.push(path.current);
        to_display.push(transform.translation.xz().extend(0.2).xzy());
        // to_display.reverse();
        if !to_display.is_empty() {
            gizmos.linestrip(
                to_display.iter().map(|xz| Vec3::new(xz.x, 0.2, xz.z)),
                palettes::tailwind::FUCHSIA_500,
            );
        }
    }
}

fn despawn_obstacles(
    mut commands: Commands,
    mut obstacles: Query<(Entity, &mut Obstacle, &mut LinearVelocity, &mut Transform)>,
    time: Res<Time>,
) {
    for (entity, mut timer, mut linvel, mut transform) in &mut obstacles {
        if timer.0.tick(time.delta()).just_finished() {
            linvel.0 = Vec3::new(0.0, 50.0, 0.0);
        }
        if timer.0.finished() {
            transform.scale *= 0.98;
            if transform.scale.x < 0.01 {
                commands.entity(entity).despawn();
            }
        }
    }
}
