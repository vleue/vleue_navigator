use std::f32::consts::FRAC_PI_2;

use bevy::{color::palettes, prelude::*};
use polyanya::{U32Layer, Vec2Helper};
use vleue_navigator::{
    VleueNavigatorPlugin, display_mesh_gizmo, display_polygon_gizmo, prelude::*,
};

#[path = "helpers/camera_controller.rs"]
mod camera_controller;

fn main() {
    let navmesh = polyanya::RecastPolyMesh::from_file("assets/recast/poly_mesh.json").into();
    let detailed_navmesh =
        polyanya::RecastPolyMeshDetail::from_file("assets/recast/detail_mesh.json").into();

    App::new()
        .insert_resource(ClearColor(Color::srgb(0., 0., 0.01)))
        .add_plugins((
            DefaultPlugins,
            camera_controller::CameraControllerPlugin,
            VleueNavigatorPlugin,
            NavmeshUpdaterPlugin::<PrimitiveObstacle>::default(),
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, draw_parsed_recast_navmesh)
        .insert_resource(RecastNavmesh(navmesh, detailed_navmesh))
        .insert_gizmo_config::<NavMeshGizmos>(
            NavMeshGizmos,
            GizmoConfig {
                line: GizmoLineConfig {
                    style: GizmoLineStyle::Dashed {
                        gap_scale: 6.0,
                        line_scale: 4.0,
                    },
                    ..default()
                },
                ..default()
            },
        )
        .insert_gizmo_config::<PathGizmos>(
            PathGizmos,
            GizmoConfig {
                line: GizmoLineConfig {
                    width: 10.0,
                    ..default()
                },
                ..default()
            },
        )
        .run();
}

#[derive(Default, Reflect, GizmoConfigGroup)]
struct NavMeshGizmos;
#[derive(Default, Reflect, GizmoConfigGroup)]
struct PathGizmos;

#[derive(Resource)]
struct RecastNavmesh(polyanya::Mesh, polyanya::Mesh);

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(50.0, 150.0, -30.0).looking_at(Vec3::new(0.0, 0.0, -30.0), Vec3::Y),
        camera_controller::CameraController::default(),
    ));

    commands.spawn(SceneRoot(
        asset_server.load(GltfAssetLabel::Scene(0).from_asset("recast/dungeon.glb")),
    ));

    commands.spawn((
        DirectionalLight {
            illuminance: light_consts::lux::OVERCAST_DAY,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(1.0, 1.0, 0.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn draw_parsed_recast_navmesh(
    mut navmesh_gizmos: Gizmos<NavMeshGizmos>,
    mut path_gizmos: Gizmos<PathGizmos>,
    mut gizmos: Gizmos,
    recast: Res<RecastNavmesh>,
    time: Res<Time>,
) {
    let start = vec3(46.998413, 9.998184, 1.717747);
    let end = vec3(20.703018, 18.651773, -80.770203);

    gizmos.sphere(start, 0.5, palettes::tailwind::LIME_400);
    gizmos.sphere(end, 0.5, palettes::tailwind::YELLOW_400);

    let mesh = if (time.elapsed_secs() as u32 / 5) % 2 == 0 {
        &recast.0
    } else {
        &recast.1
    };
    // let mesh = &recast.0;
    // let mesh = &recast.1;
    display_mesh_gizmo(
        mesh,
        &Transform::from_rotation(Quat::from_rotation_x(FRAC_PI_2)).into(),
        &[
            palettes::tailwind::BLUE_400.into(),
            palettes::tailwind::CYAN_400.into(),
            palettes::tailwind::GREEN_400.into(),
            palettes::tailwind::YELLOW_400.into(),
        ],
        &mut navmesh_gizmos,
    );
    let Some(path) = mesh.path(start.xz(), end.xz()) else {
        return;
    };

    let mesh_to_world: GlobalTransform =
        Transform::from_rotation(Quat::from_rotation_x(FRAC_PI_2)).into();

    let point_as_vec3 = |point: Vec2| {
        let coords = mesh.get_point_layer(point)[0];
        coords.position_with_height(mesh)
    };

    let mut heighted_path = vec![];

    let mut current = start;
    let mut next_i = 0;
    let mut next_coords = mesh.get_point_layer(path.path[next_i])[0];
    let mut next = next_coords.position_with_height(mesh);
    for polygon_index in &path.path_through_polygons {
        let layer = &mesh.layers[polygon_index.layer() as usize];
        display_polygon_gizmo(
            layer,
            polygon_index.polygon(),
            &mesh_to_world,
            palettes::tailwind::BLUE_500.into(),
            &mut gizmos,
        );

        let polygon = &layer.polygons[polygon_index.polygon() as usize];
        if polygon.contains(layer, next_coords.position()) {
            next_i += 1;
            if next_i < path.path.len() - 1 {
                path_gizmos.sphere(next, 0.1, palettes::tailwind::BLUE_400);
                heighted_path.push(next);
                current = next;
                next_coords = mesh.get_point_layer(path.path[next_i])[0];
                next = next_coords.position_with_height(mesh);
            }
        }
        let a = point_as_vec3(layer.vertices[polygon.vertices[0] as usize].coords);
        let b = point_as_vec3(layer.vertices[polygon.vertices[1] as usize].coords);
        let c = point_as_vec3(layer.vertices[polygon.vertices[2] as usize].coords);
        let line = next - current;
        let normal = Plane3d::from_points(a, b, c).0.normal;
        if line.dot(normal.as_vec3()).abs() > 0.00001 {
            let poly_coords = polygon.coords(layer);
            let closing = vec![
                poly_coords.last().unwrap().clone(),
                poly_coords.first().unwrap().clone(),
            ];

            if let Some(new) = poly_coords
                .windows(2)
                .chain([closing.as_slice()])
                .filter_map(|edge| {
                    polyanya::line_intersect_segment((current.xz(), next.xz()), (edge[0], edge[1]))
                })
                .filter(|p| p.on_segment((current.xz(), next.xz())))
                .max_by_key(|p| (current.xz().distance_squared(*p) * 10000.0) as u32)
            {
                let new = point_as_vec3(new);
                path_gizmos.sphere(new, 0.1, palettes::tailwind::RED_400);

                heighted_path.push(new);
                current = new;
            };
        }
    }

    heighted_path.insert(0, start);
    heighted_path.push(end);
    path_gizmos.linestrip(heighted_path, palettes::tailwind::LIME_600);
}
