use bevy::{
    asset::LoadState,
    diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin},
    gltf::{Gltf, GltfMesh},
    prelude::*,
    render::wireframe::{Wireframe, WireframeConfig, WireframePlugin},
    wgpu::{WgpuFeature, WgpuFeatures, WgpuOptions},
};
use rand::Rng;

use navmesh::*;

fn main() {
    App::build()
        .insert_resource(Msaa { samples: 4 })
        .insert_resource(ClearColor(Color::rgb(0., 0., 0.01)))
        // .insert_resource(WireframeConfig { global: true })
        .insert_resource(WgpuOptions {
            features: WgpuFeatures {
                // The Wireframe requires NonFillPolygonMode feature
                features: vec![WgpuFeature::NonFillPolygonMode],
            },
            ..Default::default()
        })
        .init_resource::<GltfHandles>()
        .add_plugins(DefaultPlugins)
        .add_plugin(WireframePlugin)
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_startup_system(setup.system())
        .init_resource::<Option<NavMesh>>()
        .add_state(AppState::Setup)
        .add_system_set(SystemSet::on_update(AppState::Setup).with_system(check_textures.system()))
        .add_system_set(SystemSet::on_enter(AppState::Playing).with_system(setup_scene.system()))
        .add_system_set(
            SystemSet::on_update(AppState::Playing)
                .with_system(give_target.system())
                .with_system(move_object.system())
                .with_system(display_fps.system())
                .with_system(rotate_camera.system()),
        )
        .run();
}

#[derive(Clone, PartialEq, Eq)]
enum AppState {
    Setup,
    Playing,
}

#[derive(Default)]
struct GltfHandles {
    handles: Vec<Handle<Gltf>>,
}

struct FpsText;

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut navmeshes: ResMut<GltfHandles>,
) {
    navmeshes.handles = vec![asset_server.load("meshes/plane_with_holes.glb")];

    commands
        .spawn(LightBundle {
            transform: Transform::from_xyz(0.0, 10.0, 0.0),
            light: Light {
                range: 40.0,
                intensity: 500.0,
                ..Default::default()
            },
            ..Default::default()
        })
        .spawn(PerspectiveCameraBundle {
            transform: Transform::from_xyz(0.0, 40.0, 0.1)
                .looking_at(Vec3::new(0.0, 0.3, 0.0), Vec3::Y),
            ..Default::default()
        })
        .with(Camera);

    commands
        .spawn(UiCameraBundle::default())
        .spawn(TextBundle {
            style: Style {
                align_self: AlignSelf::FlexEnd,
                ..Default::default()
            },
            // Use `Text` directly
            text: Text {
                // Construct a `Vec` of `TextSection`s
                sections: vec![
                    TextSection {
                        value: "FPS: ".to_string(),
                        style: TextStyle {
                            font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                            font_size: 30.0,
                            color: Color::WHITE,
                        },
                    },
                    TextSection {
                        value: "".to_string(),
                        style: TextStyle {
                            font: asset_server.load("fonts/FiraMono-Medium.ttf"),
                            font_size: 30.0,
                            color: Color::GOLD,
                        },
                    },
                ],
                ..Default::default()
            },
            ..Default::default()
        })
        .with(FpsText);
}

fn display_fps(diagnostics: Res<Diagnostics>, mut query: Query<&mut Text, With<FpsText>>) {
    for mut text in query.iter_mut() {
        if let Some(fps) = diagnostics.get(FrameTimeDiagnosticsPlugin::FPS) {
            if let Some(average) = fps.average() {
                // Update the value of the second section
                text.sections[1].value = format!("{:.2}", average);
            }
        }
    }
}

fn check_textures(
    mut state: ResMut<State<AppState>>,
    navmeshes: ResMut<GltfHandles>,
    asset_server: Res<AssetServer>,
) {
    if let LoadState::Loaded =
        asset_server.get_group_load_state(navmeshes.handles.iter().map(|handle| handle.id))
    {
        state.set_next(AppState::Playing).unwrap();
    }
}

struct Path {
    current: Vec3,
    next: Vec<Vec3>,
}
struct Object;
struct Target;
struct Waiting(Timer);

fn setup_scene(
    mut commands: Commands,
    navmeshes: Res<GltfHandles>,
    gltfs: Res<Assets<Gltf>>,
    gltf_meshes: Res<Assets<GltfMesh>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut navmesh: ResMut<Option<NavMesh>>,
) {
    if let Some(gltf) = gltfs.get(&navmeshes.handles[0]) {
        let gltf_mesh_handle = gltf.meshes[0].clone();
        let gltf_mesh = gltf_meshes.get(gltf_mesh_handle).unwrap();
        let mesh_handle = gltf_mesh.primitives[0].mesh.clone();

        commands
            .spawn(PbrBundle {
                mesh: mesh_handle.clone(),
                material: materials.add(Color::ORANGE.into()),
                ..Default::default()
            })
            .with(Wireframe);

        let mesh = meshes.get(mesh_handle).unwrap();

        *navmesh = Some(NavMesh::from_mesh(mesh));

        let mut x;
        let mut z;
        loop {
            x = rand::thread_rng().gen_range(-20.0..20.0);
            z = rand::thread_rng().gen_range(-20.0..20.0);
            if navmesh
                .as_ref()
                .unwrap()
                .point_in_mesh(Vec3::new(x, 0.0, z))
            {
                break;
            }
        }

        commands
            .spawn(PbrBundle {
                mesh: meshes.add(Mesh::from(shape::Cube { size: 0.7 })),
                material: materials.add(Color::BLUE.into()),
                transform: Transform::from_xyz(x, 0.0, z),
                ..Default::default()
            })
            .with(Object)
            .with(Waiting(Timer::from_seconds(1.0, false)));
    }
}

fn give_target(
    mut commands: Commands,
    mut object_query: Query<(Entity, &Transform, &mut Waiting)>,
    navmesh: Res<Option<NavMesh>>,
    time: Res<Time>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (entity, transform, mut waiting) in object_query.iter_mut() {
        if !waiting.0.tick(time.delta()).finished() {
            break;
        }

        let mut x;
        let mut z;
        loop {
            x = rand::thread_rng().gen_range(-20.0..20.0);
            z = rand::thread_rng().gen_range(-20.0..20.0);
            if navmesh
                .as_ref()
                .unwrap()
                .point_in_mesh(Vec3::new(x, 0.0, z))
            {
                break;
            }
        }

        let path = navmesh
            .as_ref()
            .unwrap()
            .path_from_to(transform.translation, Vec3::new(x, 0.0, z));

        if let Some((first, remaining)) = path.split_first() {
            let mut remaining = remaining.to_vec();
            remaining.reverse();
            commands.insert(
                entity,
                Path {
                    current: first.clone(),
                    next: remaining,
                },
            );
            commands.remove::<Waiting>(entity);
            commands
                .spawn(PbrBundle {
                    mesh: meshes.add(Mesh::from(shape::Cube { size: 0.5 })),
                    material: materials.add(Color::GREEN.into()),
                    transform: Transform::from_xyz(x, 0.0, z),
                    ..Default::default()
                })
                .with(Target);
        } else {
            commands.spawn(PbrBundle {
                mesh: meshes.add(Mesh::from(shape::Cube { size: 0.5 })),
                material: materials.add(Color::RED.into()),
                transform: Transform::from_xyz(x, 0.0, z),
                ..Default::default()
            });
        }
    }
}

fn move_object(
    mut commands: Commands,
    mut object_query: Query<(&mut Transform, &mut Path, Entity)>,
    target_query: Query<Entity, With<Target>>,
    time: Res<Time>,
) {
    for (mut transform, mut target, entity) in object_query.iter_mut() {
        let move_direction = target.current - transform.translation;
        transform.translation += move_direction.normalize() * time.delta_seconds() * 10.0;
        if transform.translation.distance(target.current) < 0.1 {
            if let Some(next) = target.next.pop() {
                target.current = next;
            } else {
                commands.remove::<Path>(entity);
                commands.insert(entity, Waiting(Timer::from_seconds(0.01, false)));
                for target in target_query.iter() {
                    commands.despawn_recursive(target);
                }
            }
        }
    }
}

fn rotate_camera(time: Res<Time>, mut camera_query: Query<&mut Transform, With<Camera>>) {
    for mut camera in camera_query.iter_mut() {
        *camera = Transform::from_xyz(
            (time.seconds_since_startup() / 40.0).sin() as f32 * 20.0,
            40.0,
            (time.seconds_since_startup() / 40.0).cos() as f32 * 20.0,
        )
        .looking_at(Vec3::new(0.0, 0.3, 0.0), Vec3::Y);
    }
}
