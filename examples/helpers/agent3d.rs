use bevy::{color::palettes, prelude::*, utils::EntityHashMap};
use rand::Rng;
use vleue_navigator::prelude::*;

#[derive(Component)]
pub struct Navigator {
    speed: f32,
    color: Color,
}

#[derive(Component)]
pub struct Path {
    current: Vec3,
    next: Vec<Vec3>,
    target: Entity,
}

pub fn setup_agent<const SIZE: u32>(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let sphere = meshes.add(Sphere::new(0.6).mesh());
    for color in [
        palettes::tailwind::AMBER_400,
        palettes::tailwind::BLUE_400,
        palettes::tailwind::CYAN_400,
        palettes::tailwind::EMERALD_400,
        palettes::tailwind::FUCHSIA_400,
        palettes::tailwind::GREEN_400,
        palettes::tailwind::INDIGO_400,
        palettes::tailwind::LIME_400,
        palettes::tailwind::ORANGE_400,
        palettes::tailwind::PINK_400,
        palettes::tailwind::PURPLE_400,
        palettes::tailwind::RED_400,
        palettes::tailwind::ROSE_400,
        palettes::tailwind::SKY_400,
        palettes::tailwind::STONE_400,
        palettes::tailwind::TEAL_400,
        palettes::tailwind::VIOLET_400,
        palettes::tailwind::YELLOW_400,
    ] {
        commands.spawn((
            PbrBundle {
                mesh: sphere.clone(),
                material: materials.add(StandardMaterial {
                    base_color: color.into(),
                    emissive_exposure_weight: 0.0,
                    ..default()
                }),
                ..default()
            },
            Navigator {
                speed: SIZE as f32 * 0.2,
                color: color.into(),
            },
        ));
    }
}

pub fn give_target_to_navigator<const X: u32, const Y: u32>(
    mut commands: Commands,
    navigators: Query<(Entity, &Transform, &Navigator), Without<Path>>,
    mut navmeshes: ResMut<Assets<NavMesh>>,
    navmesh: Query<&Handle<NavMesh>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (entity, transform, navigator) in &navigators {
        let Some(navmesh) = navmeshes.get_mut(navmesh.single()) else {
            continue;
        };

        let mut target;

        let mut i = 50;
        loop {
            target = Vec3::new(
                rand::thread_rng().gen_range(0.0..(X as f32)),
                0.0,
                rand::thread_rng().gen_range(0.0..(Y as f32)),
            );

            if navmesh.transformed_is_in_mesh(target) {
                break;
            }
            i -= 1;
            if i == 0 {
                error!("No target found for navigator");
                return;
            }
        }

        let Some(path) = navmesh.transformed_path(transform.translation, target) else {
            continue;
        };
        if let Some((first, remaining)) = path.path.split_first() {
            let mut remaining = remaining.to_vec();
            remaining.reverse();
            let id = commands
                .spawn(PbrBundle {
                    mesh: meshes.add(Capsule3d::new(0.5, 2.0).mesh()),
                    material: materials.add(StandardMaterial {
                        base_color: navigator.color,
                        emissive: navigator.color.to_linear(),
                        emissive_exposure_weight: 0.0,
                        ..default()
                    }),
                    transform: Transform::from_translation(target),
                    ..default()
                })
                .id();
            commands.entity(entity).insert(Path {
                current: *first,
                next: remaining,
                target: id,
            });
        }
    }
}

pub fn refresh_path<const X: u32, const Y: u32>(
    mut commands: Commands,
    mut navigator: Query<(Entity, &Transform, &mut Path), With<Navigator>>,
    mut navmeshes: ResMut<Assets<NavMesh>>,
    navmesh: Query<(&Handle<NavMesh>, Ref<NavMeshStatus>)>,
    transforms: Query<&Transform>,
    mut deltas: Local<EntityHashMap<Entity, f32>>,
) {
    let (navmesh_handle, status) = navmesh.single();
    if (!status.is_changed() || *status != NavMeshStatus::Built) && deltas.is_empty() {
        return;
    }
    let Some(navmesh) = navmeshes.get_mut(navmesh_handle) else {
        return;
    };

    for (entity, transform, mut path) in &mut navigator {
        let target = transforms.get(path.target).unwrap().translation;
        navmesh.set_search_delta(0.0);
        if !navmesh.transformed_is_in_mesh(transform.translation) {
            let delta_for_entity = deltas.entry(entity).or_insert(0.0);
            *delta_for_entity += 0.1;
            navmesh.set_search_delta(*delta_for_entity);
            continue;
        }
        if !navmesh.transformed_is_in_mesh(target) {
            commands.entity(path.target).despawn();
            commands.entity(entity).remove::<Path>();
            continue;
        }

        let Some(new_path) = navmesh.transformed_path(transform.translation, target) else {
            commands.entity(path.target).despawn();
            commands.entity(entity).remove::<Path>();
            continue;
        };
        if let Some((first, remaining)) = new_path.path.split_first() {
            let mut remaining = remaining.to_vec();
            remaining.reverse();
            path.current = *first;
            path.next = remaining;
            deltas.remove(&entity);
        }
    }
}

pub fn move_navigator<const SIZE: u32>(
    mut commands: Commands,
    mut navigator: Query<(&mut Transform, &mut Path, Entity, &mut Navigator)>,
    time: Res<Time<Real>>,
) {
    for (mut transform, mut path, entity, mut navigator) in navigator.iter_mut() {
        let move_direction = path.current - transform.translation;
        transform.translation +=
            move_direction.normalize() * time.delta_seconds() * navigator.speed;
        let mut distance_to_next = transform.translation.distance(path.current);
        if distance_to_next < navigator.speed * time.delta_seconds() * 2.0 {
            navigator.speed = (navigator.speed * 0.9).max(SIZE as f32 * 0.01);
        }
        while distance_to_next < SIZE as f32 / 250.0 {
            navigator.speed = SIZE as f32 * 0.2;
            if let Some(next) = path.next.pop() {
                path.current = next;
                distance_to_next = transform.translation.distance(path.current);
            } else {
                commands.entity(entity).remove::<Path>();
                commands.entity(path.target).despawn_recursive();
                break;
            }
        }
    }
}

pub fn display_navigator_path(
    navigator: Query<(&Transform, &Path, &Navigator)>,
    mut gizmos: Gizmos,
) {
    for (transform, path, navigator) in &navigator {
        let mut to_display = path.next.clone();
        to_display.push(path.current);
        to_display.push(transform.translation);
        to_display.reverse();
        if !to_display.is_empty() {
            gizmos.linestrip(
                to_display.iter().map(|xz| Vec3::new(xz.x, 0.1, xz.z)),
                navigator.color,
            );
        }
    }
}
