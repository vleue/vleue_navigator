use avian2d::{
    parry::{
        math::{Pose, Vector2},
        shape::TypedShape,
    },
    prelude::{Collider, Sleeping},
};
use bevy::{
    math::{Vec3Swizzles, vec3},
    prelude::*,
};

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
        (up, _shift): (Dir3, f32),
        local_pose: &Pose,
    ) -> Vec<Vec<Vec2>> {
        let mut transform = obstacle_transform.compute_transform();
        transform.scale = Vec3::ONE;
        let world_to_mesh = world_to_mesh(navmesh_transform);

        let ref_to_world = |p: &Vector2| {
            let p = local_pose.transform_point(*p);
            let mut v = vec3(p.x, 0.0, p.y);
            v = if up.is_negative_bitmask().count_ones() % 2 == 1 {
                Quat::from_rotation_arc(Vec3::Y, up.into()).mul_vec3(v)
            } else {
                Quat::from_rotation_arc(-Vec3::Y, up.into()).mul_vec3(v)
            };
            transform.transform_point(v)
        };
        let to_world = |p: Vector2| ref_to_world(&p);

        let to_navmesh = |v: Vec3| world_to_mesh.transform_point3(v).xy();

        match self {
            TypedShape::Ball(collider) => vec![
                collider
                    .to_polyline(RESOLUTION)
                    .into_iter()
                    .map(to_world)
                    .map(to_navmesh)
                    .collect(),
            ],
            TypedShape::Cuboid(collider) => vec![
                collider
                    .to_polyline()
                    .into_iter()
                    .map(to_world)
                    .map(to_navmesh)
                    .collect(),
            ],
            TypedShape::Capsule(collider) => vec![
                collider
                    .to_polyline(RESOLUTION)
                    .into_iter()
                    .map(to_world)
                    .map(to_navmesh)
                    .collect(),
            ],
            TypedShape::Triangle(collider) => vec![
                [collider.a, collider.b, collider.c]
                    .into_iter()
                    .map(to_world)
                    .map(to_navmesh)
                    .collect(),
            ],
            TypedShape::TriMesh(collider) => vec![
                collider
                    .vertices()
                    .iter()
                    .map(ref_to_world)
                    .map(to_navmesh)
                    .collect(),
            ],
            TypedShape::Polyline(collider) => vec![
                collider
                    .vertices()
                    .iter()
                    .map(ref_to_world)
                    .map(to_navmesh)
                    .collect(),
            ],
            // Each sub-shape is its own polygon, posed relative to the compound.
            TypedShape::Compound(collider) => collider
                .shapes()
                .iter()
                .flat_map(|(pose, shape)| {
                    shape.as_typed_shape().get_polygons(
                        obstacle_transform,
                        navmesh_transform,
                        (up, _shift),
                        &(local_pose * pose),
                    )
                })
                .collect(),
            TypedShape::ConvexPolygon(collider) => vec![
                collider
                    .points()
                    .iter()
                    .map(ref_to_world)
                    .map(to_navmesh)
                    .collect(),
            ],
            TypedShape::RoundCuboid(collider) => vec![
                collider
                    .to_polyline(RESOLUTION)
                    .into_iter()
                    .map(to_world)
                    .map(to_navmesh)
                    .collect(),
            ],
            TypedShape::RoundTriangle(collider) => vec![
                [
                    collider.inner_shape.a,
                    collider.inner_shape.b,
                    collider.inner_shape.c,
                ]
                .into_iter()
                .map(to_world)
                .map(to_navmesh)
                .collect(),
            ],
            TypedShape::RoundConvexPolygon(collider) => vec![
                collider
                    .to_polyline(RESOLUTION)
                    .into_iter()
                    .map(to_world)
                    .map(to_navmesh)
                    .collect(),
            ],
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
            TypedShape::Voxels(_) => {
                warn!("Voxels collider not supported for NavMesh obstacle generation");
                vec![]
            }
        }
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
    use super::*;

    fn polygons(collider: &Collider, navmesh_transform: &Transform) -> Vec<Vec<Vec2>> {
        // Matches how the updater derives `up` from the navmesh transform.
        let up = (navmesh_transform.forward(), 0.0);
        collider.get_polygons(&GlobalTransform::IDENTITY, navmesh_transform, up)
    }

    fn centroid(polygon: &[Vec2]) -> Vec2 {
        polygon.iter().sum::<Vec2>() / polygon.len() as f32
    }

    // A compound used to be flattened into a single polygon stringing every
    // sub-shape's points together, which fused disjoint obstacles into one.
    #[test]
    fn compound_yields_one_polygon_per_sub_shape() {
        let compound = Collider::compound(vec![
            (Vec2::new(-10.0, 0.0), 0.0, Collider::rectangle(2.0, 2.0)),
            (Vec2::new(10.0, 0.0), 0.0, Collider::rectangle(2.0, 2.0)),
        ]);

        let polygons = polygons(&compound, &Transform::IDENTITY);

        assert_eq!(polygons.len(), 2);
        for polygon in &polygons {
            assert_eq!(polygon.len(), 4);
        }
        let distance = centroid(&polygons[0]).distance(centroid(&polygons[1]));
        assert!(
            (distance - 20.0).abs() < 1e-3,
            "sub-shapes should stay 20 units apart, got {distance}"
        );
    }

    // The sub-shape pose used to be dropped entirely, collapsing every
    // sub-shape onto the compound's origin.
    #[test]
    fn compound_applies_sub_shape_translation() {
        let at_origin = Collider::compound(vec![(Vec2::ZERO, 0.0, Collider::rectangle(2.0, 2.0))]);
        let offset = Collider::compound(vec![(
            Vec2::new(10.0, 0.0),
            0.0,
            Collider::rectangle(2.0, 2.0),
        )]);

        let origin_centroid = centroid(&polygons(&at_origin, &Transform::IDENTITY)[0]);
        let offset_centroid = centroid(&polygons(&offset, &Transform::IDENTITY)[0]);

        let distance = origin_centroid.distance(offset_centroid);
        assert!(
            (distance - 10.0).abs() < 1e-3,
            "translated sub-shape should move 10 units, got {distance}"
        );
    }

    #[test]
    fn compound_applies_sub_shape_rotation() {
        let extents = |polygon: &[Vec2]| {
            let (min, max) = polygon.iter().fold(
                (Vec2::splat(f32::MAX), Vec2::splat(f32::MIN)),
                |(min, max), p| (min.min(*p), max.max(*p)),
            );
            max - min
        };

        // A 4x1 rectangle turned a quarter turn covers 1x4 instead.
        let upright = Collider::compound(vec![(Vec2::ZERO, 0.0, Collider::rectangle(4.0, 1.0))]);
        let turned = Collider::compound(vec![(
            Vec2::ZERO,
            std::f32::consts::FRAC_PI_2,
            Collider::rectangle(4.0, 1.0),
        )]);

        let upright = extents(&polygons(&upright, &Transform::IDENTITY)[0]);
        let turned = extents(&polygons(&turned, &Transform::IDENTITY)[0]);

        assert!(
            (upright.x - turned.y).abs() < 1e-3 && (upright.y - turned.x).abs() < 1e-3,
            "rotating a quarter turn should swap the extents: {upright} vs {turned}"
        );
    }

    // A compound wrapping a single shape at the origin must be indistinguishable
    // from that shape on its own.
    #[test]
    fn compound_of_one_matches_bare_shape() {
        let bare = Collider::rectangle(3.0, 2.0);
        let wrapped = Collider::compound(vec![(Vec2::ZERO, 0.0, Collider::rectangle(3.0, 2.0))]);

        let navmesh_transform = Transform::from_xyz(1.0, 5.0, -2.0);
        let bare = polygons(&bare, &navmesh_transform);
        let wrapped = polygons(&wrapped, &navmesh_transform);

        assert_eq!(bare.len(), wrapped.len());
        for (bare, wrapped) in bare.iter().zip(wrapped.iter()) {
            assert_eq!(bare.len(), wrapped.len());
            for (bare, wrapped) in bare.iter().zip(wrapped.iter()) {
                assert!(
                    bare.distance(*wrapped) < 1e-3,
                    "expected {bare}, got {wrapped}"
                );
            }
        }
    }
}
