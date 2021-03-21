use bevy::{
    asset::LoadState,
    diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin},
    gltf::{Gltf, GltfMesh},
    prelude::*,
    render::{
        mesh::{Indices, VertexAttributeValues},
        pipeline::PrimitiveTopology,
        wireframe::{WireframeConfig, WireframePlugin},
    },
    wgpu::{WgpuFeature, WgpuFeatures, WgpuOptions},
};
use rand::Rng;

fn main() {
    App::build()
        .insert_resource(Msaa { samples: 4 })
        .insert_resource(ClearColor(Color::rgb(0., 0., 0.01)))
        .insert_resource(WireframeConfig { global: true })
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
                .with_system(display_fps.system()),
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
        });

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

struct Target(Vec3);
struct Object;

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

        commands.spawn(PbrBundle {
            mesh: mesh_handle.clone(),
            material: materials.add(Color::ORANGE.into()),
            ..Default::default()
        });

        let mesh = meshes.get(mesh_handle).unwrap();

        *navmesh = Some(NavMesh::from_mesh(mesh));

        let mut x;
        let y = 0.0;
        let mut z;
        loop {
            x = rand::thread_rng().gen_range(-20.0..20.0);
            z = rand::thread_rng().gen_range(-20.0..20.0);
            if navmesh
                .as_ref()
                .unwrap()
                .point_in_mesh(Vec3::new(x, y, z))
                .is_some()
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
            .with(Object);
    }
}

fn give_target(
    mut commands: Commands,
    object_query: Query<Entity, (With<Object>, Without<Target>)>,
    navmesh: Res<Option<NavMesh>>,
) {
    if let Some(entity) = object_query.iter().next() {
        let mut x;
        let y = 0.0;
        let mut z;
        loop {
            x = rand::thread_rng().gen_range(-20.0..20.0);
            z = rand::thread_rng().gen_range(-20.0..20.0);
            if navmesh
                .as_ref()
                .unwrap()
                .point_in_mesh(Vec3::new(x, y, z))
                .is_some()
            {
                break;
            }
        }

        commands.insert(entity, Target(Vec3::new(x, y, z)));
    }
}

fn move_object(
    mut commands: Commands,
    mut object_query: Query<(&mut Transform, &Target, Entity)>,
    time: Res<Time>,
) {
    for (mut transform, target, entity) in object_query.iter_mut() {
        let move_direction = target.0 - transform.translation;
        transform.translation += move_direction.normalize() * time.delta_seconds() * 5.0;
        if transform.translation.distance(target.0) < 0.05 {
            commands.remove::<Target>(entity);
        }
    }
}

pub struct NavMesh {
    triangles: Vec<(Vec3, Vec3, Vec3)>,
}
impl NavMesh {
    pub fn from_mesh(mesh: &Mesh) -> NavMesh {
        fn mesh_to_list(mesh: &Mesh) -> Option<Vec<(Vec3, Vec3, Vec3)>> {
            let indices = match mesh.primitive_topology() {
                PrimitiveTopology::TriangleList => mesh.indices()?,
                PrimitiveTopology::TriangleStrip => mesh.indices()?,
                _ => return None,
            };

            let indices: Vec<usize> = match indices {
                Indices::U16(indices) => indices.iter().map(|v| *v as usize).collect(),
                Indices::U32(indices) => indices.iter().map(|v| *v as usize).collect(),
            };

            let grouped_indices = indices
                .iter()
                .fold((vec![], vec![]), |(mut triangles, mut buffer), i| {
                    buffer.push(*i);
                    if buffer.len() == 3 {
                        triangles.push(buffer.clone());
                        buffer = vec![];
                        if mesh.primitive_topology() == PrimitiveTopology::LineStrip {
                            buffer.push(*i);
                        }
                    }
                    (triangles, buffer)
                })
                .0;

            if let VertexAttributeValues::Float3(positions) = mesh.attribute("Vertex_Position")? {
                return Some(
                    grouped_indices
                        .iter()
                        .map(|indices| {
                            (
                                Vec3::from_slice_unaligned(&positions[indices[0]]),
                                Vec3::from_slice_unaligned(&positions[indices[1]]),
                                Vec3::from_slice_unaligned(&positions[indices[2]]),
                            )
                        })
                        .collect(),
                );
            }
            None
        }

        NavMesh {
            triangles: mesh_to_list(mesh).unwrap_or_default(),
        }
    }

    pub fn point_in_mesh(&self, point: Vec3) -> Option<(Vec3, Vec3, Vec3)> {
        self.triangles
            .iter()
            .filter(|(a, b, c)| point_in_triangle(point, (a, b, c)))
            .next()
            .cloned()
    }
}

fn point_in_triangle(point: Vec3, (a, b, c): (&Vec3, &Vec3, &Vec3)) -> bool {
    let a = *a - point;
    let b = *b - point;
    let c = *c - point;

    let u = b.cross(c);
    let v = c.cross(a);
    let w = a.cross(b);

    if u.dot(v) < 0.0 {
        false
    } else if u.dot(w) < 0.0 {
        false
    } else {
        true
    }
}
