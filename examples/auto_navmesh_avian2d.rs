use avian2d::{math::*, prelude::*};
use bevy::{color::palettes, math::vec2, prelude::*};
use rand::Rng;
use vleue_navigator::prelude::*;

#[derive(Component)]
enum Obstacle {
    Peg,
    Wall,
}

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Navmesh with Polyanya".to_string(),
                    fit_canvas_to_parent: true,
                    ..default()
                }),
                ..default()
            }),
            PhysicsPlugins::default().with_length_unit(20.0),
            VleueNavigatorPlugin,
            NavmeshUpdaterPlugin::<Collider, Obstacle>::default(),
        ))
        .insert_resource(ClearColor(Color::srgb(0.05, 0.05, 0.1)))
        .insert_resource(Gravity(Vector::NEG_Y * 9.81 * 100.0))
        .add_systems(Startup, setup)
        .add_systems(
            PreUpdate,
            (puck_back_to_start, move_puck, display_puck_path),
        )
        .insert_resource(NavMeshesDebug(palettes::tailwind::RED_800.into()))
        .run();
}

fn setup(
    mut commands: Commands,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    commands.spawn(Camera2d);

    let square_sprite = Sprite {
        color: Color::srgb(0.7, 0.7, 0.8),
        custom_size: Some(Vec2::splat(50.0)),
        ..default()
    };

    // Left wall
    commands.spawn((
        square_sprite.clone(),
        Transform::from_xyz(-50.0 * 9.5, 0.0, 0.0).with_scale(Vec3::new(1.0, 15.0, 1.0)),
        RigidBody::Static,
        Collider::rectangle(50.0, 50.0),
        Obstacle::Wall,
    ));
    // Right wall
    commands.spawn((
        square_sprite,
        Transform::from_xyz(50.0 * 9.5, 0.0, 0.0).with_scale(Vec3::new(1.0, 15.0, 1.0)),
        RigidBody::Static,
        Collider::rectangle(50.0, 50.0),
        Obstacle::Wall,
    ));

    let peg_radius = 15.0;
    let step = 10;
    let peg_mesh = meshes.add(Circle::new(peg_radius));
    let peg_material = materials.add(Color::srgb(0.2, 0.7, 0.9));

    for x in (-50..50).step_by(step).skip(1) {
        for (yi, y) in (-50..50).step_by(step).skip(1).enumerate() {
            commands.spawn((
                Mesh2d(peg_mesh.clone().into()),
                MeshMaterial2d(peg_material.clone()),
                Transform::from_xyz(
                    (x as f32
                        + if yi % 2 == 0 {
                            -(step as f32 / 4.0)
                        } else {
                            step as f32 / 4.0
                        }
                        + rand::thread_rng().gen_range(-1.0..1.0))
                        * 9.5,
                    (y as f32 + rand::thread_rng().gen_range(-1.0..1.0)) * 6.0,
                    0.0,
                ),
                RigidBody::Static,
                Collider::circle(peg_radius as Scalar),
                Obstacle::Peg,
            ));
        }
    }

    let marble_mesh = meshes.add(Circle::new(5.0));
    let marble_material = materials.add(Color::srgb(0.7, 0.9, 0.2));
    for x in (-50..50).step_by(5).skip(1) {
        let start = Vec3::new(x as f32 * 9.5, 300.0, 0.0);
        commands.spawn((
            Mesh2d(marble_mesh.clone().into()),
            MeshMaterial2d(marble_material.clone()),
            Transform::from_translation(start),
            RigidBody::Dynamic,
            LinearVelocity(
                Vec2::new(
                    rand::thread_rng().gen_range(-1.0..1.0),
                    rand::thread_rng().gen_range(-1.0..1.0),
                )
                .normalize()
                    * 200.0,
            ),
            Collider::circle(5.0 as Scalar),
            Puck(start),
        ));
    }

    commands.spawn((
        NavMeshSettings {
            // Define the outer borders of the navmesh.
            fixed: Triangulation::from_outer_edges(&[
                vec2(-500.0, -500.0),
                vec2(500.0, -500.0),
                vec2(500.0, 500.0),
                vec2(-500.0, 500.0),
            ]),
            agent_radius: 5.0,
            simplify: 4.0,
            merge_steps: 1,
            ..default()
        },
        NavMeshUpdateMode::Direct,
    ));
}

#[derive(Component)]
struct Puck(Vec3);

fn puck_back_to_start(
    mut commands: Commands,
    query: Query<(Entity, Ref<Transform>, &Puck), Without<Path>>,
    navmeshes: Res<Assets<NavMesh>>,
    navmesh: Query<&ManagedNavMesh>,
) {
    let Some(navmesh) = navmeshes.get(navmesh.single()) else {
        return;
    };

    for (entity, transform, puck) in query.iter() {
        if transform.translation.y < -300.0 {
            let Some(path) = navmesh.transformed_path(transform.translation, puck.0) else {
                continue;
            };

            if let Some((first, remaining)) = path.path.split_first() {
                let mut remaining = remaining.to_vec();
                remaining.reverse();

                commands.entity(entity).insert((
                    RigidBody::Static,
                    Path {
                        current: *first,
                        next: remaining,
                    },
                ));
            }
        }
    }
}

#[derive(Component)]
pub struct Path {
    current: Vec3,
    next: Vec<Vec3>,
}

pub fn move_puck(
    mut commands: Commands,
    mut navigator: Query<(&mut Transform, &mut Path, Entity, &mut LinearVelocity)>,
    time: Res<Time>,
) {
    for (mut transform, mut path, entity, mut linvel) in navigator.iter_mut() {
        let move_direction = path.current - transform.translation;
        transform.translation += move_direction.normalize() * time.delta_secs() * 100.0;

        if transform.translation.distance(path.current) < 10.0 {
            if let Some(next) = path.next.pop() {
                path.current = next;
            }
        }
        if transform.translation.distance(path.current) < 50.0 && path.next.is_empty() {
            commands
                .entity(entity)
                .insert(RigidBody::Dynamic)
                .remove::<Path>();
            linvel.0 = (path.current - transform.translation).xy() * 10.0;
            continue;
        }
    }
}

pub fn display_puck_path(navigator: Query<(&Transform, &Path)>, mut gizmos: Gizmos) {
    for (transform, path) in &navigator {
        let mut to_display = path.next.iter().map(|v| v.xy()).collect::<Vec<_>>();
        to_display.push(path.current.xy());
        to_display.push(transform.translation.xy());
        to_display.reverse();
        if !to_display.is_empty() {
            gizmos.linestrip_2d(to_display, palettes::tailwind::YELLOW_400);
        }
    }
}
