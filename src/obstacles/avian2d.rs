use avian2d::{parry::shape::TypedShape, prelude::Collider};
use bevy::{
    log::warn,
    math::{vec3, Vec2, Vec3Swizzles},
    prelude::{GlobalTransform, Transform},
};

use super::{ObstacleSource, RESOLUTION};

impl ObstacleSource for Collider {
    fn get_polygon(
        &self,
        obstacle_transform: &GlobalTransform,
        navmesh_transform: &Transform,
    ) -> Vec<Vec2> {
        self.shape()
            .as_typed_shape()
            .get_polygon(obstacle_transform, navmesh_transform)
    }
}

trait InnerObstacleSource {
    fn get_polygon(
        &self,
        obstacle_transform: &GlobalTransform,
        navmesh_transform: &Transform,
    ) -> Vec<Vec2>;
}

impl<'a> InnerObstacleSource for TypedShape<'a> {
    fn get_polygon(
        &self,
        obstacle_transform: &GlobalTransform,
        navmesh_transform: &Transform,
    ) -> Vec<Vec2> {
        let transform = obstacle_transform.compute_transform();

        match self {
            TypedShape::Ball(collider) => collider
                .to_polyline(RESOLUTION)
                .into_iter()
                .map(|v| transform.transform_point(vec3(v.x, v.y, 0.0)))
                .map(|p| p.xy())
                .collect(),
            TypedShape::Cuboid(collider) => collider
                .to_polyline()
                .into_iter()
                .map(|v| transform.transform_point(vec3(v.x, v.y, 0.0)))
                .map(|p| p.xy())
                .collect(),
            TypedShape::Capsule(collider) => collider
                .to_polyline(RESOLUTION)
                .into_iter()
                .map(|v| transform.transform_point(vec3(v.x, v.y, 0.0)))
                .map(|p| p.xy())
                .collect(),
            TypedShape::Triangle(collider) => [collider.a, collider.b, collider.c]
                .into_iter()
                .map(|v| transform.transform_point(vec3(v.x, v.y, 0.0)))
                .map(|p| p.xy())
                .collect(),
            TypedShape::TriMesh(collider) => collider
                .vertices()
                .into_iter()
                .map(|v| transform.transform_point(vec3(v.x, v.y, 0.0)))
                .map(|p| p.xy())
                .collect(),
            TypedShape::Polyline(collider) => collider
                .vertices()
                .into_iter()
                .map(|v| transform.transform_point(vec3(v.x, v.y, 0.0)))
                .map(|p| p.xy())
                .collect(),
            TypedShape::Compound(collider) => collider
                .shapes()
                .iter()
                .flat_map(|(_iso, shape)| {
                    // TODO: handle the isometry of each shape
                    shape
                        .as_typed_shape()
                        .get_polygon(obstacle_transform, navmesh_transform)
                })
                .collect(),
            TypedShape::ConvexPolygon(collider) => collider
                .points()
                .into_iter()
                .map(|v| transform.transform_point(vec3(v.x, v.y, 0.0)))
                .map(|p| p.xy())
                .collect(),
            TypedShape::RoundCuboid(collider) => collider
                .to_polyline(RESOLUTION)
                .into_iter()
                .map(|v| transform.transform_point(vec3(v.x, v.y, 0.0)))
                .map(|p| p.xy())
                .collect(),
            // TODO: handle the round corner or RoundTriangle
            TypedShape::RoundTriangle(collider) => [
                collider.inner_shape.a,
                collider.inner_shape.b,
                collider.inner_shape.c,
            ]
            .into_iter()
            .map(|v| transform.transform_point(vec3(v.x, v.y, 0.0)))
            .map(|p| p.xy())
            .collect(),
            TypedShape::RoundConvexPolygon(collider) => collider
                .to_polyline(RESOLUTION)
                .into_iter()
                .map(|v| transform.transform_point(vec3(v.x, v.y, 0.0)))
                .map(|p| p.xy())
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
