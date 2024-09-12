use avian3d::{
    dynamics::rigid_body::Sleeping,
    parry::{
        math::Vector,
        na::{Const, OPoint, Unit, Vector3},
        query::IntersectResult,
        shape::{
            Ball, Capsule, Cone, Cuboid, Cylinder, HeightField, Polyline, TriMesh, TypedShape,
        },
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

fn shape_to_trymesh(shape: &TypedShape, agent_radius: f32) -> Option<TriMesh> {
    if let TypedShape::TriMesh(trimesh) = shape {
        return (*trimesh).clone().into();
    }

    let verts_and_indices = match shape {
        TypedShape::Cuboid(collider) => {
            let expanded = Cuboid::new(Vector::new(
                collider.half_extents.x + agent_radius,
                collider.half_extents.y + agent_radius,
                collider.half_extents.z + agent_radius,
            ));
            Some(expanded.to_trimesh())
        }
        TypedShape::Ball(collider) => {
            let expanded = Ball::new(collider.radius + agent_radius);
            Some(expanded.to_trimesh(RESOLUTION, RESOLUTION))
        }
        TypedShape::Capsule(collider) => {
            let segment = collider.segment;
            let expanded = Capsule::new(segment.a, segment.b, collider.radius + agent_radius);
            Some(expanded.to_trimesh(RESOLUTION, RESOLUTION))
        }
        TypedShape::HeightField(collider) => {
            let expanded = HeightField::new(
                collider.heights().add_scalar(agent_radius),
                *collider.scale(),
            );
            Some(expanded.to_trimesh())
        }
        TypedShape::ConvexPolyhedron(collider) => {
            if agent_radius > 0.0 {
                warn!("ConvexPolyhedron doesn't support agent radius");
            }
            Some(collider.to_trimesh())
        }
        TypedShape::Cylinder(collider) => {
            let expanded = Cylinder::new(
                collider.half_height + (agent_radius / 2.0),
                collider.radius + agent_radius,
            );
            Some(expanded.to_trimesh(RESOLUTION))
        }
        TypedShape::Cone(collider) => {
            let expanded = Cone::new(
                collider.half_height + (agent_radius / 2.0),
                collider.radius + agent_radius,
            );
            Some(expanded.to_trimesh(RESOLUTION))
        }
        TypedShape::Segment(_) => {
            warn!("Segment collider not supported for NavMesh obstacle generation");
            None
        }
        TypedShape::Triangle(_) => {
            warn!("Triangle collider not supported for NavMesh obstacle generation");
            None
        }
        TypedShape::Polyline(_) => {
            warn!("Polyline collider not supported for NavMesh obstacle generation");
            None
        }
        TypedShape::HalfSpace(_) => {
            warn!("HalfSpace collider not supported for NavMesh obstacle generation");
            None
        }
        TypedShape::RoundCuboid(collider) => {
            if agent_radius > 0.0 {
                // ConvexPolyhedron needs to be constructured
                warn!("RoundCuboid doesn't support agent radius");
                None
            } else {
                Some(collider.inner_shape.to_trimesh())
            }
        }
        TypedShape::RoundCone(collider) => {
            if agent_radius > 0.0 {
                warn!("RoundTriangle doesn't support agent radius");
            }
            Some(collider.inner_shape.to_trimesh(RESOLUTION))
        }
        TypedShape::RoundCylinder(collider) => {
            if agent_radius > 0.0 {
                warn!("RoundCylinder doesn't support agent radius");
            }
            Some(collider.inner_shape.to_trimesh(RESOLUTION))
        }
        TypedShape::RoundTriangle(_) => {
            warn!("Polyline collider not supported for NavMesh obstacle generation");
            None
        }
        TypedShape::RoundConvexPolyhedron(collider) => {
            if agent_radius > 0.0 {
                warn!("RoundConvexPolyhedron doesn't support agent radius");
            }
            Some(collider.inner_shape.to_trimesh())
        }
        TypedShape::Custom(_) => {
            warn!("Cusomt collider not supported for NavMesh obstacle generation");
            None
        }

        TypedShape::Compound(_) => unreachable!(),
        TypedShape::TriMesh(_) => unreachable!(),
    };
    if let Some((vertices, indices)) = verts_and_indices {
        TriMesh::new(vertices, indices).into()
    } else {
        None
    }
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
        if let Some(tri_mesh) = shape_to_trymesh(self, agent_radius) {
            let trimesh = TriMesh::new(
                trimesh_to_world(tri_mesh.vertices().to_vec()),
                tri_mesh.indices().to_vec(),
            );
            vec![intersection_to_navmesh(
                trimesh.intersection_with_local_plane(&up_axis, shift, f32::EPSILON),
            )]
            .into_iter()
            .flatten()
            .collect()
        } else {
            vec![]
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
