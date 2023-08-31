use bevy::{
    math::{vec3, Vec3Swizzles},
    prelude::*,
    render::primitives::Aabb,
};
use bevy_pathmesh::{
    updater::{NavMeshStatus, NavmeshUpdateTask, NavmeshUpdaterPlugin, ObstacleSource},
    PathMesh,
};
use bevy_xpbd_3d::parry::{math::Isometry, na::Vector3, shape::TriMesh};

use crate::{
    ui::{Slider, UiButton, UiInfo},
    Agent, MyCollider, Obstacle, Path, Target, HANDLE_NAVMESH_MESH, HANDLE_NAVMESH_WIREFRAME,
};

pub struct BuilderPlugin;

impl Plugin for BuilderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(NavmeshUpdaterPlugin::<Obstacle, MyCollider>::default())
            .add_systems(Update, (update, ignore_fail_updates));
    }
}

impl ObstacleSource for MyCollider {
    fn get_polygon(
        &self,
        global_transform: &GlobalTransform,
        mesh_transform: &Transform,
    ) -> Vec<Vec2> {
        let transform = global_transform.compute_transform();
        let to_vec2 = |v: Vec3| mesh_transform.transform_point(v).xy();
        let intersection = match self.0.as_typed_shape() {
            bevy_xpbd_3d::parry::shape::TypedShape::Ball(collider) => {
                let (vtx, idx) = collider.to_trimesh(10, 10);
                TriMesh::new(vtx, idx).intersection_with_plane(
                    &Isometry::from_parts(transform.translation.into(), transform.rotation.into()),
                    &Vector3::ith_axis(1),
                    0.1,
                    std::f32::EPSILON,
                )
            }
            bevy_xpbd_3d::parry::shape::TypedShape::Cuboid(collider) => {
                let (vtx, idx) = collider.to_trimesh();
                TriMesh::new(vtx, idx).intersection_with_plane(
                    &Isometry::from_parts(transform.translation.into(), transform.rotation.into()),
                    &Vector3::ith_axis(1),
                    0.1,
                    std::f32::EPSILON,
                )
            }
            bevy_xpbd_3d::parry::shape::TypedShape::Capsule(collider) => {
                let (vtx, idx) = collider.to_trimesh(10, 10);
                TriMesh::new(vtx, idx).intersection_with_plane(
                    &Isometry::from_parts(transform.translation.into(), transform.rotation.into()),
                    &Vector3::ith_axis(1),
                    0.1,
                    std::f32::EPSILON,
                )
            }
            bevy_xpbd_3d::parry::shape::TypedShape::HeightField(collider) => {
                let (vtx, idx) = collider.to_trimesh();
                TriMesh::new(vtx, idx).intersection_with_plane(
                    &Isometry::from_parts(transform.translation.into(), transform.rotation.into()),
                    &Vector3::ith_axis(1),
                    0.1,
                    std::f32::EPSILON,
                )
            }
            bevy_xpbd_3d::parry::shape::TypedShape::ConvexPolyhedron(collider) => {
                let (vtx, idx) = collider.to_trimesh();
                TriMesh::new(vtx, idx).intersection_with_plane(
                    &Isometry::from_parts(transform.translation.into(), transform.rotation.into()),
                    &Vector3::ith_axis(1),
                    0.1,
                    std::f32::EPSILON,
                )
            }
            bevy_xpbd_3d::parry::shape::TypedShape::Cylinder(collider) => {
                let (vtx, idx) = collider.to_trimesh(10);
                TriMesh::new(vtx, idx).intersection_with_plane(
                    &Isometry::from_parts(transform.translation.into(), transform.rotation.into()),
                    &Vector3::ith_axis(1),
                    0.1,
                    std::f32::EPSILON,
                )
            }
            bevy_xpbd_3d::parry::shape::TypedShape::Cone(collider) => {
                let (vtx, idx) = collider.to_trimesh(10);
                TriMesh::new(vtx, idx).intersection_with_plane(
                    &Isometry::from_parts(transform.translation.into(), transform.rotation.into()),
                    &Vector3::ith_axis(1),
                    0.1,
                    std::f32::EPSILON,
                )
            }
            bevy_xpbd_3d::parry::shape::TypedShape::TriMesh(collider) => collider
                .intersection_with_plane(
                    &Isometry::from_parts(transform.translation.into(), transform.rotation.into()),
                    &Vector3::ith_axis(1),
                    0.1,
                    std::f32::EPSILON,
                ),
            _ => return vec![],
        };
        match intersection {
            bevy_xpbd_3d::parry::query::IntersectResult::Intersect(i) => i
                .segments()
                .map(|s| s.a)
                .map(|p| to_vec2(transform.transform_point(vec3(p[0], p[1], p[2]))))
                .collect(),
            bevy_xpbd_3d::parry::query::IntersectResult::Negative => vec![],
            bevy_xpbd_3d::parry::query::IntersectResult::Positive => vec![],
        }
    }
}

fn update(
    mut commands: Commands,
    obstacles: Query<(Ref<GlobalTransform>, &Aabb), With<Obstacle>>,
    mut meshes: ResMut<Assets<Mesh>>,
    pathmeshes: Res<Assets<PathMesh>>,
    mut agents: Query<(Entity, &mut Agent)>,
    targets: Query<&Transform, With<Target>>,
    mut text_info: Query<(&mut Text, &UiInfo)>,
    mut sliders: Query<(&mut Slider, &UiButton)>,
    navmeshes: Query<(&Handle<PathMesh>, Ref<NavMeshStatus>)>,
) {
    for (handle, status) in &navmeshes {
        if status.is_changed() {
            let pathmesh = pathmeshes.get(handle).unwrap();

            for (agent_entity, mut agent) in &mut agents {
                commands.entity(agent_entity).remove::<Path>();
                if let Some(target_entity) = agent.target {
                    if let Ok(target_transform) = targets.get(target_entity) {
                        if !pathmesh.transformed_is_in_mesh(target_transform.translation) {
                            commands.entity(target_entity).despawn_recursive();
                            agent.target = None;
                        }
                    }
                }
            }

            for (mut text, info) in &mut text_info {
                match info {
                    UiInfo::PolygonCount => {
                        text.sections[1].value = pathmesh.get().polygons.len().to_string();
                    }
                    UiInfo::ObstacleCount => {
                        text.sections[1].value = obstacles.iter().count().to_string();
                    }
                    UiInfo::Simplification => {
                        text.sections[0].style.color = Color::WHITE;
                    }
                    _ => (),
                }
            }
            for (mut slider, button) in &mut sliders {
                match button {
                    UiButton::Simplification => slider.line_color = Color::GREEN,
                    _ => (),
                }
            }

            meshes.set_untracked(HANDLE_NAVMESH_MESH, pathmesh.to_mesh());
            meshes.set_untracked(HANDLE_NAVMESH_WIREFRAME, pathmesh.to_wireframe_mesh());
        }
        match *status {
            NavMeshStatus::Failed => {
                for (mut text, info) in &mut text_info {
                    match info {
                        UiInfo::ObstacleCount => {
                            text.sections[1].value = obstacles.iter().count().to_string();
                        }
                        UiInfo::Simplification => {
                            text.sections[0].style.color = Color::RED;
                        }
                        _ => (),
                    }
                }
                for (mut slider, button) in &mut sliders {
                    match button {
                        UiButton::Simplification => slider.line_color = Color::RED,
                        _ => (),
                    }
                }
            }
            _ => (),
        }
    }
}

#[derive(Component)]
struct KillTimer(Timer);

fn ignore_fail_updates(
    mut commands: Commands,
    navmeshes: Query<Entity, Added<NavmeshUpdateTask>>,
    mut late: Query<(Entity, &mut KillTimer)>,
    time: Res<Time>,
) {
    for entity in &navmeshes {
        commands
            .entity(entity)
            .insert(KillTimer(Timer::from_seconds(0.5, TimerMode::Once)));
    }
    for (entity, mut timer) in &mut late {
        if timer.0.tick(time.delta()).just_finished() {
            commands
                .entity(entity)
                .remove::<KillTimer>()
                .remove::<NavmeshUpdateTask>();
        }
    }
}
