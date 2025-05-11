use bevy::{color::palettes, prelude::*};
use rand::Rng;
use std::ops::Deref;
use vleue_navigator::prelude::*;

#[derive(Component)]
pub struct Navigator {
    speed: f32,
}

#[derive(Component)]
pub struct Path {
    current: Vec2,
    next: Vec<Vec2>,
    target: Entity,
}

pub fn setup_agent<const SIZE: u32>(mut commands: Commands) {
    commands.spawn((
        Sprite {
            color: palettes::css::RED.into(),
            custom_size: Some(Vec2::ONE),
            ..default()
        },
        Transform::from_translation(Vec3::new(0.0, 0.0, 1.0)).with_scale(Vec3::splat(SIZE as f32)),
        Navigator {
            speed: SIZE as f32 * 5.0,
        },
    ));
}

pub fn give_target_to_navigator<const SIZE: u32, const X: u32, const Y: u32>(
    mut commands: Commands,
    navigator: Query<(Entity, &Transform), (With<Navigator>, Without<Path>)>,
    navmeshes: Res<Assets<NavMesh>>,
    navmesh: Query<&ManagedNavMesh>,
) {
    for (entity, transform) in &navigator {
        let Ok(navmesh) = navmesh.single() else {
            continue;
        };
        let Some(navmesh) = navmeshes.get(navmesh) else {
            continue;
        };
        let mut x = 1.0;
        let mut y = 1.0;
        for _ in 0..10 {
            x = rand::thread_rng().gen_range(0.0..(X as f32));
            y = rand::thread_rng().gen_range(0.0..(Y as f32));

            if navmesh.is_in_mesh(Vec2::new(x, y)) {
                break;
            }
        }
        let Some(path) = navmesh.transformed_path(
            transform.translation.xyz(),
            navmesh.transform().transform_point(Vec3::new(x, y, 0.0)),
        ) else {
            break;
        };
        if let Some((first, remaining)) = path.path.split_first() {
            let mut remaining = remaining.iter().map(|p| p.xy()).collect::<Vec<_>>();
            remaining.reverse();
            let id = commands
                .spawn((
                    Sprite {
                        color: palettes::tailwind::FUCHSIA_500.into(),
                        custom_size: Some(Vec2::ONE),
                        ..default()
                    },
                    Transform::from_translation(
                        remaining.first().unwrap_or(&first.xy()).extend(1.5),
                    )
                    .with_scale(Vec3::splat(SIZE as f32)),
                ))
                .id();
            commands.entity(entity).insert(Path {
                current: first.xy(),
                next: remaining,
                target: id,
            });
        }
    }
}

pub fn refresh_path<const SIZE: u32, const X: u32, const Y: u32>(
    mut commands: Commands,
    mut navigator: Query<(Entity, &Transform, &mut Path), With<Navigator>>,
    mut navmeshes: ResMut<Assets<NavMesh>>,
    navmesh: Single<(&ManagedNavMesh, Ref<NavMeshStatus>)>,
    transforms: Query<&Transform>,
    mut delta: Local<f32>,
) {
    let (navmesh_handle, status) = navmesh.deref();
    if (!status.is_changed() || **status != NavMeshStatus::Built) && *delta == 0.0 {
        return;
    }
    let Some(navmesh) = navmeshes.get_mut(*navmesh_handle) else {
        return;
    };

    for (entity, transform, mut path) in &mut navigator {
        let target = transforms.get(path.target).unwrap().translation.xy();
        if !navmesh.transformed_is_in_mesh(transform.translation) {
            *delta += 0.1;
            navmesh.set_search_delta(*delta);
            continue;
        }
        if !navmesh.transformed_is_in_mesh(target.extend(0.0)) {
            commands.entity(path.target).despawn();
            commands.entity(entity).remove::<Path>();
            continue;
        }

        let Some(new_path) = navmesh.transformed_path(transform.translation, target.extend(0.0))
        else {
            commands.entity(path.target).despawn();
            commands.entity(entity).remove::<Path>();
            continue;
        };
        if let Some((first, remaining)) = new_path.path.split_first() {
            let mut remaining = remaining.iter().map(|p| p.xy()).collect::<Vec<_>>();
            remaining.reverse();
            path.current = first.xy();
            path.next = remaining;
            *delta = 0.0;
        }
    }
}

pub fn move_navigator(
    mut commands: Commands,
    mut navigator: Query<(&mut Transform, &mut Path, Entity, &Navigator)>,
    time: Res<Time>,
) {
    for (mut transform, mut path, entity, navigator) in navigator.iter_mut() {
        let move_direction = path.current - transform.translation.xy();
        transform.translation +=
            (move_direction.normalize() * time.delta_secs() * navigator.speed).extend(0.0);
        while transform.translation.xy().distance(path.current) < navigator.speed / 50.0 {
            if let Some(next) = path.next.pop() {
                path.current = next;
            } else {
                commands.entity(entity).remove::<Path>();
                commands.entity(path.target).despawn();
                break;
            }
        }
    }
}

pub fn display_navigator_path(navigator: Query<(&Transform, &Path)>, mut gizmos: Gizmos) {
    let Ok((transform, path)) = navigator.single() else {
        return;
    };
    let mut to_display = path.next.clone();
    to_display.push(path.current);
    to_display.push(transform.translation.xy());
    to_display.reverse();
    if !to_display.is_empty() {
        gizmos.linestrip_2d(to_display, palettes::css::YELLOW);
    }
}
