use avian2d::{
    dynamics::rigid_body::Sleeping,
    parry::{
        na::{Const, OPoint},
        shape::TypedShape,
    },
    prelude::Collider,
};
use bevy::{
    math::{vec3, Vec3Swizzles},
    prelude::*,
};

use crate::{updater::CachableObstacle, world_to_mesh};

use super::{ObstacleSource, RESOLUTION};

impl ObstacleSource for Collider {
    fn get_polygon(
        &self,
        obstacle_transform: &GlobalTransform,
        navmesh_transform: &Transform,
        up: (Dir3, f32),
    ) -> Vec<Vec2> {
        self.shape_scaled()
            .as_typed_shape()
            .get_polygon(obstacle_transform, navmesh_transform, up)
    }
}

trait InnerObstacleSource {
    fn get_polygon(
        &self,
        obstacle_transform: &GlobalTransform,
        navmesh_transform: &Transform,
        up: (Dir3, f32),
    ) -> Vec<Vec2>;
}

impl<'a> InnerObstacleSource for TypedShape<'a> {
    fn get_polygon(
        &self,
        obstacle_transform: &GlobalTransform,
        navmesh_transform: &Transform,
        (up, _shift): (Dir3, f32),
    ) -> Vec<Vec2> {
        let transform = obstacle_transform.compute_transform();
        let world_to_mesh = world_to_mesh(navmesh_transform);

        let ref_to_world = |p: &OPoint<f32, Const<2>>| {
            let mut v = vec3(p.x, 0.0, p.y);
            v = if up.is_negative_bitmask().count_ones() % 2 == 1 {
                Quat::from_rotation_arc(-Vec3::Y, up.into()).mul_vec3(v)
            } else {
                Quat::from_rotation_arc(Vec3::Y, up.into()).mul_vec3(v)
            };
            transform.transform_point(v)
        };
        let to_world = |p: OPoint<f32, Const<2>>| ref_to_world(&p);

        let to_navmesh = |v: Vec3| world_to_mesh.transform_point3(v).xy();

        match self {
            TypedShape::Ball(collider) => collider
                .to_polyline(RESOLUTION)
                .into_iter()
                .map(to_world)
                .map(to_navmesh)
                .collect(),
            TypedShape::Cuboid(collider) => collider
                .to_polyline()
                .into_iter()
                .map(to_world)
                .map(to_navmesh)
                .collect(),
            TypedShape::Capsule(collider) => collider
                .to_polyline(RESOLUTION)
                .into_iter()
                .map(to_world)
                .map(to_navmesh)
                .collect(),
            TypedShape::Triangle(collider) => [collider.a, collider.b, collider.c]
                .into_iter()
                .map(to_world)
                .map(to_navmesh)
                .collect(),
            TypedShape::TriMesh(collider) => collider
                .vertices()
                .iter()
                .map(ref_to_world)
                .map(to_navmesh)
                .collect(),
            TypedShape::Polyline(collider) => collider
                .vertices()
                .iter()
                .map(ref_to_world)
                .map(to_navmesh)
                .collect(),
            TypedShape::Compound(collider) => collider
                .shapes()
                .iter()
                .flat_map(|(_iso, shape)| {
                    // TODO: handle the isometry of each shape
                    shape.as_typed_shape().get_polygon(
                        obstacle_transform,
                        navmesh_transform,
                        (up, _shift),
                    )
                })
                .collect(),
            TypedShape::ConvexPolygon(collider) => collider
                .points()
                .iter()
                .map(ref_to_world)
                .map(to_navmesh)
                .collect(),
            TypedShape::RoundCuboid(collider) => collider
                .to_polyline(RESOLUTION)
                .into_iter()
                .map(to_world)
                .map(to_navmesh)
                .collect(),
            // TODO: handle the round corner or RoundTriangle
            TypedShape::RoundTriangle(collider) => [
                collider.inner_shape.a,
                collider.inner_shape.b,
                collider.inner_shape.c,
            ]
            .into_iter()
            .map(to_world)
            .map(to_navmesh)
            .collect(),
            TypedShape::RoundConvexPolygon(collider) => collider
                .to_polyline(RESOLUTION)
                .into_iter()
                .map(to_world)
                .map(to_navmesh)
                .collect(),
            TypedShape::Segment(_) => {
                warn!("Segment collider not supported for NavMesh obstacle generation");
                vec![]
            }
            TypedShape::HalfSpace(_) => {
                warn!("HalfSpace collider not supported for NavMesh obstacle generation");
                vec![]
            }
            TypedShape::HeightField(_) => {
                warn!("HeightField collider not supported for NavMesh obstacle generation");
                vec![]
            }
            TypedShape::Custom(_) => {
                warn!("Custom collider not supported for NavMesh obstacle generation");
                vec![]
            }
        }
    }
}

pub fn on_sleeping_inserted(trigger: Trigger<OnInsert, Sleeping>, mut commands: Commands) {
    commands.entity(trigger.entity()).insert(CachableObstacle);
}

pub fn on_sleeping_removed(trigger: Trigger<OnRemove, Sleeping>, mut commands: Commands) {
    commands
        .entity(trigger.entity())
        .remove::<CachableObstacle>();
}
