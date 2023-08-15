use std::f32::consts::FRAC_PI_2;

use bevy::{
    core_pipeline::clear_color::ClearColorConfig,
    math::{vec2, vec3},
    pbr::NotShadowCaster,
    prelude::*,
    reflect::TypeUuid,
    render::view::{RenderLayers, VisibilitySystems},
};
use bevy_pathmesh::{
    updater::{NavMeshBundle, NavMeshSettings, NavMeshStatus, NavMeshUpdateMode},
    PathMesh, PathMeshPlugin, PolyanyaTriangulation,
};
use bevy_vector_shapes::Shape2dPlugin;
use rand::Rng;

mod build_navmesh;
mod helpers;
mod ui;

const HANDLE_NAVMESH_WIREFRAME: HandleUntyped = HandleUntyped::weak_from_u64(Mesh::TYPE_UUID, 1);
const HANDLE_NAVMESH_MESH: HandleUntyped = HandleUntyped::weak_from_u64(Mesh::TYPE_UUID, 2);

const HANDLE_OBSTACLE_MESH: HandleUntyped = HandleUntyped::weak_from_u64(Mesh::TYPE_UUID, 3);
const HANDLE_AGENT_MESH: HandleUntyped = HandleUntyped::weak_from_u64(Mesh::TYPE_UUID, 4);
const HANDLE_TARGET_MESH: HandleUntyped = HandleUntyped::weak_from_u64(Mesh::TYPE_UUID, 5);

const HANDLE_OBSTACLE_MATERIAL: HandleUntyped =
    HandleUntyped::weak_from_u64(StandardMaterial::TYPE_UUID, 1);
const HANDLE_AGENT_MATERIAL: HandleUntyped =
    HandleUntyped::weak_from_u64(StandardMaterial::TYPE_UUID, 2);
const HANDLE_TARGET_MATERIAL: HandleUntyped =
    HandleUntyped::weak_from_u64(StandardMaterial::TYPE_UUID, 3);

const BOARD_LIMIT: f32 = 4.4;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Navmesh with Polyanya".to_string(),
                    fit_canvas_to_parent: true,
                    ..default()
                }),
                ..default()
            }),
            bevy_mod_picking::DefaultPickingPlugins,
            bevy_transform_gizmo::TransformGizmoPlugin::default(),
            Shape2dPlugin::default(),
            PathMeshPlugin,
        ))
        .add_plugins((
            ui::UiPlugin,
            helpers::HelperPlugin,
            build_navmesh::BuilderPlugin,
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, (give_target_auto, move_agent, display_path))
        .add_systems(
            PostUpdate,
            (find_path_to_target, apply_deferred)
                .chain()
                .before(VisibilitySystems::CalculateBounds),
        )
        .insert_resource(GizmoConfig {
            depth_bias: -1.0,
            render_layers: RenderLayers::layer(1),
            ..default()
        })
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut pathmeshes: ResMut<Assets<PathMesh>>,
) {
    meshes.set_untracked(HANDLE_OBSTACLE_MESH, Mesh::from(shape::Cube { size: 0.4 }));
    meshes.set_untracked(
        HANDLE_AGENT_MESH,
        Mesh::from(shape::Capsule {
            radius: 0.1,
            depth: 0.2,
            ..default()
        }),
    );
    meshes.set_untracked(
        HANDLE_TARGET_MESH,
        Mesh::from(shape::UVSphere {
            radius: 0.05,
            ..default()
        }),
    );
    materials.set_untracked(
        HANDLE_OBSTACLE_MATERIAL,
        StandardMaterial {
            base_color: Color::NONE,
            alpha_mode: AlphaMode::Blend,
            ..default()
        },
    );
    materials.set_untracked(
        HANDLE_AGENT_MATERIAL,
        StandardMaterial {
            base_color: Color::GREEN,
            ..default()
        },
    );
    materials.set_untracked(
        HANDLE_TARGET_MATERIAL,
        StandardMaterial {
            base_color: Color::YELLOW,
            unlit: true,
            ..default()
        },
    );

    let mut pathmesh = bevy_pathmesh::PathMesh::from_edge_and_obstacles(
        vec![vec2(-5., -5.), vec2(5., -5.), vec2(5., 5.), vec2(-5., 5.)],
        vec![],
    );
    pathmesh.set_transform(Transform::from_rotation(Quat::from_rotation_x(FRAC_PI_2)));
    meshes.set_untracked(HANDLE_NAVMESH_WIREFRAME, pathmesh.to_wireframe_mesh());
    meshes.set_untracked(HANDLE_NAVMESH_MESH, pathmesh.to_mesh());
    commands.spawn(NavMeshBundle {
        settings: NavMeshSettings {
            simplify: 0.0,
            merge_steps: 0,
            default_delta: 0.01,
            fixed: PolyanyaTriangulation::from_outer_edges(&vec![
                vec2(-5., -5.),
                vec2(5., -5.),
                vec2(5., 5.),
                vec2(-5., 5.),
            ]),
        },
        status: NavMeshStatus::Building,
        handle: pathmeshes.add(pathmesh),
        transform: Transform::from_rotation(Quat::from_rotation_x(FRAC_PI_2)),
        update_mode: NavMeshUpdateMode::Debounced(0.025),
    });
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Plane {
                size: 10.0,
                ..default()
            })),
            material: materials.add(StandardMaterial {
                base_color: Color::BEIGE,
                perceptual_roughness: 1.0,
                metallic: 0.0,
                ..default()
            }),
            transform: Transform::from_xyz(0.0, -0.05, 0.0),
            ..default()
        },
        RenderLayers::layer(1),
    ));
    commands.spawn((
        PbrBundle {
            mesh: HANDLE_NAVMESH_MESH.typed(),
            material: materials.add(StandardMaterial {
                base_color: Color::MIDNIGHT_BLUE,
                perceptual_roughness: 1.0,
                metallic: 0.0,
                ..default()
            }),
            ..default()
        },
        RenderLayers::layer(1),
    ));
    commands.spawn((
        PbrBundle {
            mesh: HANDLE_NAVMESH_WIREFRAME.typed(),
            transform: Transform::from_translation(Vec3::new(0., 0.01, 0.)),
            material: materials.add(StandardMaterial {
                base_color: Color::RED,
                unlit: true,
                ..default()
            }),
            ..default()
        },
        RenderLayers::layer(1),
    ));

    // light
    commands.spawn((
        PointLightBundle {
            transform: Transform::from_xyz(0.0, 8.0, 0.0),
            ..Default::default()
        },
        RenderLayers::layer(1),
    ));
    // camera
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(-5.0, 10.0, 0.0)
                .looking_at(Vec3::new(-0.8, 0.0, 0.0), Vec3::Y),
            ..Default::default()
        },
        UiCameraConfig { show_ui: false },
        bevy_mod_picking::backends::raycast::RaycastPickCamera::default(),
        bevy_transform_gizmo::GizmoPickSource::default(),
        RenderLayers::layer(1),
    ));
    commands.spawn(Camera2dBundle {
        camera: Camera {
            order: 1,
            ..default()
        },
        camera_2d: Camera2d {
            clear_color: ClearColorConfig::None,
        },
        ..default()
    });
}

#[derive(Component)]
struct Obstacle;

#[derive(Component)]
struct Agent {
    target: Option<Entity>,
}

#[derive(Component)]
struct Target;

#[derive(Component)]
struct Path {
    current: Vec3,
    next: Vec<Vec3>,
}

fn give_target_auto(
    mut commands: Commands,
    mut object_query: Query<&mut Agent, Without<Path>>,
    navmeshes: Res<Assets<PathMesh>>,
    navmesh: Query<&Handle<PathMesh>>,
) {
    for mut agent in object_query.iter_mut() {
        if agent.target.is_some() {
            continue;
        }
        let navmesh = navmeshes.get(navmesh.single()).unwrap();
        let mut x;
        let mut z;
        loop {
            x = rand::thread_rng().gen_range(-4.75..4.75);
            z = rand::thread_rng().gen_range(-4.75..4.75);

            if navmesh.transformed_is_in_mesh(Vec3::new(x, 0.0, z)) {
                break;
            }
        }
        let target_id = commands
            .spawn((
                PbrBundle {
                    mesh: HANDLE_TARGET_MESH.typed(),
                    material: HANDLE_TARGET_MATERIAL.typed(),
                    transform: Transform::from_xyz(x, 0.0, z),
                    ..Default::default()
                },
                NotShadowCaster,
                Target,
                RenderLayers::layer(1),
            ))
            .id();
        agent.target = Some(target_id);
    }
}

fn find_path_to_target(
    mut commands: Commands,
    agents: Query<(Entity, &Transform, &Agent), (With<Agent>, Without<Path>)>,
    targets: Query<&Transform, With<Target>>,
    mut navmeshes: ResMut<Assets<PathMesh>>,
    navmesh: Query<(&Handle<PathMesh>, &NavMeshSettings)>,
) {
    let (navmesh_handle, settings) = navmesh.single();
    let navmesh = navmeshes.get(navmesh_handle).unwrap();
    let current_delta = navmesh.delta();
    let mut has_unreachable = false;

    for (agent_entity, from, agent) in &agents {
        if agent.target.is_none() {
            continue;
        }
        let Ok(target) = targets.get(agent.target.unwrap()) else {
            continue;
        };
        let Some(path) = navmesh.transformed_path(from.translation, target.translation)
        else {
            has_unreachable = true;
            continue
        };
        if let Some((first, remaining)) = path.path.split_first() {
            let mut remaining = remaining.to_vec();
            remaining.reverse();

            commands.entity(agent_entity).insert(Path {
                current: first.clone(),
                next: remaining,
            });
        }
    }
    if has_unreachable {
        let navmesh = navmeshes.get_mut(navmesh_handle).unwrap();
        warn!(
            "some agents have an unreachable target, increasing delta to {}",
            (current_delta * 3.0).min(settings.default_delta * 1000.0)
        );
        navmesh.set_delta((current_delta * 3.0).min(settings.default_delta * 1000.0));
    } else {
        if current_delta != settings.default_delta {
            info!("resetting delta");
            let navmesh = navmeshes.get_mut(navmesh_handle).unwrap();
            navmesh.set_delta(settings.default_delta);
        }
    }
}

fn move_agent(
    mut commands: Commands,
    mut object_query: Query<(&mut Transform, &mut Path, Entity, &mut Agent)>,
    time: Res<Time>,
) {
    for (mut transform, mut path, entity, mut object) in object_query.iter_mut() {
        let move_direction = path.current - transform.translation;
        transform.translation += move_direction.normalize() * time.delta_seconds() * 1.0;
        if transform.translation.distance(path.current) < 0.01 {
            if let Some(next) = path.next.pop() {
                path.current = next;
            } else {
                commands.entity(entity).remove::<Path>();
                let target_entity = object.target.take().unwrap();
                commands.entity(target_entity).despawn_recursive();
            }
        }
    }
}

fn display_path(query: Query<(&Transform, &Path)>, mut gizmos: Gizmos) {
    for (transform, path) in &query {
        let mut next = path.next.clone();
        next.reverse();

        let count = next.len() + 2;

        gizmos.linestrip_gradient(
            std::iter::once(vec3(transform.translation.x, 0.0, transform.translation.z))
                .chain(std::iter::once(path.current))
                .chain(next.into_iter())
                .zip(
                    (0..count).map(|i| {
                        Color::hsl(120.0 - 60.0 * (i + 1) as f32 / count as f32, 1.0, 0.5)
                    }),
                ),
        );
    }
}
