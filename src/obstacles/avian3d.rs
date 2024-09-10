use avian3d::{
    dynamics::rigid_body::Sleeping,
    math::Vector,
    parry::{
        na::{Const, OPoint, Unit, Vector3},
        query::IntersectResult,
        shape::{Cuboid, Polyline, TriMesh, TypedShape},
    },
    prelude::Collider,
};
use bevy::{
    log::warn,
    math::{vec3, Dir3, Vec2, Vec3, Vec3Swizzles},
    prelude::{Commands, GlobalTransform, OnInsert, OnRemove, Transform, TransformPoint, Trigger},
};

use crate::{updater::CachableObstacle, world_to_mesh};

use super::{ObstacleSource, RESOLUTION};

impl ObstacleSource for Collider {
    fn get_polygon(
        &self,
        obstacle_transform: &GlobalTransform,
        navmesh_transform: &Transform,
        up: (Dir3, f32),
        agent_radius: f32,
    ) -> Vec<Vec2> {
        self.shape_scaled().as_typed_shape().get_polygon(
            obstacle_transform,
            navmesh_transform,
            up,
            agent_radius,
        )
    }
}

trait InnerObstacleSource {
    fn get_polygon(
        &self,
        obstacle_transform: &GlobalTransform,
        navmesh_transform: &Transform,
        up: (Dir3, f32),
        agent_radius: f32,
    ) -> Vec<Vec2>;
}

impl<'a> InnerObstacleSource for TypedShape<'a> {
    fn get_polygon(
        &self,
        obstacle_transform: &GlobalTransform,
        navmesh_transform: &Transform,
        (up, shift): (Dir3, f32),
        agent_radius: f32,
    ) -> Vec<Vec2> {
        let mut transform = obstacle_transform.compute_transform();
        transform.scale = Vec3::ONE;
        let world_to_mesh = world_to_mesh(navmesh_transform);

        let to_navmesh =
            |p: OPoint<f32, Const<3>>| world_to_mesh.transform_point(vec3(p.x, p.y, p.z)).xy();

        let intersection_to_navmesh = |intersection: IntersectResult<Polyline>| match intersection {
            IntersectResult::Intersect(i) => i.segments().map(|s| s.a).map(to_navmesh).collect(),
            IntersectResult::Negative => vec![],
            IntersectResult::Positive => vec![],
        };

        let d = (-up.x * navmesh_transform.translation.x
            - up.y * navmesh_transform.translation.y
            - up.z * navmesh_transform.translation.z)
            / (up.x.powi(2) + up.y.powi(2) + up.z.powi(2)).sqrt();
        let shift: f32 = shift - d;

        let to_world = |p: &OPoint<f32, Const<3>>| transform.transform_point(vec3(p.x, p.y, p.z));

        let up_axis = Unit::new_normalize(Vector3::new(up.x, up.y, up.z));
        let trimesh_to_world = |vertices: Vec<OPoint<f32, Const<3>>>| {
            vertices
                .iter()
                .map(to_world)
                .map(|v| v.into())
                .collect::<Vec<OPoint<f32, Const<3>>>>()
        };
        match self {
            TypedShape::Cuboid(collider) => {
                let collider = Cuboid::new(
                    Vector::new(
                        collider.half_extents.x + agent_radius,
                        collider.half_extents.y + agent_radius,
                        collider.half_extents.z + agent_radius,
                    )
                    .into(),
                );
                let (vertices, indices) = collider.to_trimesh();
                let trimesh = TriMesh::new(trimesh_to_world(vertices), indices);
                vec![intersection_to_navmesh(
                    trimesh.intersection_with_local_plane(&up_axis, shift, f32::EPSILON),
                )]
            }
            TypedShape::Ball(collider) => {
                let (vertices, indices) = collider.to_trimesh(RESOLUTION, RESOLUTION);
                let trimesh = TriMesh::new(trimesh_to_world(vertices), indices);
                vec![intersection_to_navmesh(
                    trimesh.intersection_with_local_plane(&up_axis, shift, f32::EPSILON),
                )]
            }
            TypedShape::Capsule(collider) => {
                let (vertices, indices) = collider.to_trimesh(RESOLUTION, RESOLUTION);
                let trimesh = TriMesh::new(trimesh_to_world(vertices), indices);
                vec![intersection_to_navmesh(
                    trimesh.intersection_with_local_plane(&up_axis, shift, f32::EPSILON),
                )]
            }
            TypedShape::TriMesh(collider) => {
                vec![intersection_to_navmesh(
                    collider.intersection_with_local_plane(&up_axis, shift, f32::EPSILON),
                )]
            }
            TypedShape::HeightField(collider) => {
                let (vertices, indices) = collider.to_trimesh();
                let trimesh = TriMesh::new(trimesh_to_world(vertices), indices);
                vec![intersection_to_navmesh(
                    trimesh.intersection_with_local_plane(&up_axis, shift, f32::EPSILON),
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
                            (up, shift),
                            agent_radius,
                        )
                    })
                    .collect()
            }
            TypedShape::ConvexPolyhedron(collider) => {
                let (vertices, indices) = collider.to_trimesh();
                let trimesh = TriMesh::new(trimesh_to_world(vertices), indices);
                vec![intersection_to_navmesh(
                    trimesh.intersection_with_local_plane(&up_axis, shift, f32::EPSILON),
                )]
            }
            TypedShape::Cylinder(collider) => {
                let (vertices, indices) = collider.to_trimesh(RESOLUTION);
                let trimesh = TriMesh::new(trimesh_to_world(vertices), indices);
                vec![intersection_to_navmesh(
                    trimesh.intersection_with_local_plane(&up_axis, shift, f32::EPSILON),
                )]
            }
            TypedShape::Cone(collider) => {
                let (vertices, indices) = collider.to_trimesh(RESOLUTION);
                let trimesh = TriMesh::new(trimesh_to_world(vertices), indices);
                vec![intersection_to_navmesh(
                    trimesh.intersection_with_local_plane(&up_axis, shift, f32::EPSILON),
                )]
            }
            TypedShape::RoundCuboid(collider) => {
                let (vertices, indices) = collider.inner_shape.to_trimesh();
                let trimesh = TriMesh::new(trimesh_to_world(vertices), indices);
                vec![intersection_to_navmesh(
                    trimesh.intersection_with_local_plane(&up_axis, shift, f32::EPSILON),
                )]
            }
            TypedShape::RoundCylinder(collider) => {
                let (vertices, indices) = collider.inner_shape.to_trimesh(RESOLUTION);
                let trimesh = TriMesh::new(trimesh_to_world(vertices), indices);
                vec![intersection_to_navmesh(
                    trimesh.intersection_with_local_plane(&up_axis, shift, f32::EPSILON),
                )]
            }
            TypedShape::RoundCone(collider) => {
                let (vertices, indices) = collider.inner_shape.to_trimesh(RESOLUTION);
                let trimesh = TriMesh::new(trimesh_to_world(vertices), indices);
                vec![intersection_to_navmesh(
                    trimesh.intersection_with_local_plane(&up_axis, shift, f32::EPSILON),
                )]
            }
            TypedShape::RoundConvexPolyhedron(collider) => {
                let (vertices, indices) = collider.inner_shape.to_trimesh();
                let trimesh = TriMesh::new(trimesh_to_world(vertices), indices);
                vec![intersection_to_navmesh(
                    trimesh.intersection_with_local_plane(&up_axis, shift, f32::EPSILON),
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
