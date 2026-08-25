use avian3d::{
    dynamics::rigid_body::sleeping::Sleeping,
    parry::{
        math::{Pose, Vector3},
        query::IntersectResult,
        shape::{Polyline, TriMesh, TypedShape},
    },
    prelude::Collider,
};
use bevy::{math::vec3, prelude::*};

use crate::{updater::CachableObstacle, world_to_mesh};

use super::{ObstacleSource, RESOLUTION};

impl ObstacleSource for Collider {
    fn get_polygons(
        &self,
        obstacle_transform: &GlobalTransform,
        navmesh_transform: &Transform,
        up: (Dir3, f32),
    ) -> Vec<Vec<Vec2>> {
        self.shape_scaled().as_typed_shape().get_polygons(
            obstacle_transform,
            navmesh_transform,
            up,
            &Pose::IDENTITY,
        )
    }
}

trait InnerObstacleSource {
    fn get_polygons(
        &self,
        obstacle_transform: &GlobalTransform,
        navmesh_transform: &Transform,
        up: (Dir3, f32),
        // Pose of this shape relative to the collider root, accumulated through
        // nested compounds.
        local_pose: &Pose,
    ) -> Vec<Vec<Vec2>>;
}

impl InnerObstacleSource for TypedShape<'_> {
    fn get_polygons(
        &self,
        obstacle_transform: &GlobalTransform,
        navmesh_transform: &Transform,
        (up, shift): (Dir3, f32),
        local_pose: &Pose,
    ) -> Vec<Vec<Vec2>> {
        let mut transform = obstacle_transform.compute_transform();
        transform.scale = Vec3::ONE;
        let world_to_mesh = world_to_mesh(navmesh_transform);

        let to_navmesh = |p: Vector3| world_to_mesh.transform_point(vec3(p.x, p.y, p.z)).xy();

        let intersection_to_navmesh = |intersection: IntersectResult<Polyline>| match intersection {
            IntersectResult::Intersect(i) => i
                .extract_connected_components()
                .iter()
                .map(|p| p.segments().map(|s| s.a).map(to_navmesh).collect())
                .collect(),
            IntersectResult::Negative => vec![],
            IntersectResult::Positive => vec![],
        };

        let d = (-up.x * navmesh_transform.translation.x
            - up.y * navmesh_transform.translation.y
            - up.z * navmesh_transform.translation.z)
            / (up.x.powi(2) + up.y.powi(2) + up.z.powi(2)).sqrt();
        let plane_shift: f32 = shift - d;

        let to_world = |p: &Vector3| {
            let p = local_pose.transform_point(*p);
            transform.transform_point(vec3(p.x, p.y, p.z))
        };

        let up_axis = Vector3::new(up.x, up.y, up.z).normalize();
        let trimesh_to_world = |vertices: Vec<Vector3>| {
            vertices
                .iter()
                .map(to_world)
                .map(|v| Vector3::new(v.x, v.y, v.z))
                .collect::<Vec<Vector3>>()
        };
        match self {
            TypedShape::Cuboid(collider) => {
                let (vertices, indices) = collider.to_trimesh();
                let trimesh = TriMesh::new(trimesh_to_world(vertices), indices).unwrap();
                vec![intersection_to_navmesh(
                    trimesh.intersection_with_local_plane(up_axis, plane_shift, f32::EPSILON),
                )]
            }
            TypedShape::Ball(collider) => {
                let (vertices, indices) = collider.to_trimesh(RESOLUTION, RESOLUTION);
                let trimesh = TriMesh::new(trimesh_to_world(vertices), indices).unwrap();
                vec![intersection_to_navmesh(
                    trimesh.intersection_with_local_plane(up_axis, plane_shift, f32::EPSILON),
                )]
            }
            TypedShape::Capsule(collider) => {
                let (vertices, indices) = collider.to_trimesh(RESOLUTION, RESOLUTION);
                let trimesh = TriMesh::new(trimesh_to_world(vertices), indices).unwrap();
                vec![intersection_to_navmesh(
                    trimesh.intersection_with_local_plane(up_axis, plane_shift, f32::EPSILON),
                )]
            }
            TypedShape::TriMesh(collider) => {
                let trimesh = TriMesh::new(
                    trimesh_to_world(collider.vertices().to_vec()),
                    collider.indices().to_vec(),
                )
                .expect("Failed to create TriMesh");
                vec![intersection_to_navmesh(
                    trimesh.intersection_with_local_plane(up_axis, plane_shift, f32::EPSILON),
                )]
            }
            TypedShape::HeightField(collider) => {
                let (vertices, indices) = collider.to_trimesh();
                let trimesh = TriMesh::new(trimesh_to_world(vertices), indices).unwrap();
                vec![intersection_to_navmesh(
                    trimesh.intersection_with_local_plane(up_axis, plane_shift, f32::EPSILON),
                )]
            }
            // Each sub-shape is its own polygon, posed relative to the compound.
            TypedShape::Compound(collider) => collider
                .shapes()
                .iter()
                .map(|(pose, shape)| {
                    shape.as_typed_shape().get_polygons(
                        obstacle_transform,
                        navmesh_transform,
                        (up, shift),
                        &(local_pose * pose),
                    )
                })
                .collect(),
            TypedShape::ConvexPolyhedron(collider) => {
                let (vertices, indices) = collider.to_trimesh();
                let trimesh = TriMesh::new(trimesh_to_world(vertices), indices)
                    .expect("Failed to create TriMesh");
                vec![intersection_to_navmesh(
                    trimesh.intersection_with_local_plane(up_axis, plane_shift, f32::EPSILON),
                )]
            }
            TypedShape::Cylinder(collider) => {
                let (vertices, indices) = collider.to_trimesh(RESOLUTION);
                let trimesh = TriMesh::new(trimesh_to_world(vertices), indices)
                    .expect("Failed to create TriMesh");
                vec![intersection_to_navmesh(
                    trimesh.intersection_with_local_plane(up_axis, plane_shift, f32::EPSILON),
                )]
            }
            TypedShape::Cone(collider) => {
                let (vertices, indices) = collider.to_trimesh(RESOLUTION);
                let trimesh = TriMesh::new(trimesh_to_world(vertices), indices)
                    .expect("Failed to create TriMesh");
                vec![intersection_to_navmesh(
                    trimesh.intersection_with_local_plane(up_axis, plane_shift, f32::EPSILON),
                )]
            }
            TypedShape::RoundCuboid(collider) => {
                let (vertices, indices) = collider.inner_shape.to_trimesh();
                let trimesh = TriMesh::new(trimesh_to_world(vertices), indices)
                    .expect("Failed to create TriMesh");
                vec![intersection_to_navmesh(
                    trimesh.intersection_with_local_plane(up_axis, plane_shift, f32::EPSILON),
                )]
            }
            TypedShape::RoundCylinder(collider) => {
                let (vertices, indices) = collider.inner_shape.to_trimesh(RESOLUTION);
                let trimesh = TriMesh::new(trimesh_to_world(vertices), indices)
                    .expect("Failed to create TriMesh");
                vec![intersection_to_navmesh(
                    trimesh.intersection_with_local_plane(up_axis, plane_shift, f32::EPSILON),
                )]
            }
            TypedShape::RoundCone(collider) => {
                let (vertices, indices) = collider.inner_shape.to_trimesh(RESOLUTION);
                let trimesh = TriMesh::new(trimesh_to_world(vertices), indices)
                    .expect("Failed to create TriMesh");
                vec![intersection_to_navmesh(
                    trimesh.intersection_with_local_plane(up_axis, plane_shift, f32::EPSILON),
                )]
            }
            TypedShape::RoundConvexPolyhedron(collider) => {
                let (vertices, indices) = collider.inner_shape.to_trimesh();
                let trimesh = TriMesh::new(trimesh_to_world(vertices), indices)
                    .expect("Failed to create TriMesh");
                vec![intersection_to_navmesh(
                    trimesh.intersection_with_local_plane(up_axis, plane_shift, f32::EPSILON),
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
            TypedShape::Voxels(_) => {
                warn!("Voxels collider not supported for NavMesh obstacle generation");
                vec![]
            }
        }
        .into_iter()
        .flatten()
        .collect()
    }
}

pub fn on_sleeping_inserted(trigger: On<Insert, Sleeping>, mut commands: Commands) {
    commands
        .entity(trigger.event().entity)
        .insert(CachableObstacle);
}

pub fn on_sleeping_removed(trigger: On<Remove, Sleeping>, mut commands: Commands) {
    commands
        .entity(trigger.event().entity)
        .try_remove::<CachableObstacle>();
}

#[cfg(test)]
mod tests {
    use std::f32::consts::FRAC_PI_2;

    use super::*;

    // A 3d navmesh lies in the XY plane and is rotated flat, so its `forward`
    // (which is what the updater uses as `up`) points along +Y.
    fn navmesh_transform(translation: Vec3) -> Transform {
        Transform::from_translation(translation).with_rotation(Quat::from_rotation_x(FRAC_PI_2))
    }

    fn polygons(
        collider: &Collider,
        obstacle_transform: &GlobalTransform,
        navmesh_transform: &Transform,
    ) -> Vec<Vec<Vec2>> {
        let up = (navmesh_transform.forward(), 0.0);
        collider.get_polygons(obstacle_transform, navmesh_transform, up)
    }

    fn centroid(polygon: &[Vec2]) -> Vec2 {
        polygon.iter().sum::<Vec2>() / polygon.len() as f32
    }

    // A unit cube centred on the origin, as a vertex/index buffer.
    fn cube_trimesh() -> Collider {
        let vertices = vec![
            Vec3::new(-1.0, -1.0, -1.0),
            Vec3::new(1.0, -1.0, -1.0),
            Vec3::new(1.0, 1.0, -1.0),
            Vec3::new(-1.0, 1.0, -1.0),
            Vec3::new(-1.0, -1.0, 1.0),
            Vec3::new(1.0, -1.0, 1.0),
            Vec3::new(1.0, 1.0, 1.0),
            Vec3::new(-1.0, 1.0, 1.0),
        ];
        let indices = vec![
            [0, 2, 1],
            [0, 3, 2],
            [4, 5, 6],
            [4, 6, 7],
            [0, 1, 5],
            [0, 5, 4],
            [2, 3, 7],
            [2, 7, 6],
            [1, 2, 6],
            [1, 6, 5],
            [0, 4, 7],
            [0, 7, 3],
        ];
        Collider::trimesh(vertices, indices)
    }

    // The sub-shape pose used to be dropped, collapsing every sub-shape onto
    // the compound's origin.
    #[test]
    fn compound_applies_sub_shape_translation() {
        let compound = Collider::compound(vec![
            (
                Vec3::new(-10.0, 0.0, 0.0),
                Quat::IDENTITY,
                Collider::cuboid(2.0, 2.0, 2.0),
            ),
            (
                Vec3::new(10.0, 0.0, 0.0),
                Quat::IDENTITY,
                Collider::cuboid(2.0, 2.0, 2.0),
            ),
        ]);

        let polygons = polygons(
            &compound,
            &GlobalTransform::IDENTITY,
            &navmesh_transform(Vec3::ZERO),
        );

        assert_eq!(polygons.len(), 2);
        let distance = centroid(&polygons[0]).distance(centroid(&polygons[1]));
        assert!(
            (distance - 20.0).abs() < 1e-3,
            "sub-shapes should stay 20 units apart, got {distance}"
        );
    }

    // A compound wrapping a single shape at the origin must be indistinguishable
    // from that shape on its own. Uses a sphere and a navmesh offset along `up`,
    // so slicing at the wrong height changes the radius: the compound recursion
    // used to subtract the plane offset once per level of nesting.
    #[test]
    fn compound_of_one_matches_bare_shape_on_offset_navmesh() {
        let navmesh_transform = navmesh_transform(Vec3::new(0.0, 0.5, 0.0));

        let bare = polygons(
            &Collider::sphere(1.0),
            &GlobalTransform::IDENTITY,
            &navmesh_transform,
        );
        let wrapped = polygons(
            &Collider::compound(vec![(Vec3::ZERO, Quat::IDENTITY, Collider::sphere(1.0))]),
            &GlobalTransform::IDENTITY,
            &navmesh_transform,
        );

        assert_eq!(bare.len(), wrapped.len());
        assert!(!bare[0].is_empty());
        for (bare, wrapped) in bare.iter().flatten().zip(wrapped.iter().flatten()) {
            assert!(
                bare.distance(*wrapped) < 1e-3,
                "expected {bare}, got {wrapped}"
            );
        }
    }

    // Trimesh colliders used to be sliced in their own local space, ignoring
    // where the obstacle actually sits in the world.
    #[test]
    fn trimesh_respects_obstacle_transform() {
        let navmesh_transform = navmesh_transform(Vec3::ZERO);

        let at_origin = polygons(
            &cube_trimesh(),
            &GlobalTransform::IDENTITY,
            &navmesh_transform,
        );
        let offset = polygons(
            &cube_trimesh(),
            &GlobalTransform::from_translation(Vec3::new(10.0, 0.0, 0.0)),
            &navmesh_transform,
        );

        assert!(!at_origin[0].is_empty() && !offset[0].is_empty());
        let distance = centroid(&at_origin[0]).distance(centroid(&offset[0]));
        assert!(
            (distance - 10.0).abs() < 1e-3,
            "moving the obstacle 10 units should move its polygon, got {distance}"
        );
    }
}
