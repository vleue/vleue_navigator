use bevy::{
    asset::LoadState,
    gltf::{Gltf, GltfMesh},
    math::Vec3Swizzles,
    pbr::NotShadowCaster,
    prelude::*,
    reflect::TypeUuid,
    render::mesh::VertexAttributeValues,
};
#[cfg(not(target_arch = "wasm32"))]
use bevy::{
    pbr::wireframe::{Wireframe, WireframePlugin},
    render::settings::{WgpuFeatures, WgpuSettings},
};
use bevy_pathmesh::{PathMesh, PathMeshPlugin};
use itertools::Itertools;
use rand::Rng;
use std::f32::consts::FRAC_PI_2;

const HANDLE_TRIMESH_OPTIMIZED: HandleUntyped =
    HandleUntyped::weak_from_u64(PathMesh::TYPE_UUID, 0);

fn main() {
    let mut app = App::new();
    app.insert_resource(Msaa { samples: 4 })
        .insert_resource(ClearColor(Color::rgb(0., 0., 0.01)))
        .init_resource::<GltfHandles>();
    #[cfg(not(target_arch = "wasm32"))]
    app.insert_resource(WgpuSettings {
        features: WgpuFeatures::POLYGON_MODE_LINE,
        ..default()
    });
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        window: WindowDescriptor {
            title: "Navmesh with Polyanya".to_string(),
            fit_canvas_to_parent: true,
            ..default()
        },
        ..default()
    }));
    #[cfg(not(target_arch = "wasm32"))]
    app.add_plugin(WireframePlugin);
    app.add_plugin(PathMeshPlugin)
        .add_state(AppState::Setup)
        .add_system_set(SystemSet::on_enter(AppState::Setup).with_system(setup))
        .add_system_set(SystemSet::on_update(AppState::Setup).with_system(check_textures))
        .add_system_set(SystemSet::on_exit(AppState::Setup).with_system(setup_scene))
        .add_system_set({
            let set = SystemSet::on_update(AppState::Playing)
                .with_system(give_target_auto)
                .with_system(give_target_on_click)
                .with_system(move_object)
                .with_system(move_hover)
                .with_system(target_activity);
            #[cfg(not(target_arch = "wasm32"))]
            let set = set.with_system(trigger_navmesh_visibility);
            #[cfg(target_arch = "wasm32")]
            let set = set;
            set
        })
        .run();
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum AppState {
    Setup,
    Playing,
}

#[derive(Resource, Default)]
struct GltfHandles {
    handle: Handle<Gltf>,
}

#[derive(Resource)]
struct CurrentMesh(Handle<PathMesh>);

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut navmeshes: ResMut<GltfHandles>,
) {
    navmeshes.handle = asset_server.load("meshes/navmesh.glb");

    commands.insert_resource(AmbientLight {
        color: Color::SEA_GREEN,
        brightness: 0.05,
    });

    commands.spawn(Camera3dBundle {
        camera: Camera {
            #[cfg(not(target_arch = "wasm32"))]
            hdr: true,
            ..default()
        },
        transform: Transform::from_xyz(0.0, 70.0, 5.0)
            .looking_at(Vec3::new(0.0, 0.3, 0.0), Vec3::Y),
        ..Default::default()
    });

    commands.spawn(TextBundle {
        style: Style {
            align_self: AlignSelf::FlexStart,
            margin: UiRect::all(Val::Px(15.0)),
            ..Default::default()
        },
        text: Text {
            sections: vec![
                #[cfg(not(target_arch = "wasm32"))]
                TextSection {
                    value: "<space> to display the navmesh, ".to_string(),
                    style: TextStyle {
                        font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                        font_size: 30.0,
                        color: Color::WHITE,
                    },
                },
                TextSection {
                    value: "click to change the destination".to_string(),
                    style: TextStyle {
                        font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                        font_size: 30.0,
                        color: Color::WHITE,
                    },
                },
            ],
            ..Default::default()
        },
        ..Default::default()
    });

    commands.insert_resource(CurrentMesh(HANDLE_TRIMESH_OPTIMIZED.typed()));
}

fn check_textures(
    mut state: ResMut<State<AppState>>,
    navmeshes: ResMut<GltfHandles>,
    asset_server: Res<AssetServer>,
) {
    if let LoadState::Loaded = asset_server.get_load_state(navmeshes.handle.id()) {
        state.set(AppState::Playing).unwrap();
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

#[derive(Component)]
struct Waiting(Timer);

#[derive(Component, Clone)]
struct NavMeshDisp(Handle<PathMesh>);

fn mesh_cleanup_from_blender(mesh: &Mesh) -> PathMesh {
    let normal = Vec3::Y;
    let rotation = Quat::from_rotation_arc(normal, Vec3::Z);

    let vertices = match mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap() {
        VertexAttributeValues::Float32x3(values) => values,
        // Guaranteed by Bevy
        _ => unreachable!(),
    }
    .iter()
    .cloned()
    .map(Vec3::from)
    .collect::<Vec<_>>();

    let mut merged = vec![];
    let mut kept = vec![];
    let mut new_indexes = vec![];
    for (index, vertice) in vertices.iter().enumerate() {
        if vertice.y < 0.0 {
            merged.push(usize::MAX);
            new_indexes.push(usize::MAX);
        } else if let Some(close_to) = merged
            .iter()
            .filter(|p| **p < usize::MAX)
            .find(|p| vertice.distance_squared(vertices[**p]) < 0.1)
        {
            merged.push(*close_to);
            new_indexes.push(usize::MAX);
        } else {
            merged.push(index);
            kept.push(index);
            new_indexes.push(kept.len() - 1);
        }
    }

    let triangles: Vec<_> = mesh
        .indices()
        .expect("No polygon indices found in mesh")
        .iter()
        .tuples::<(_, _, _)>()
        .map(|t| (merged[t.0], merged[t.1], merged[t.2]))
        .filter(|t| t.0 < usize::MAX && t.1 < usize::MAX && t.2 < usize::MAX)
        .map(|i| (new_indexes[i.0], new_indexes[i.1], new_indexes[i.2]))
        .filter(|(a, b, c)| a != b && a != c && b != c)
        .map(polyanya::Triangle::from)
        .collect();

    let vertices: Vec<Vec2> = vertices
        .into_iter()
        .enumerate()
        .filter_map(|(i, p)| kept.contains(&i).then_some(p))
        .map(|vertex| rotation.mul_vec3(vertex))
        .map(|coords| coords.xy())
        .collect();

    let polyanya_mesh = polyanya::Mesh::from_trimesh(vertices, triangles);

    let mut path_mesh = PathMesh::from_polyanya_mesh(polyanya_mesh);
    path_mesh.set_transform(Transform::from_rotation(rotation));
    path_mesh
}

fn setup_scene(
    mut commands: Commands,
    navmeshes: Res<GltfHandles>,
    gltfs: Res<Assets<Gltf>>,
    gltf_meshes: Res<Assets<GltfMesh>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut pathmeshes: ResMut<Assets<PathMesh>>,
) {
    let mut material: StandardMaterial = Color::ALICE_BLUE.into();
    material.perceptual_roughness = 1.0;
    let ground_material = materials.add(material);
    if let Some(gltf) = gltfs.get(&navmeshes.handle) {
        let mesh = gltf_meshes.get(&gltf.named_meshes["obstacles"]).unwrap();
        let mut material: StandardMaterial = Color::GRAY.into();
        material.perceptual_roughness = 1.0;
        commands.spawn(PbrBundle {
            mesh: mesh.primitives[0].mesh.clone(),
            material: materials.add(material),
            ..default()
        });

        let mesh = gltf_meshes.get(&gltf.named_meshes["plane"]).unwrap();
        commands.spawn(PbrBundle {
            mesh: mesh.primitives[0].mesh.clone(),
            transform: Transform::from_xyz(0.0, 0.1, 0.0),
            material: ground_material.clone(),
            ..default()
        });
    }

    {
        #[cfg(target_arch = "wasm32")]
        const NB_HOVER: usize = 5;
        #[cfg(not(target_arch = "wasm32"))]
        const NB_HOVER: usize = 10;

        for _i in 0..NB_HOVER {
            commands.spawn((
                SpotLightBundle {
                    spot_light: SpotLight {
                        intensity: 800.0,
                        color: Color::SEA_GREEN,
                        shadows_enabled: true,
                        inner_angle: 0.5,
                        outer_angle: 0.8,
                        range: 250.0,
                        ..default()
                    },
                    transform: Transform::from_xyz(
                        rand::thread_rng().gen_range(-50.0..50.0),
                        20.0,
                        rand::thread_rng().gen_range(-25.0..25.0),
                    )
                    .with_rotation(Quat::from_rotation_x(-FRAC_PI_2)),
                    ..default()
                },
                Hover(Vec2::new(
                    rand::thread_rng().gen_range(-50.0..50.0),
                    rand::thread_rng().gen_range(-25.0..25.0),
                )),
            ));
        }
    }

    if let Some(gltf) = gltfs.get(&navmeshes.handle) {
        // due to how this mesh was built in Blender, it has volume and duplicated vertices
        {
            let navmesh = mesh_cleanup_from_blender(
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

            commands.spawn((
                PbrBundle {
                    mesh: meshes.add(navmesh.to_mesh()),
                    material: ground_material.clone(),
                    transform: Transform::from_xyz(0.0, 0.2, 0.0),
                    visibility: Visibility::INVISIBLE,
                    ..Default::default()
                },
                #[cfg(not(target_arch = "wasm32"))]
                Wireframe,
                NavMeshDisp(HANDLE_TRIMESH_OPTIMIZED.typed()),
            ));
            pathmeshes.set_untracked(HANDLE_TRIMESH_OPTIMIZED, navmesh);
        }

        commands
            .spawn((
                PbrBundle {
                    mesh: meshes.add(Mesh::from(shape::Capsule { ..default() })),
                    material: materials.add(Color::BLUE.into()),
                    transform: Transform::from_xyz(-1.0, 0.0, -2.0),
                    ..Default::default()
                },
                Object(None),
                NotShadowCaster,
                Waiting(Timer::from_seconds(0.25, TimerMode::Once)),
            ))
            .add_children(|object| {
                object.spawn(PointLightBundle {
                    point_light: PointLight {
                        color: Color::BLUE,
                        range: 500.0,
                        intensity: 2000.0,
                        shadows_enabled: true,
                        ..default()
                    },
                    transform: Transform::from_xyz(0.0, 1.2, 0.0),
                    ..default()
                });
            });
    }
}

fn give_target_auto(
    mut commands: Commands,
    mut object_query: Query<(Entity, &Transform, &mut Waiting, &mut Object)>,
    navmeshes: Res<Assets<PathMesh>>,
    time: Res<Time>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    current_mesh: Res<CurrentMesh>,
) {
    for (entity, transform, mut waiting, mut object) in object_query.iter_mut() {
        if !waiting.0.tick(time.delta()).finished() {
            break;
        }

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
            break
        };
        if let Some((first, remaining)) = path.path.split_first() {
            let mut remaining = remaining.to_vec();
            remaining.reverse();
            let target_id = commands
                .spawn((
                    PbrBundle {
                        mesh: meshes.add(Mesh::from(shape::UVSphere {
                            radius: 0.5,
                            ..default()
                        })),
                        material: materials.add(Color::RED.into()),
                        transform: Transform::from_xyz(x, 0.0, z),
                        ..Default::default()
                    },
                    NotShadowCaster,
                    Target,
                ))
                .with_children(|target| {
                    target.spawn(PointLightBundle {
                        point_light: PointLight {
                            color: Color::RED,
                            shadows_enabled: true,
                            range: 3.0,
                            ..default()
                        },
                        transform: Transform::from_xyz(0.0, 1.5, 0.0),
                        ..default()
                    });
                })
                .id();
            commands
                .entity(entity)
                .insert(Path {
                    current: first.clone(),
                    next: remaining,
                })
                .remove::<Waiting>();
            object.0 = Some(target_id);
        }
    }
}

fn give_target_on_click(
    mut commands: Commands,
    mut object_query: Query<(Entity, &Transform, &mut Object)>,
    targets: Query<Entity, With<Target>>,
    navmeshes: Res<Assets<PathMesh>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    current_mesh: Res<CurrentMesh>,
    mouse_buttons: Res<Input<MouseButton>>,
    windows: Res<Windows>,
    camera: Query<(&Camera, &GlobalTransform)>,
) {
    if mouse_buttons.just_pressed(MouseButton::Left) {
        let navmesh = navmeshes.get(&current_mesh.0).unwrap();
        let Some(target) = (|| {
            let position = windows.primary().cursor_position()?;
            let (camera, transform) = camera.get_single().ok()?;
            let ray = camera.viewport_to_world(transform, position)?;
            let denom = Vec3::Y.dot(ray.direction);
            let t =  (Vec3::ZERO - ray.origin).dot(Vec3::Y) / denom;
            let target = ray.origin + ray.direction * t;
            navmesh.transformed_is_in_mesh(target).then_some(target)
        })() else {
            return
        };

        for (entity, transform, mut object) in object_query.iter_mut() {
            let Some(path) = navmesh.transformed_path(transform.translation, target)
            else {
                break
            };
            if let Some((first, remaining)) = path.path.split_first() {
                let mut remaining = remaining.to_vec();
                remaining.reverse();
                let target_id = commands
                    .spawn((
                        PbrBundle {
                            mesh: meshes.add(Mesh::from(shape::UVSphere {
                                radius: 0.5,
                                ..default()
                            })),
                            material: materials.add(Color::RED.into()),
                            transform: Transform::from_translation(target),
                            ..Default::default()
                        },
                        NotShadowCaster,
                        Target,
                    ))
                    .with_children(|target| {
                        target.spawn(PointLightBundle {
                            point_light: PointLight {
                                color: Color::RED,
                                shadows_enabled: true,
                                range: 3.0,
                                ..default()
                            },
                            transform: Transform::from_xyz(0.0, 1.5, 0.0),
                            ..default()
                        });
                    })
                    .id();
                commands
                    .entity(entity)
                    .insert(Path {
                        current: first.clone(),
                        next: remaining,
                    })
                    .remove::<Waiting>();
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
        transform.translation += move_direction.normalize() * time.delta_seconds() * 10.0;
        if transform.translation.distance(target.current) < 0.1 {
            if let Some(next) = target.next.pop() {
                target.current = next;
            } else {
                commands
                    .entity(entity)
                    .remove::<Path>()
                    .insert(Waiting(Timer::from_seconds(0.1, TimerMode::Once)));
                let target_entity = object.0.take().unwrap();
                commands.entity(target_entity).despawn_recursive();
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn trigger_navmesh_visibility(
    mut query: Query<(&mut Visibility, &NavMeshDisp)>,
    keyboard_input: ResMut<Input<KeyCode>>,
    current_mesh: Res<CurrentMesh>,
) {
    if keyboard_input.just_pressed(KeyCode::Space) {
        for (mut visible, nav) in query.iter_mut() {
            if nav.0 == current_mesh.0 {
                visible.is_visible = !visible.is_visible;
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
            (time.elapsed_seconds() * 10.0).sin().abs() * 100.0;
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
        transform.translation += ((hover.0 - current).normalize() * time.delta_seconds() * 5.0)
            .extend(0.0)
            .xzy();
    }
}
