pub mod error;
pub mod math;
pub mod primitives;
pub mod shape;
pub mod transform;

use bevy::prelude::*;
use nalgebra::{Const, OPoint};
use parry2d::shape::TypedShape;

use crate::{
    obstacles::{
        RESOLUTION,
        parry2d::primitives::{EllipseShape, RegularPolygonShape},
    },
    prelude::{ObstacleSource, SharedShapeStorage},
    world_to_mesh,
};

impl ObstacleSource for SharedShapeStorage {
    fn get_polygons(
        &self,
        obstacle_transform: &GlobalTransform,
        navmesh_transform: &Transform,
        up: (Dir3, f32),
    ) -> Vec<Vec<Vec2>> {
        vec![self.shape_scaled().as_typed_shape().get_polygon(
            obstacle_transform,
            navmesh_transform,
            up,
        )]
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

impl InnerObstacleSource for TypedShape<'_> {
    fn get_polygon(
        &self,
        obstacle_transform: &GlobalTransform,
        navmesh_transform: &Transform,
        (up, _shift): (Dir3, f32),
    ) -> Vec<Vec2> {
        let mut transform = obstacle_transform.compute_transform();
        transform.scale = Vec3::ONE;
        let world_to_mesh = world_to_mesh(navmesh_transform);

        let ref_to_world = |p: &OPoint<f32, Const<2>>| {
            let mut v = vec3(p.x, 0.0, p.y);
            v = if up.is_negative_bitmask().count_ones() % 2 == 1 {
                Quat::from_rotation_arc(Vec3::Y, up.into()).mul_vec3(v)
            } else {
                Quat::from_rotation_arc(-Vec3::Y, up.into()).mul_vec3(v)
            };
            transform.transform_point(v)
        };
        let to_world = |p: OPoint<f32, Const<2>>| ref_to_world(&p);

        let to_navmesh = |v: Vec3| world_to_mesh.transform_point3(v).xy();

        match self {
            TypedShape::Ball(shape) => shape
                .to_polyline(RESOLUTION)
                .into_iter()
                .map(to_world)
                .map(to_navmesh)
                .collect(),
            TypedShape::Cuboid(shape) => shape
                .to_polyline()
                .into_iter()
                .map(to_world)
                .map(to_navmesh)
                .collect(),
            TypedShape::Capsule(shape) => shape
                .to_polyline(RESOLUTION)
                .into_iter()
                .map(to_world)
                .map(to_navmesh)
                .collect(),
            TypedShape::Triangle(shape) => [shape.a, shape.b, shape.c]
                .into_iter()
                .map(to_world)
                .map(to_navmesh)
                .collect(),
            TypedShape::TriMesh(shape) => shape
                .vertices()
                .iter()
                .map(ref_to_world)
                .map(to_navmesh)
                .collect(),
            TypedShape::Polyline(shape) => shape
                .vertices()
                .iter()
                .map(ref_to_world)
                .map(to_navmesh)
                .collect(),
            TypedShape::Compound(shape) => shape
                .shapes()
                .iter()
                .flat_map(|(_iso, shape)| {
                    // TODO: handle the isometry of each shape
                    warn!("TODO: handle the isometry of each shape");
                    shape.as_typed_shape().get_polygon(
                        obstacle_transform,
                        navmesh_transform,
                        (up, _shift),
                    )
                })
                .collect(),
            TypedShape::ConvexPolygon(shape) => shape
                .points()
                .iter()
                .map(ref_to_world)
                .map(to_navmesh)
                .collect(),
            TypedShape::RoundCuboid(shape) => shape
                .to_polyline(RESOLUTION)
                .into_iter()
                .map(to_world)
                .map(to_navmesh)
                .collect(),
            TypedShape::RoundTriangle(shape) => [
                shape.inner_shape.a,
                shape.inner_shape.b,
                shape.inner_shape.c,
            ]
            .into_iter()
            .map(to_world)
            .map(to_navmesh)
            .collect(),
            TypedShape::RoundConvexPolygon(shape) => shape
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
            TypedShape::Custom(custom) => {
                if custom.is::<RegularPolygonShape>() {
                    let regular_polygon_shape = custom
                        .as_shape::<RegularPolygonShape>()
                        .expect("the custom shape should be a RegularPolygonShape");

                    (0..=regular_polygon_shape.sides)
                        .map(|p| {
                            copypasta::single_circle_coordinate(
                                regular_polygon_shape.circumcircle.radius,
                                regular_polygon_shape.sides,
                                p.try_into().unwrap(),
                            )
                        })
                        .map(|v| to_navmesh(to_world(v.into())))
                        .collect()
                } else if custom.is::<EllipseShape>() {
                    let ellipse_shape = custom
                        .as_shape::<EllipseShape>()
                        .expect("the custom shape should be a RegularPolygonShape");

                    copypasta::ellipse_inner(ellipse_shape.half_size, RESOLUTION)
                        .map(|v| to_navmesh(to_world(v.into())))
                        .collect()
                } else {
                    warn!("Custom collider not supported for NavMesh obstacle generation");
                    vec![]
                }
            }
            TypedShape::Voxels(_) => {
                warn!("Voxels collider not supported for NavMesh obstacle generation");
                vec![]
            }
        }
    }
}

// Functions in this module are copied from Bevy
mod copypasta {
    use std::f64::consts::TAU;

    use bevy::math::Vec2;

    pub(crate) fn ellipse_inner(half_size: Vec2, resolution: u32) -> impl Iterator<Item = Vec2> {
        (0..resolution + 1).map(move |i| {
            let angle = i as f64 * TAU / resolution as f64;
            let (x, y) = angle.sin_cos();
            Vec2::new(x as f32, y as f32) * half_size
        })
    }

    #[allow(dead_code)]
    pub(crate) fn arc_2d_inner(
        direction_angle: f64,
        arc_angle: f64,
        radius: f32,
        resolution: u32,
    ) -> impl Iterator<Item = Vec2> {
        (0..resolution + 1).map(move |i| {
            let start = direction_angle - arc_angle / 2.;

            let angle =
                start + (i as f64 * (arc_angle / resolution as f64)) + std::f64::consts::FRAC_PI_2;

            Vec2::new(angle.cos() as f32, angle.sin() as f32) * radius
        })
    }

    pub(crate) fn single_circle_coordinate(radius: f32, resolution: u32, nth_point: usize) -> Vec2 {
        let angle = nth_point as f64 * TAU / resolution as f64;
        let (x, y) = angle.sin_cos();
        Vec2::new(x as f32, y as f32) * radius
    }
}
