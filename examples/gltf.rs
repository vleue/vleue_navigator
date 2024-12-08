use bevy::{
    asset::LoadState,
    color::palettes,
    gltf::{Gltf, GltfMesh},
    math::Vec3Swizzles,
    pbr::NotShadowCaster,
    prelude::*,
    window::PrimaryWindow,
};
use rand::Rng;
use std::f32::consts::FRAC_PI_2;
use vleue_navigator::{NavMesh, VleueNavigatorPlugin};

const HANDLE_TRIMESH_OPTIMIZED: Handle<NavMesh> = Handle::weak_from_u128(0);

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::srgb(0., 0., 0.01)))
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Navmesh with Polyanya".to_string(),
                    fit_canvas_to_parent: true,
                    ..default()
                }),
                ..default()
            }),
            VleueNavigatorPlugin,
        ))
        .init_state::<AppState>()
        .add_systems(OnEnter(AppState::Setup), setup)
        .add_systems(Update, check_textures.run_if(in_state(AppState::Setup)))
        .add_systems(OnExit(AppState::Setup), setup_scene)
        .add_systems(
            Update,
            (
                give_target_auto,
                give_target_on_click,
                move_object,
                move_hover,
                target_activity,
                trigger_navmesh_visibility,
            )
                .run_if(in_state(AppState::Playing)),
        )
        .run();
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, States, Default)]
enum AppState {
    #[default]
    Setup,
    Playing,
}

#[derive(Resource, Default, Deref)]
struct GltfHandle(Handle<Gltf>);

#[derive(Resource)]
struct CurrentMesh(Handle<NavMesh>);

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(GltfHandle(asset_server.load("meshes/navmesh.glb")));

    commands.insert_resource(AmbientLight {
        color: palettes::css::SEA_GREEN.into(),
        brightness: 100.0,
    });

    commands.spawn((
        Camera3d::default(),
        Camera {
            #[cfg(not(target_arch = "wasm32"))]
            hdr: true,
            ..default()
        },
        Transform::from_xyz(0.0, 70.0, 5.0).looking_at(Vec3::new(0.0, 0.3, 0.0), Vec3::Y),
    ));

    commands
        .spawn((
            Text::default(),
            Node {
                align_self: AlignSelf::FlexStart,
                margin: UiRect::all(Val::Px(15.0)),
                ..Default::default()
            },
        ))
        .with_children(|p| {
            let font_size = TextFont {
                font_size: 20.0,
                ..default()
            };
            p.spawn((
                TextSpan::new("<span>".to_string()),
                TextColor(palettes::css::GOLD.into()),
                font_size.clone(),
            ));
            p.spawn((
                TextSpan::new(" to display the navmesh, ".to_string()),
                TextColor(palettes::css::WHITE.into()),
                font_size.clone(),
            ));
            p.spawn((
                TextSpan::new("click".to_string()),
                TextColor(palettes::css::GOLD.into()),
                font_size.clone(),
            ));
            p.spawn((
                TextSpan::new(" to set the destination".to_string()),
                TextColor(palettes::css::WHITE.into()),
                font_size,
            ));
        });

    commands.insert_resource(CurrentMesh(HANDLE_TRIMESH_OPTIMIZED));
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

#[derive(Component, Clone)]
struct NavMeshDisp(Handle<NavMesh>);

fn setup_scene(
    mut commands: Commands,
    gltf: Res<GltfHandle>,
    gltfs: Res<Assets<Gltf>>,
    gltf_meshes: Res<Assets<GltfMesh>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut navmeshes: ResMut<Assets<NavMesh>>,
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
        ));

        let mesh = gltf_meshes.get(&gltf.named_meshes["plane"]).unwrap();
        commands.spawn((
            Mesh3d(mesh.primitives[0].mesh.clone()),
            MeshMaterial3d(ground_material.clone()),
            Transform::from_xyz(0.0, 0.1, 0.0),
        ));
    }

    {
        #[cfg(target_arch = "wasm32")]
        const NB_HOVER: usize = 5;
        #[cfg(not(target_arch = "wasm32"))]
        const NB_HOVER: usize = 10;

        for _i in 0..NB_HOVER {
            commands.spawn((
                SpotLight {
                    intensity: 1000000.0,
                    color: palettes::css::SEA_GREEN.into(),
                    shadows_enabled: true,
                    inner_angle: 0.5,
                    outer_angle: 0.8,
                    range: 250.0,
                    ..default()
                },
                Transform::from_xyz(
                    rand::thread_rng().gen_range(-50.0..50.0),
                    20.0,
                    rand::thread_rng().gen_range(-25.0..25.0),
                )
                .with_rotation(Quat::from_rotation_x(FRAC_PI_2)),
                Hover(Vec2::new(
                    rand::thread_rng().gen_range(-50.0..50.0),
                    rand::thread_rng().gen_range(-25.0..25.0),
                )),
            ));
        }
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
            );

            let mut material: StandardMaterial = Color::Srgba(palettes::css::ANTIQUE_WHITE).into();
            material.unlit = true;

            commands.spawn((
                Mesh3d(meshes.add(navmesh.to_wireframe_mesh())),
                MeshMaterial3d(materials.add(material)),
                Transform::from_xyz(0.0, 0.2, 0.0),
                Visibility::Hidden,
                NavMeshDisp(HANDLE_TRIMESH_OPTIMIZED),
            ));
            navmeshes.insert(&HANDLE_TRIMESH_OPTIMIZED, navmesh);
        }

        commands
            .spawn((
                Mesh3d(meshes.add(Mesh::from(Capsule3d { ..default() }))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: palettes::css::BLUE.into(),
                    emissive: (palettes::css::BLUE * 5.0).into(),
                    ..default()
                })),
                Transform::from_xyz(0.0, 0.0, 0.0),
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
    current_mesh: Res<CurrentMesh>,
) {
    for (entity, transform, mut object) in object_query.iter_mut() {
        let navmesh = navmeshes.get(&current_mesh.0).unwrap();
        let mut x;
        let mut z;
        loop {
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

fn give_target_on_click(
    mut commands: Commands,
    mut object_query: Query<(Entity, &Transform, &mut Object)>,
    targets: Query<Entity, With<Target>>,
    navmeshes: Res<Assets<NavMesh>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    current_mesh: Res<CurrentMesh>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    primary_window: Query<&Window, With<PrimaryWindow>>,
    camera: Query<(&Camera, &GlobalTransform)>,
) {
    if mouse_buttons.just_pressed(MouseButton::Left) {
        let navmesh = navmeshes.get(&current_mesh.0).unwrap();
        let Some(target) = (|| {
            let position = primary_window.single().cursor_position()?;
            let (camera, transform) = camera.get_single().ok()?;
            let ray = camera.viewport_to_world(transform, position).ok()?;
            let denom = Vec3::Y.dot(ray.direction.into());
            let t = (Vec3::ZERO - ray.origin).dot(Vec3::Y) / denom;
            let target = ray.origin + ray.direction * t;
            navmesh.transformed_is_in_mesh(target).then_some(target)
        })() else {
            return;
        };

        for (entity, transform, mut object) in object_query.iter_mut() {
            let Some(path) = navmesh.transformed_path(transform.translation, target) else {
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
                        Transform::from_translation(target),
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
        for entity in &targets {
            commands.entity(entity).despawn_recursive();
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
        transform.translation += move_direction.normalize() * time.delta_secs() * 10.0;
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

fn trigger_navmesh_visibility(
    mut query: Query<(&mut Visibility, &NavMeshDisp)>,
    keyboard_input: ResMut<ButtonInput<KeyCode>>,
    current_mesh: Res<CurrentMesh>,
) {
    if keyboard_input.just_pressed(KeyCode::Space) {
        for (mut visible, nav) in query.iter_mut() {
            if nav.0 == current_mesh.0 {
                match *visible {
                    Visibility::Visible => *visible = Visibility::Hidden,
                    Visibility::Hidden => *visible = Visibility::Visible,
                    Visibility::Inherited => *visible = Visibility::Inherited,
                }
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
