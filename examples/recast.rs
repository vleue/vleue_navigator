use std::{f32::consts::FRAC_PI_2, fs::File};

use bevy::{
    color::palettes::{self},
    core_pipeline::tonemapping::Tonemapping,
    post_process::bloom::Bloom,
    prelude::*,
    render::view::Hdr,
};
use vleue_navigator::{VleueNavigatorPlugin, display_layer_gizmo, prelude::*};

#[path = "helpers/camera_controller.rs"]
mod camera_controller;

fn main() {
    let rasterised: polyanya::RecastPolyMesh =
        serde_json::from_reader(File::open("assets/recast/poly_mesh.json").unwrap()).unwrap();
    let detailed: polyanya::RecastPolyMeshDetail =
        serde_json::from_reader(File::open("assets/recast/detail_mesh.json").unwrap()).unwrap();
    let mut full_navmesh: polyanya::Mesh =
        polyanya::RecastFullMesh::new(rasterised, detailed).into();

    full_navmesh.search_delta = 0.25;
    full_navmesh.search_steps = 20;

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
            (
                draw_navmesh,
                draw_path,
                layers_info,
                switch_layers,
                autonomous_demo,
            ),
        )
        .insert_resource(Layers(vec![true; full_navmesh.layers.len()]))
        .insert_resource(RecastNavmesh(full_navmesh))
        .insert_resource(GlobalAmbientLight {
            brightness: 200.0,
            ..default()
        })
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
        Camera::default(),
        Hdr,
        Transform::from_xyz(35.0, 90.0, -40.0).looking_at(Vec3::new(15.0, 0.0, -40.0), Vec3::Y),
        camera_controller::CameraController::default(),
        Tonemapping::TonyMcMapface,
        Bloom::NATURAL,
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

fn draw_navmesh(
    mut navmesh_gizmos: Gizmos<NavMeshGizmos>,
    recast: Res<RecastNavmesh>,
    layers: Res<Layers>,
) {
    let mesh = &recast.0;
    let colors: Vec<Color> = LAYER_COLORS
        .iter()
        .map(|color| color.clone().into())
        .collect();
    for (layer, (color, enabled)) in mesh.layers.iter().zip(colors.iter().zip(layers.0.iter())) {
        let mut color = *color;
        if !enabled {
            color.set_alpha(0.2);
        }
        display_layer_gizmo(
            layer,
            &Transform::from_rotation(Quat::from_rotation_x(FRAC_PI_2)).into(),
            color,
            &mut navmesh_gizmos,
        );
    }
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

    let mut path_with_height = path.path_with_height(start, end, mesh);

    for point in &path_with_height {
        path_gizmos.sphere(*point, 0.1, palettes::tailwind::BLUE_600);
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

fn autonomous_demo(
    mut camera_transform: Single<&mut Transform, (Without<Agent>, With<Camera>)>,
    input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut current_position: Local<(Option<Timer>, u32)>,
    mut moving: Local<bool>,
    mut started: Local<bool>,
    mut finished: Local<bool>,
    mut bloomed: Local<bool>,
    mut timer: Local<Option<Timer>>,
    mut layers: ResMut<Layers>,
    mut commands: Commands,
    recast: Res<RecastNavmesh>,
    mut spawn_timer: Local<(Option<Timer>, f32)>,
    mut agents: Query<(Entity, &mut Transform, &MeshMaterial3d<StandardMaterial>), With<Agent>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let positions = [
        vec3(35.0, 90.0, -40.0),
        vec3(40.0, 20.0, -60.0),
        vec3(20.0, 30.0, -15.0),
        vec3(35.0, 50.0, -15.0),
    ];
    let rotations = [
        Transform::from_translation(positions[0])
            .looking_at(Vec3::new(15.0, 0.0, -40.0), Vec3::Y)
            .rotation,
        Transform::from_translation(positions[1])
            .looking_at(Vec3::new(15.0, 10.0, -70.0), Vec3::Y)
            .rotation,
        Transform::from_translation(positions[2])
            .looking_at(Vec3::new(10.0, 10.0, -25.0), Vec3::Y)
            .rotation,
        Transform::from_translation(positions[3])
            .looking_at(Vec3::new(40.0, 10.0, 0.0), Vec3::Y)
            .rotation,
    ];

    if current_position.0.is_none() {
        current_position.0 = Some(Timer::from_seconds(10.0, TimerMode::Repeating));
    }

    if camera_transform
        .translation
        .distance_squared(positions[current_position.1 as usize])
        < 1.0
    {
        *moving = false;
    }

    if input.just_pressed(KeyCode::Space)
        || (current_position
            .0
            .as_mut()
            .unwrap()
            .tick(time.delta())
            .just_finished()
            && !*finished)
    {
        current_position.1 = (current_position.1 + 1) % positions.len() as u32;
        *moving = true;
        if current_position.1 == 2 {
            *started = true;
        }
        if current_position.1 == 3 {
            *bloomed = true;
        }
        if *started && current_position.1 == 0 {
            *finished = true;
            spawn_timer.0 = Some(Timer::from_seconds(1.5, TimerMode::Repeating));
        }
    }

    if *moving {
        camera_transform.translation.smooth_nudge(
            &positions[current_position.1 as usize],
            2.0,
            time.delta_secs(),
        );
        camera_transform.rotation.smooth_nudge(
            &rotations[current_position.1 as usize],
            2.0,
            time.delta_secs(),
        );
    }

    if *started {
        if timer.is_none() {
            *timer = Some(Timer::from_seconds(0.87, TimerMode::Repeating));
        }
        use rand::seq::IndexedRandom;

        if timer.as_mut().unwrap().tick(time.delta()).just_finished() {
            let layer = match current_position.1 {
                0 => *[1, 4].choose(&mut rand::rng()).unwrap(),
                1 => 4,
                2 => 4,
                3 => 1,
                _ => unreachable!(),
            };
            layers.0[layer as usize] = !layers.0[layer as usize];
        }
    }
    let start = vec3(46.998413, 9.998184, 1.717747);
    let end = vec3(20.703018, 18.651773, -80.770203);

    if spawn_timer.0.is_none() {
        let mut timer = Timer::from_seconds(2.5, TimerMode::Repeating);
        timer.set_elapsed(std::time::Duration::from_secs(2));
        spawn_timer.0 = Some(timer);
    }

    if spawn_timer
        .0
        .as_mut()
        .unwrap()
        .tick(time.delta())
        .just_finished()
    {
        let sphere = meshes.add(Sphere::new(1.0).mesh());

        commands.spawn((
            Mesh3d(sphere.clone()),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: palettes::tailwind::LIME_400.into(),
                emissive_exposure_weight: 0.0,
                ..default()
            })),
            Transform::from_translation(start),
            Agent,
        ));
    }

    let mesh = &recast.0;
    let layers: std::collections::HashSet<u8> = layers
        .0
        .iter()
        .enumerate()
        .filter_map(|(n, e)| (!e).then_some(n as u8))
        .collect();
    for (entity, mut transform, material) in &mut agents {
        let Some(path) = mesh.path_on_layers(transform.translation.xz(), end.xz(), layers.clone())
        else {
            continue;
        };
        if *bloomed {
            let next_polygon_in_path = path.polygons().first().unwrap().0;
            let material = materials.get_mut(material.id());
            let material = material.unwrap();
            if next_polygon_in_path == 2 {
                material.emissive = Srgba::new(0.6, 10.0, 0.2, 1.0).into();
            } else {
                material.emissive = palettes::css::BLACK.into();
            }
            if next_polygon_in_path != 0 {
                material.base_color = LAYER_COLORS[next_polygon_in_path as usize].into();
            }
        }
        let move_dir = (path.path_with_height(transform.translation, end, mesh)[0]
            - transform.translation)
            .normalize();
        transform.translation += move_dir / 10.0;
        if *finished {
            transform.translation += move_dir / 15.0;
        }
        if transform.translation.distance_squared(end) < 0.1 {
            commands.entity(entity).despawn();
        }
    }
}

#[derive(Component)]
struct Agent;
