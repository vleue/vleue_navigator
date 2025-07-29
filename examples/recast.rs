use std::f32::consts::FRAC_PI_2;

use bevy::{color::palettes, prelude::*};
use vleue_navigator::{
    VleueNavigatorPlugin, display_mesh_gizmo, display_polygon_gizmo, prelude::*,
};

#[path = "helpers/camera_controller.rs"]
mod camera_controller;

fn main() {
    let rasterised_mesh = polyanya::RecastPolyMesh::from_file("assets/recast/poly_mesh.json");
    let detailed_mesh = polyanya::RecastPolyMeshDetail::from_file("assets/recast/detail_mesh.json");
    let full_navmesh: polyanya::Mesh =
        polyanya::RecastFullMesh::new(rasterised_mesh, detailed_mesh).into();

    App::new()
        .insert_resource(ClearColor(Color::srgb(0., 0., 0.01)))
        .add_plugins((
            DefaultPlugins,
            camera_controller::CameraControllerPlugin,
            VleueNavigatorPlugin,
            NavmeshUpdaterPlugin::<PrimitiveObstacle>::default(),
        ))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (draw_navmesh, draw_path, layers_info, switch_layers),
        )
        .insert_resource(Layers(vec![true; full_navmesh.layers.len()]))
        .insert_resource(RecastNavmesh(full_navmesh))
        .insert_gizmo_config::<NavMeshGizmos>(
            NavMeshGizmos,
            GizmoConfig {
                line: GizmoLineConfig {
                    style: GizmoLineStyle::Dashed {
                        gap_scale: 1.0,
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

const LAYER_COLORS: [Srgba; 5] = [
    palettes::tailwind::BLUE_600,
    palettes::tailwind::RED_600,
    palettes::tailwind::GREEN_600,
    palettes::tailwind::YELLOW_600,
    palettes::tailwind::FUCHSIA_600,
];

#[derive(Default, Reflect, GizmoConfigGroup)]
struct NavMeshGizmos;
#[derive(Default, Reflect, GizmoConfigGroup)]
struct PathGizmos;

#[derive(Resource)]
struct RecastNavmesh(polyanya::Mesh);

#[derive(Resource)]
struct Layers(Vec<bool>);

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
    let colors: Vec<_> = LAYER_COLORS
        .iter()
        .map(|color| color.clone().into())
        .collect();
    display_mesh_gizmo(
        mesh,
        &Transform::from_rotation(Quat::from_rotation_x(FRAC_PI_2)).into(),
        colors.as_slice(),
        &mut navmesh_gizmos,
    );
}

fn draw_path(
    mut path_gizmos: Gizmos<PathGizmos>,
    mut gizmos: Gizmos,
    recast: Res<RecastNavmesh>,
    layers: Res<Layers>,
) {
    let start = vec3(46.998413, 9.998184, 1.717747);
    let end = vec3(20.703018, 18.651773, -80.770203);

    gizmos.sphere(start, 0.5, palettes::tailwind::LIME_400);
    gizmos.sphere(end, 0.5, palettes::tailwind::YELLOW_400);

    let mesh = &recast.0;
    // let Some(path) = mesh.path(start.xz(), end.xz()) else {
    //     return;
    // };
    let Some(path) = mesh.path_on_layers(
        start.xz(),
        end.xz(),
        layers
            .0
            .iter()
            .enumerate()
            .filter_map(|(n, e)| (!e).then_some(n as u8))
            .collect(),
    ) else {
        return;
    };

    // for (layer, polygon) in path.polygons() {
    //     let layer = &mesh.layers[layer as usize];
    //     display_polygon_gizmo(
    //         layer,
    //         polygon,
    //         &Transform::from_rotation(Quat::from_rotation_x(FRAC_PI_2)).into(),
    //         palettes::tailwind::BLUE_500.into(),
    //         &mut gizmos,
    //     );
    // }

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

fn layers_info(mut commands: Commands, layers: Res<Layers>, texts: Query<Entity, With<Text>>) {
    if let Ok(entity) = texts.single() {
        commands.entity(entity).despawn();
    }
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
                font_size: 15.0,
                ..default()
            };
            for layer in layers.0.iter().enumerate() {
                let color = LAYER_COLORS[layer.0].clone().into();
                p.spawn((
                    TextSpan::new(format!("Layer {}: ", layer.0)),
                    TextColor(color),
                    font_size.clone(),
                ));
                if *layer.1 {
                    p.spawn((
                        TextSpan::new(format!("{}\n", layer.1)),
                        TextColor(palettes::css::GREEN.into()),
                        font_size.clone(),
                    ));
                } else {
                    p.spawn((
                        TextSpan::new(format!("{}\n", layer.1)),
                        TextColor(palettes::css::RED.into()),
                        font_size.clone(),
                    ));
                }
            }
        });
}

fn switch_layers(mut layers: ResMut<Layers>, input: Res<ButtonInput<KeyCode>>) {
    for (index, key) in [
        KeyCode::Digit0,
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
        KeyCode::Digit5,
        KeyCode::Digit6,
    ]
    .iter()
    .enumerate()
    {
        if input.just_pressed(*key) {
            if let Some(layer) = layers.0.get_mut(index) {
                *layer = !*layer;
            }
        }
    }
}
