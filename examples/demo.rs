use std::f32::consts::{FRAC_PI_2, PI};

use bevy::{
    color::palettes, ecs::entity::EntityHashSet, math::vec2, prelude::*, render::view::RenderLayers,
};
use polyanya::Triangulation;
use rand::Rng;
use vleue_navigator::prelude::*;

#[path = "helpers/agent3d.rs"]
mod agent3d;
#[path = "helpers/ui.rs"]
mod ui;

const MESH_WIDTH: u32 = 150;
const MESH_HEIGHT: u32 = 100;

pub const MATERIAL_OBSTACLE_1: Handle<StandardMaterial> = Handle::weak_from_u128(0);
pub const MATERIAL_OBSTACLE_2: Handle<StandardMaterial> = Handle::weak_from_u128(1);
pub const MATERIAL_OBSTACLE_3: Handle<StandardMaterial> = Handle::weak_from_u128(2);
pub const MATERIAL_OBSTACLE_CACHED_1: Handle<StandardMaterial> = Handle::weak_from_u128(10);
pub const MATERIAL_OBSTACLE_CACHED_2: Handle<StandardMaterial> = Handle::weak_from_u128(11);
pub const MATERIAL_OBSTACLE_CACHED_3: Handle<StandardMaterial> = Handle::weak_from_u128(12);
pub const MATERIAL_NAVMESH: Handle<StandardMaterial> = Handle::weak_from_u128(3);

#[derive(Component, Debug)]
struct Obstacle;

fn main() {
    App::new()
        .insert_resource(ClearColor(palettes::css::BLACK.into()))
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
            NavmeshUpdaterPlugin::<PrimitiveObstacle>::default(),
        ))
        .add_systems(
            Startup,
            (
                setup,
                ui::setup_stats::<false>,
                ui::setup_settings::<false>,
                agent3d::setup_agent::<100>,
            ),
        )
        .add_systems(
            Update,
            (
                display_mesh,
                ui::update_stats::<PrimitiveObstacle>,
                remove_obstacles,
                ui::display_settings,
                ui::update_settings::<10>,
                agent3d::give_target_to_navigator::<MESH_WIDTH, MESH_HEIGHT>,
                agent3d::move_navigator::<100>,
                agent3d::display_navigator_path.after(agent3d::move_navigator::<100>),
                agent3d::refresh_path::<MESH_WIDTH, MESH_HEIGHT>,
                life_of_obstacle,
                ui::toggle_ui,
                toggle_ui,
                pause,
                cached_material,
            ),
        )
        .add_systems(FixedUpdate, random_obstacle)
        .insert_resource(Time::<Fixed>::from_seconds(0.25))
        .insert_resource(NavMeshesDebug(palettes::tailwind::RED_800.into()))
        .run();
}

fn pause(keyboard_input: Res<ButtonInput<KeyCode>>, mut virtual_time: ResMut<Time<Virtual>>) {
    if keyboard_input.just_pressed(KeyCode::Space) {
        if virtual_time.is_paused() {
            virtual_time.unpause();
        } else {
            virtual_time.pause();
        }
    }
}

fn cached_material(
    obstacles: Query<(&Children, Option<Ref<CachableObstacle>>)>,
    mut materials: Query<&mut Handle<StandardMaterial>>,
    mut removed: RemovedComponents<CachableObstacle>,
) {
    for (children, cachable) in &obstacles {
        if cachable.is_some() && cachable.unwrap().is_added() {
            let mut material = materials.get_mut(children[0]).unwrap();
            *material = match rand::thread_rng().gen_range(0..3) {
                0 => MATERIAL_OBSTACLE_CACHED_1,
                1 => MATERIAL_OBSTACLE_CACHED_2,
                2 => MATERIAL_OBSTACLE_CACHED_3,
                _ => unreachable!(),
            };
        }
    }
    for removed in removed.read() {
        let (children, _) = obstacles.get(removed).unwrap();
        let mut material = materials.get_mut(children[0]).unwrap();
        *material = match rand::thread_rng().gen_range(0..3) {
            0 => MATERIAL_OBSTACLE_1,
            1 => MATERIAL_OBSTACLE_2,
            2 => MATERIAL_OBSTACLE_3,
            _ => unreachable!(),
        };
    }
}

#[derive(Component)]
struct Lifetime(Timer);

fn life_of_obstacle(
    mut commands: Commands,
    time: Res<Time<Virtual>>,
    mut obstacles: Query<(Entity, &mut Lifetime, &mut Transform)>,
    mut cachable: Local<EntityHashSet>,
    example_settings: Res<ui::ExampleSettings>,
) {
    for (entity, mut lifetime, mut transform) in obstacles.iter_mut() {
        // reset cache when settings change
        if example_settings.is_changed() && !example_settings.cache_enabled {
            cachable.remove(&entity);
            commands.entity(entity).remove::<CachableObstacle>();
        }
        lifetime.0.tick(time.delta());

        if lifetime.0.finished() {
            commands.entity(entity).despawn_recursive();
        } else if lifetime.0.fraction() < 0.2 {
            transform.scale = Vec3::new(
                lifetime.0.fraction() * 4.0,
                1.0,
                lifetime.0.fraction() * 4.0,
            );
        } else if lifetime.0.fraction() > 0.8 {
            transform.scale = Vec3::new(
                (-lifetime.0.fraction() + 1.0) * 4.0 + 0.01,
                1.0,
                (-lifetime.0.fraction() + 1.0) * 4.0 + 0.01,
            );
            if cachable.remove(&entity) {
                commands.entity(entity).remove::<CachableObstacle>();
            }
        } else {
            if example_settings.cache_enabled {
                if cachable.insert(entity) {
                    commands.entity(entity).insert(CachableObstacle);
                }
            }
        }
    }
}

fn random_obstacle(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    let mut rng = rand::thread_rng();
    let mat = match rand::thread_rng().gen_range(0..3) {
        0 => MATERIAL_OBSTACLE_1,
        1 => MATERIAL_OBSTACLE_2,
        2 => MATERIAL_OBSTACLE_3,
        _ => unreachable!(),
    };
    let transform = Transform::from_translation(Vec3::new(
        rng.gen_range(0.0..(MESH_WIDTH as f32)),
        0.0,
        rng.gen_range(0.0..(MESH_HEIGHT as f32)),
    ))
    .with_rotation(Quat::from_rotation_y(rng.gen_range(0.0..PI)))
    .with_scale(Vec3::splat(0.0));
    new_obstacle(&mut commands, &mut rng, transform, &mut meshes, &mat);
}

fn setup(mut commands: Commands, mut materials: ResMut<Assets<StandardMaterial>>) {
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(
                MESH_WIDTH as f32 / 2.0,
                MESH_WIDTH.min(MESH_HEIGHT) as f32,
                -1.0,
            )
            .looking_at(
                Vec3::new(
                    MESH_WIDTH as f32 / 2.0,
                    0.0,
                    MESH_HEIGHT as f32 / 2.0 - MESH_HEIGHT as f32 / 12.0,
                ),
                Vec3::Y,
            ),
            ..Default::default()
        },
        RenderLayers::default().with(1),
    ));
    // light
    for (x, y) in [(0.25, 0.25), (0.75, 0.25), (0.25, 0.75), (0.75, 0.75)] {
        commands.spawn(PointLightBundle {
            point_light: PointLight {
                shadows_enabled: true,
                intensity: MESH_WIDTH.min(MESH_HEIGHT) as f32 * 300_000.0,
                range: MESH_WIDTH.min(MESH_HEIGHT) as f32 * 10.0,
                ..default()
            },
            transform: Transform::from_xyz(
                MESH_WIDTH as f32 * x,
                MESH_WIDTH.min(MESH_HEIGHT) as f32 / 3.0,
                MESH_HEIGHT as f32 * y,
            ),
            ..default()
        });
    }

    // Spawn a new navmesh that will be automatically updated.
    commands.spawn(NavMeshBundle {
        settings: NavMeshSettings {
            // Define the outer borders of the navmesh.
            fixed: Triangulation::from_outer_edges(&[
                vec2(0.0, 0.0),
                vec2(MESH_WIDTH as f32, 0.0),
                vec2(MESH_WIDTH as f32, MESH_HEIGHT as f32),
                vec2(0.0, MESH_HEIGHT as f32),
            ]),
            simplify: 0.1,
            merge_steps: 1,
            build_timeout: Some(1.0),
            agent_radius: 1.0,
            ..default()
        },
        transform: Transform::from_rotation(Quat::from_rotation_x(FRAC_PI_2)),
        // Mark it for update as soon as obstacles are changed.
        // Other modes can be debounced or manually triggered.
        update_mode: NavMeshUpdateMode::Direct,
        ..NavMeshBundle::with_default_id()
    });

    materials.insert(
        &MATERIAL_OBSTACLE_1,
        StandardMaterial::from(Color::Srgba(palettes::tailwind::RED_600)),
    );
    materials.insert(
        &MATERIAL_OBSTACLE_2,
        StandardMaterial::from(Color::Srgba(palettes::tailwind::RED_700)),
    );
    materials.insert(
        &MATERIAL_OBSTACLE_3,
        StandardMaterial::from(Color::Srgba(palettes::tailwind::ORANGE_700)),
    );
    materials.insert(
        &MATERIAL_OBSTACLE_CACHED_1,
        StandardMaterial::from(Color::Srgba(palettes::tailwind::GREEN_600)),
    );
    materials.insert(
        &MATERIAL_OBSTACLE_CACHED_2,
        StandardMaterial::from(Color::Srgba(palettes::tailwind::GREEN_700)),
    );
    materials.insert(
        &MATERIAL_OBSTACLE_CACHED_3,
        StandardMaterial::from(Color::Srgba(palettes::tailwind::TEAL_700)),
    );
    materials.insert(
        &MATERIAL_NAVMESH,
        StandardMaterial::from(Color::Srgba(palettes::tailwind::BLUE_800)),
    );
}

fn new_obstacle(
    commands: &mut Commands,
    rng: &mut impl Rng,
    transform: Transform,
    meshes: &mut Assets<Mesh>,
    mat: &Handle<StandardMaterial>,
) {
    match rng.gen_range(0..8) {
        0 => {
            let primitive = Rectangle {
                half_size: vec2(rng.gen_range(1.0..5.0), rng.gen_range(1.0..5.0)),
            };
            commands
                .spawn((
                    SpatialBundle::from_transform(transform),
                    PrimitiveObstacle::Rectangle(primitive),
                    Lifetime(Timer::from_seconds(
                        rng.gen_range(20.0..40.0),
                        TimerMode::Once,
                    )),
                ))
                .with_children(|parent| {
                    parent.spawn(PbrBundle {
                        mesh: meshes.add(Extrusion::new(primitive, rng.gen_range(5.0..15.0))),
                        material: mat.clone(),
                        transform: Transform::from_rotation(Quat::from_rotation_x(FRAC_PI_2)),
                        ..default()
                    });
                });
        }
        1 => {
            let primitive = Circle {
                radius: rng.gen_range(1.0..5.0),
            };
            commands
                .spawn((
                    SpatialBundle::from_transform(transform),
                    PrimitiveObstacle::Circle(primitive),
                    Lifetime(Timer::from_seconds(
                        rng.gen_range(20.0..40.0),
                        TimerMode::Once,
                    )),
                ))
                .with_children(|parent| {
                    parent.spawn(PbrBundle {
                        mesh: meshes.add(Extrusion::new(primitive, rng.gen_range(5.0..15.0))),
                        material: mat.clone(),
                        transform: Transform::from_rotation(Quat::from_rotation_x(FRAC_PI_2)),
                        ..default()
                    });
                });
        }
        2 => {
            let primitive = Ellipse {
                half_size: vec2(rng.gen_range(1.0..5.0), rng.gen_range(1.0..5.0)),
            };
            commands
                .spawn((
                    SpatialBundle::from_transform(transform),
                    PrimitiveObstacle::Ellipse(primitive),
                    Lifetime(Timer::from_seconds(
                        rng.gen_range(20.0..40.0),
                        TimerMode::Once,
                    )),
                ))
                .with_children(|parent| {
                    parent.spawn(PbrBundle {
                        mesh: meshes.add(Extrusion::new(primitive, rng.gen_range(5.0..15.0))),
                        material: mat.clone(),
                        transform: Transform::from_rotation(Quat::from_rotation_x(FRAC_PI_2)),
                        ..default()
                    });
                });
        }
        3 => {
            let primitive = Capsule2d::new(rng.gen_range(1.0..3.0), rng.gen_range(1.5..5.0));
            commands
                .spawn((
                    SpatialBundle::from_transform(transform),
                    PrimitiveObstacle::Capsule(primitive),
                    Lifetime(Timer::from_seconds(
                        rng.gen_range(20.0..40.0),
                        TimerMode::Once,
                    )),
                ))
                .with_children(|parent| {
                    parent.spawn(PbrBundle {
                        mesh: meshes.add(Extrusion::new(primitive, rng.gen_range(5.0..15.0))),
                        material: mat.clone(),
                        transform: Transform::from_rotation(Quat::from_rotation_x(FRAC_PI_2)),
                        ..default()
                    });
                });
        }
        4 => {
            let primitive = RegularPolygon::new(rng.gen_range(1.0..5.0), rng.gen_range(3..11));
            commands
                .spawn((
                    SpatialBundle::from_transform(transform),
                    PrimitiveObstacle::RegularPolygon(primitive),
                    Lifetime(Timer::from_seconds(
                        rng.gen_range(20.0..40.0),
                        TimerMode::Once,
                    )),
                ))
                .with_children(|parent| {
                    parent.spawn(PbrBundle {
                        mesh: meshes.add(Extrusion::new(primitive, rng.gen_range(5.0..15.0))),
                        material: mat.clone(),
                        transform: Transform::from_rotation(Quat::from_rotation_x(FRAC_PI_2)),
                        ..default()
                    });
                });
        }
        5 => {
            let primitive = Rhombus::new(rng.gen_range(3.0..6.0), rng.gen_range(2.0..3.0));
            commands
                .spawn((
                    SpatialBundle::from_transform(transform),
                    PrimitiveObstacle::Rhombus(primitive),
                    Lifetime(Timer::from_seconds(
                        rng.gen_range(20.0..40.0),
                        TimerMode::Once,
                    )),
                ))
                .with_children(|parent| {
                    parent.spawn(PbrBundle {
                        mesh: meshes.add(Extrusion::new(primitive, rng.gen_range(5.0..15.0))),
                        material: mat.clone(),
                        transform: Transform::from_rotation(Quat::from_rotation_x(FRAC_PI_2)),
                        ..default()
                    });
                });
        }
        6 => {
            let primitive =
                CircularSector::new(rng.gen_range(1.5..5.0), rng.gen_range(0.5..FRAC_PI_2));
            commands
                .spawn((
                    SpatialBundle::from_transform(transform),
                    PrimitiveObstacle::CircularSector(primitive),
                    Lifetime(Timer::from_seconds(
                        rng.gen_range(20.0..40.0),
                        TimerMode::Once,
                    )),
                ))
                .with_children(|parent| {
                    parent.spawn(PbrBundle {
                        mesh: meshes.add(Extrusion::new(primitive, rng.gen_range(5.0..15.0))),
                        material: mat.clone(),
                        transform: Transform::from_rotation(Quat::from_rotation_x(FRAC_PI_2)),
                        ..default()
                    });
                });
        }
        7 => {
            let primitive = CircularSegment::new(rng.gen_range(1.5..5.0), rng.gen_range(1.0..PI));
            commands
                .spawn((
                    SpatialBundle::from_transform(transform),
                    PrimitiveObstacle::CircularSegment(primitive),
                    Lifetime(Timer::from_seconds(
                        rng.gen_range(20.0..40.0),
                        TimerMode::Once,
                    )),
                ))
                .with_children(|parent| {
                    parent.spawn(PbrBundle {
                        mesh: meshes.add(Extrusion::new(primitive, rng.gen_range(5.0..15.0))),
                        material: mat.clone(),
                        transform: Transform::from_rotation(Quat::from_rotation_x(FRAC_PI_2)),
                        ..default()
                    });
                });
        }
        _ => unreachable!(),
    }
}

fn display_mesh(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    navmeshes: Res<Assets<NavMesh>>,
    mut current_mesh_entity: Local<Option<Entity>>,
    navmesh: Query<(&Handle<NavMesh>, Ref<NavMeshStatus>)>,
) {
    let (navmesh_handle, status) = navmesh.single();
    if !status.is_changed() || *status != NavMeshStatus::Built {
        return;
    }

    if navmeshes.get(navmesh_handle).is_none() {
        return;
    };
    if let Some(entity) = *current_mesh_entity {
        commands.entity(entity).despawn_recursive();
    }

    *current_mesh_entity = Some(
        commands
            .spawn(PbrBundle {
                mesh: meshes.add(Plane3d::new(
                    Vec3::Y,
                    Vec2::new(MESH_WIDTH as f32 / 2.0, MESH_HEIGHT as f32 / 2.0),
                )),
                transform: Transform::from_translation(Vec3::new(
                    (MESH_WIDTH as f32) / 2.0,
                    0.0,
                    MESH_HEIGHT as f32 / 2.0,
                )),
                material: MATERIAL_NAVMESH,
                ..default()
            })
            .id(),
    );
}

fn remove_obstacles(
    obstacles: Query<Entity, With<Obstacle>>,
    mut commands: Commands,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    if keyboard_input.just_pressed(KeyCode::Space) {
        for entity in obstacles.iter() {
            commands.entity(entity).despawn();
        }
    }
}

fn toggle_ui(
    mut layers: Query<&mut RenderLayers, With<Camera>>,
    mut entered: EventReader<CursorEntered>,
    mut left: EventReader<CursorLeft>,
) {
    for _ in entered.read() {
        for mut layers in &mut layers {
            *layers = layers.clone().with(1);
        }
    }
    for _ in left.read() {
        for mut layers in &mut layers {
            *layers = layers.clone().without(1);
        }
    }
}
