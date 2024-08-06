use avian3d::{
    dynamics::rigid_body::Sleeping,
    parry::{
        na::{Const, OPoint, Vector3},
        query::IntersectResult,
        shape::{Polyline, TriMesh, TypedShape},
    },
    prelude::Collider,
};
use bevy::{math::vec3, prelude::*};

use crate::{updater::CachableObstacle, world_to_mesh};

use super::{ObstacleSource, RESOLUTION};

impl ObstacleSource for Collider {
    fn get_polygon(
        &self,
        obstacle_transform: &GlobalTransform,
        navmesh_transform: &Transform,
        up: Dir3,
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
        up: Dir3,
    ) -> Vec<Vec2>;
}

impl<'a> InnerObstacleSource for TypedShape<'a> {
    fn get_polygon(
        &self,
        obstacle_transform: &GlobalTransform,
        navmesh_transform: &Transform,
        _up: Dir3,
    ) -> Vec<Vec2> {
        let transform = obstacle_transform.compute_transform();
        let world_to_mesh = world_to_mesh(navmesh_transform);

        let to_navmesh =
            |p: OPoint<f32, Const<3>>| world_to_mesh.transform_point3(vec3(p.x, p.y, p.z)).xy();

        let intersection_to_polygon = |intersection: IntersectResult<Polyline>| match intersection {
            IntersectResult::Intersect(i) => i.segments().map(|s| s.a).map(to_navmesh).collect(),
            IntersectResult::Negative => vec![],
            IntersectResult::Positive => vec![],
        };

        let to_world = |p: &OPoint<f32, Const<3>>| transform.transform_point(vec3(p.x, p.y, p.z));

        let up_axis = Vector3::ith_axis(1);
        let trimesh_to_navmesh = |vertices: Vec<OPoint<f32, Const<3>>>| {
            vertices
                .iter()
                .map(to_world)
                .map(|v| v.into())
                .collect::<Vec<OPoint<f32, Const<3>>>>()
        };
        match self {
            TypedShape::Cuboid(collider) => {
                let (vertices, indices) = collider.to_trimesh();
                let trimesh = TriMesh::new(trimesh_to_navmesh(vertices), indices);
                vec![intersection_to_polygon(
                    trimesh.intersection_with_local_plane(
                        &up_axis,
                        navmesh_transform.translation.y,
                        f32::EPSILON,
                    ),
                )]
            }
            TypedShape::Ball(collider) => {
                let (vertices, indices) = collider.to_trimesh(RESOLUTION, RESOLUTION);
                let trimesh = TriMesh::new(trimesh_to_navmesh(vertices), indices);
                vec![intersection_to_polygon(
                    trimesh.intersection_with_local_plane(
                        &Vector3::ith_axis(1),
                        navmesh_transform.translation.y,
                        f32::EPSILON,
                    ),
                )]
            }
            TypedShape::Capsule(collider) => {
                let (vertices, indices) = collider.to_trimesh(RESOLUTION, RESOLUTION);
                let trimesh = TriMesh::new(trimesh_to_navmesh(vertices), indices);
                vec![intersection_to_polygon(
                    trimesh.intersection_with_local_plane(
                        &Vector3::ith_axis(1),
                        navmesh_transform.translation.y,
                        f32::EPSILON,
                    ),
                )]
            }
            TypedShape::TriMesh(collider) => {
                vec![intersection_to_polygon(
                    collider.intersection_with_local_plane(
                        &Vector3::ith_axis(1),
                        navmesh_transform.translation.y,
                        f32::EPSILON,
                    ),
                )]
            }
            TypedShape::HeightField(collider) => {
                let (vertices, indices) = collider.to_trimesh();
                let trimesh = TriMesh::new(trimesh_to_navmesh(vertices), indices);
                vec![intersection_to_polygon(
                    trimesh.intersection_with_local_plane(
                        &Vector3::ith_axis(1),
                        navmesh_transform.translation.y,
                        f32::EPSILON,
                    ),
                )]
            }
            TypedShape::Compound(collider) => {
                collider
                    .shapes()
                    .iter()
                    .map(|(_iso, shape)| {
                        // TODO: handle the isometry of each shape
                        shape.as_typed_shape().get_polygon(
                            obstacle_transform,
                            navmesh_transform,
                            _up,
                        )
                    })
                    .collect()
            }
            TypedShape::ConvexPolyhedron(collider) => {
                let (vertices, indices) = collider.to_trimesh();
                let trimesh = TriMesh::new(trimesh_to_navmesh(vertices), indices);
                vec![intersection_to_polygon(
                    trimesh.intersection_with_local_plane(
                        &Vector3::ith_axis(1),
                        navmesh_transform.translation.y,
                        f32::EPSILON,
                    ),
                )]
            }
            TypedShape::Cylinder(collider) => {
                let (vertices, indices) = collider.to_trimesh(RESOLUTION);
                let trimesh = TriMesh::new(trimesh_to_navmesh(vertices), indices);
                vec![intersection_to_polygon(
                    trimesh.intersection_with_local_plane(
                        &Vector3::ith_axis(1),
                        navmesh_transform.translation.y,
                        f32::EPSILON,
                    ),
                )]
            }
            TypedShape::Cone(collider) => {
                let (vertices, indices) = collider.to_trimesh(RESOLUTION);
                let trimesh = TriMesh::new(trimesh_to_navmesh(vertices), indices);
                vec![intersection_to_polygon(
                    trimesh.intersection_with_local_plane(
                        &Vector3::ith_axis(1),
                        navmesh_transform.translation.y,
                        f32::EPSILON,
                    ),
                )]
            }
            TypedShape::RoundCuboid(collider) => {
                let (vertices, indices) = collider.inner_shape.to_trimesh();
                let trimesh = TriMesh::new(trimesh_to_navmesh(vertices), indices);
                vec![intersection_to_polygon(
                    trimesh.intersection_with_local_plane(
                        &Vector3::ith_axis(1),
                        navmesh_transform.translation.y,
                        f32::EPSILON,
                    ),
                )]
            }
            TypedShape::RoundCylinder(collider) => {
                let (vertices, indices) = collider.inner_shape.to_trimesh(RESOLUTION);
                let trimesh = TriMesh::new(trimesh_to_navmesh(vertices), indices);
                vec![intersection_to_polygon(
                    trimesh.intersection_with_local_plane(
                        &Vector3::ith_axis(1),
                        navmesh_transform.translation.y,
                        f32::EPSILON,
                    ),
                )]
            }
            TypedShape::RoundCone(collider) => {
                let (vertices, indices) = collider.inner_shape.to_trimesh(RESOLUTION);
                let trimesh = TriMesh::new(trimesh_to_navmesh(vertices), indices);
                vec![intersection_to_polygon(
                    trimesh.intersection_with_local_plane(
                        &Vector3::ith_axis(1),
                        navmesh_transform.translation.y,
                        f32::EPSILON,
                    ),
                )]
            }
            TypedShape::RoundConvexPolyhedron(collider) => {
                let (vertices, indices) = collider.inner_shape.to_trimesh();
                let trimesh = TriMesh::new(trimesh_to_navmesh(vertices), indices);
                vec![intersection_to_polygon(
                    trimesh.intersection_with_local_plane(
                        &Vector3::ith_axis(1),
                        navmesh_transform.translation.y,
                        f32::EPSILON,
                    ),
                )]
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
