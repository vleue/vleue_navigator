use std::f32::consts::FRAC_PI_2;

use bevy::{color::palettes, prelude::*};
use vleue_navigator::{
    VleueNavigatorPlugin, display_mesh_gizmo, display_polygon_gizmo, prelude::*,
};

#[path = "helpers/camera_controller.rs"]
mod camera_controller;

fn main() {
    // let navmesh = polyanya::RecastPolyMesh::from_file("assets/recast/poly_mesh.json").into();
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
        .add_systems(Update, (draw_navmesh, draw_path))
        .insert_resource(RecastNavmesh(detailed_navmesh))
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
                depth_bias: -1.0,
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
struct RecastNavmesh(polyanya::Mesh);

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

fn draw_navmesh(mut navmesh_gizmos: Gizmos<NavMeshGizmos>, recast: Res<RecastNavmesh>) {
    let mesh = &recast.0;
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
}

fn draw_path(mut path_gizmos: Gizmos<PathGizmos>, mut gizmos: Gizmos, recast: Res<RecastNavmesh>) {
    let start = vec3(46.998413, 9.998184, 1.717747);
    let end = vec3(20.703018, 18.651773, -80.770203);

    gizmos.sphere(start, 0.5, palettes::tailwind::LIME_400);
    gizmos.sphere(end, 0.5, palettes::tailwind::YELLOW_400);

    let mesh = &recast.0;
    let Some(path) = mesh.path(start.xz(), end.xz()) else {
        return;
    };

    for (layer, polygon) in path.polygons() {
        let layer = &mesh.layers[layer as usize];
        display_polygon_gizmo(
            layer,
            polygon,
            &Transform::from_rotation(Quat::from_rotation_x(FRAC_PI_2)).into(),
            palettes::tailwind::BLUE_500.into(),
            &mut gizmos,
        );
    }

    let mut path_with_height = path.path_with_height(start, end, mesh);

    for point in &path_with_height {
        if path.path.contains(&point.xz()) {
            // This point is on the original path
            path_gizmos.sphere(*point, 0.1, palettes::tailwind::BLUE_600);
        } else {
            // This point was added to follow the terrain height
            path_gizmos.sphere(*point, 0.1, palettes::tailwind::RED_600);
        }
    }
    path_with_height.insert(0, start);
    path_gizmos.linestrip(path_with_height, palettes::tailwind::LIME_600);
}
