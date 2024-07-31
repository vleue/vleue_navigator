use avian3d::{
    dynamics::rigid_body::Sleeping,
    parry::{
        math::Isometry,
        na::{Const, OPoint, Vector3},
        query::IntersectResult,
        shape::{Polyline, TriMesh, TypedShape},
    },
    prelude::Collider,
};
use bevy::{math::vec3, prelude::*};

use crate::updater::CachableObstacle;

use super::{ObstacleSource, RESOLUTION};

impl ObstacleSource for Collider {
    fn get_polygon(
        &self,
        obstacle_transform: &GlobalTransform,
        navmesh_transform: &Transform,
    ) -> Vec<Vec2> {
        self.shape_scaled()
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

const BIAS: f32 = 0.0;

impl<'a> InnerObstacleSource for TypedShape<'a> {
    fn get_polygon(
        &self,
        obstacle_transform: &GlobalTransform,
        navmesh_transform: &Transform,
    ) -> Vec<Vec2> {
        let transform = obstacle_transform.compute_transform();

        let inverse_navmesh_transform = navmesh_transform.compute_affine().inverse();

        let to_vec2 = |v: Vec3| inverse_navmesh_transform.transform_point3(v).xz();
        let intersection_to_polygon = |intersection: IntersectResult<Polyline>| match intersection {
            IntersectResult::Intersect(i) => i
                .segments()
                .map(|s| s.a)
                .map(|p| to_vec2(vec3(p[0], p[1], p[2])))
                .collect(),
            IntersectResult::Negative => vec![],
            IntersectResult::Positive => vec![],
        };

        let trimesh_to_local = |vertices: Vec<OPoint<f32, Const<3>>>| {
            vertices
                .iter()
                .map(|p| transform.transform_point(vec3(p[0], p[1], p[2])))
                .map(|p| navmesh_transform.transform_point(vec3(p[0], p[1], p[2])))
                .map(|v| v.into())
                .collect::<Vec<OPoint<f32, Const<3>>>>()
        };

        let intersection_plane = Isometry::from_parts(
            navmesh_transform.translation.into(),
            navmesh_transform.rotation.into(),
        );
        match self {
            TypedShape::Cuboid(collider) => {
                let (vertices, indices) = collider.to_trimesh();
                let trimesh = TriMesh::new(trimesh_to_local(vertices), indices);
                vec![intersection_to_polygon(trimesh.intersection_with_plane(
                    &intersection_plane,
                    &Vector3::ith_axis(1),
                    BIAS,
                    std::f32::EPSILON,
                ))]
            }
            TypedShape::Ball(collider) => {
                let (vertices, indices) = collider.to_trimesh(RESOLUTION, RESOLUTION);
                let trimesh = TriMesh::new(trimesh_to_local(vertices), indices);
                vec![intersection_to_polygon(trimesh.intersection_with_plane(
                    &intersection_plane,
                    &Vector3::ith_axis(1),
                    BIAS,
                    std::f32::EPSILON,
                ))]
            }
            TypedShape::Capsule(collider) => {
                let (vertices, indices) = collider.to_trimesh(RESOLUTION, RESOLUTION);
                let trimesh = TriMesh::new(trimesh_to_local(vertices), indices);
                vec![intersection_to_polygon(trimesh.intersection_with_plane(
                    &intersection_plane,
                    &Vector3::ith_axis(1),
                    BIAS,
                    std::f32::EPSILON,
                ))]
            }
            TypedShape::TriMesh(collider) => {
                vec![intersection_to_polygon(collider.intersection_with_plane(
                    &intersection_plane,
                    &Vector3::ith_axis(1),
                    BIAS,
                    std::f32::EPSILON,
                ))]
            }
            TypedShape::HeightField(collider) => {
                let (vertices, indices) = collider.to_trimesh();
                let trimesh = TriMesh::new(trimesh_to_local(vertices), indices);
                vec![intersection_to_polygon(trimesh.intersection_with_plane(
                    &intersection_plane,
                    &Vector3::ith_axis(1),
                    BIAS,
                    std::f32::EPSILON,
                ))]
            }
            TypedShape::Compound(collider) => {
                collider
                    .shapes()
                    .iter()
                    .map(|(_iso, shape)| {
                        // TODO: handle the isometry of each shape
                        shape
                            .as_typed_shape()
                            .get_polygon(obstacle_transform, navmesh_transform)
                    })
                    .collect()
            }
            TypedShape::ConvexPolyhedron(collider) => {
                let (vertices, indices) = collider.to_trimesh();
                let trimesh = TriMesh::new(trimesh_to_local(vertices), indices);
                vec![intersection_to_polygon(trimesh.intersection_with_plane(
                    &intersection_plane,
                    &Vector3::ith_axis(1),
                    BIAS,
                    std::f32::EPSILON,
                ))]
            }
            TypedShape::Cylinder(collider) => {
                let (vertices, indices) = collider.to_trimesh(RESOLUTION);
                let trimesh = TriMesh::new(trimesh_to_local(vertices), indices);
                vec![intersection_to_polygon(trimesh.intersection_with_plane(
                    &intersection_plane,
                    &Vector3::ith_axis(1),
                    BIAS,
                    std::f32::EPSILON,
                ))]
            }
            TypedShape::Cone(collider) => {
                let (vertices, indices) = collider.to_trimesh(RESOLUTION);
                let trimesh = TriMesh::new(trimesh_to_local(vertices), indices);
                vec![intersection_to_polygon(trimesh.intersection_with_plane(
                    &Isometry::from_parts(
                        navmesh_transform.translation.into(),
                        navmesh_transform.rotation.into(),
                    ),
                    &Vector3::ith_axis(1),
                    BIAS,
                    std::f32::EPSILON,
                ))]
            }
            TypedShape::RoundCuboid(collider) => {
                let (vertices, indices) = collider.inner_shape.to_trimesh();
                let trimesh = TriMesh::new(trimesh_to_local(vertices), indices);
                vec![intersection_to_polygon(trimesh.intersection_with_plane(
                    &intersection_plane,
                    &Vector3::ith_axis(1),
                    BIAS,
                    std::f32::EPSILON,
                ))]
            }
            TypedShape::RoundCylinder(collider) => {
                let (vertices, indices) = collider.inner_shape.to_trimesh(RESOLUTION);
                let trimesh = TriMesh::new(trimesh_to_local(vertices), indices);
                vec![intersection_to_polygon(trimesh.intersection_with_plane(
                    &intersection_plane,
                    &Vector3::ith_axis(1),
                    BIAS,
                    std::f32::EPSILON,
                ))]
            }
            TypedShape::RoundCone(collider) => {
                let (vertices, indices) = collider.inner_shape.to_trimesh(RESOLUTION);
                let trimesh = TriMesh::new(trimesh_to_local(vertices), indices);
                vec![intersection_to_polygon(trimesh.intersection_with_plane(
                    &intersection_plane,
                    &Vector3::ith_axis(1),
                    BIAS,
                    std::f32::EPSILON,
                ))]
            }
            TypedShape::RoundConvexPolyhedron(collider) => {
                let (vertices, indices) = collider.inner_shape.to_trimesh();
                let trimesh = TriMesh::new(trimesh_to_local(vertices), indices);
                vec![intersection_to_polygon(trimesh.intersection_with_plane(
                    &intersection_plane,
                    &Vector3::ith_axis(1),
                    BIAS,
                    std::f32::EPSILON,
                ))]
            }
            TypedShape::Segment(_) => {
                warn!("Segment collider not supported for NavMesh obstacle generation");
                vec![]
            }
            TypedShape::Triangle(_) => {
                warn!("Triangle collider not supported for NavMesh obstacle generation");
                vec![]
            }
            TypedShape::Polyline(_) => {
                warn!("Polyline collider not supported for NavMesh obstacle generation");
                vec![]
            }
            TypedShape::HalfSpace(_) => {
                warn!("HalfSpace collider not supported for NavMesh obstacle generation");
                vec![]
            }
            TypedShape::RoundTriangle(_) => {
                warn!("RoundTriangle collider not supported for NavMesh obstacle generation");
                vec![]
            }
            TypedShape::Custom(_) => {
                warn!("Custom collider not supported for NavMesh obstacle generation");
                vec![]
            }
        }
        .into_iter()
        .flatten()
        .collect()
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
