pub mod math;
pub mod primitives;
pub mod transform;

use bevy::prelude::*;
use itertools::Either;
use nalgebra::{Const, OPoint};
use parry2d::{
    query::Unsupported,
    shape::{RoundShape, SharedShape, TypedShape},
};

use crate::{
    obstacles::{
        RESOLUTION,
        parry2d::{
            math::{AdjustPrecision, AsF32, Scalar, Vector, make_isometry},
            primitives::{EllipseColliderShape, RegularPolygonColliderShape},
            transform::Rotation,
        },
    },
    prelude::ObstacleSource,
    world_to_mesh,
};

/// A sharedshape obstacle that can be used to create a [`NavMesh`].
/// something defined in avian2d
#[derive(Debug, Clone, Component)]
pub struct SharedShapeStorage {
    /// The raw unscaled collider shape.
    pub shape: SharedShape,
    /// The scaled version of the collider shape.
    ///
    /// If the scale is `Vector::ONE`, this will be `None` and `unscaled_shape`
    /// will be used instead.
    scaled_shape: SharedShape,
    /// The global scale used for the collider shape.
    scale: Vec2,
}

impl From<SharedShape> for SharedShapeStorage {
    fn from(value: SharedShape) -> Self {
        Self {
            shape: value.clone(),
            scaled_shape: value,
            scale: Vec2::ONE,
        }
    }
}

impl SharedShapeStorage {
    /// Returns the raw unscaled shape of the collider.
    pub fn shape(&self) -> &SharedShape {
        &self.shape
    }

    /// Returns the shape of the collider with the scale from its `GlobalTransform` applied.
    pub fn shape_scaled(&self) -> &SharedShape {
        &self.scaled_shape
    }

    /// Sets the unscaled shape of the collider. The collider's scale will be applied to this shape.
    pub fn set_shape(&mut self, shape: SharedShape) {
        self.shape = shape;

        // TODO: The number of subdivisions probably shouldn't be hard-coded
        if let Ok(scaled) = scale_shape(&self.shape, self.scale, 10) {
            self.scaled_shape = scaled;
        } else {
            log::error!("Failed to create convex hull for scaled collider.");
        }
    }

    /// Returns the global scale of the collider.
    pub fn scale(&self) -> Vec2 {
        self.scale
    }

    /// Creates a collider with a circle shape defined by its radius.
    pub fn circle(radius: Scalar) -> Self {
        SharedShape::ball(radius).into()
    }

    /// Creates a collider with an ellipse shape defined by a half-width and half-height.
    pub fn ellipse(half_width: Scalar, half_height: Scalar) -> Self {
        SharedShape::new(EllipseColliderShape(Ellipse::new(half_width, half_height))).into()
    }

    /// Creates a collider with a rectangle shape defined by its extents.
    pub fn rectangle(x_length: Scalar, y_length: Scalar) -> Self {
        SharedShape::cuboid(x_length * 0.5, y_length * 0.5).into()
    }

    /// Creates a collider with a rectangle shape defined by its extents and rounded corners.
    pub fn round_rectangle(x_length: Scalar, y_length: Scalar, border_radius: Scalar) -> Self {
        SharedShape::round_cuboid(x_length * 0.5, y_length * 0.5, border_radius).into()
    }

    /// Creates a collider with a capsule shape defined by its radius
    /// and its height along the `Y` axis, excluding the hemispheres.
    pub fn capsule(radius: Scalar, length: Scalar) -> Self {
        SharedShape::capsule(
            (Vector::Y * length * 0.5).into(),
            (Vector::NEG_Y * length * 0.5).into(),
            radius,
        )
        .into()
    }

    /// Creates a collider with a capsule shape defined by its radius and endpoints `a` and `b`.
    pub fn capsule_endpoints(radius: Scalar, a: Vector, b: Vector) -> Self {
        SharedShape::capsule(a.into(), b.into(), radius).into()
    }

    /// Creates a collider with a [half-space](https://en.wikipedia.org/wiki/Half-space_(geometry)) shape
    /// defined by the outward normal of its planar boundary.
    pub fn half_space(outward_normal: Vector) -> Self {
        SharedShape::halfspace(nalgebra::Unit::new_normalize(outward_normal.into())).into()
    }

    /// Creates a collider with a segment shape defined by its endpoints `a` and `b`.
    pub fn segment(a: Vector, b: Vector) -> Self {
        SharedShape::segment(a.into(), b.into()).into()
    }

    /// Creates a collider with a triangle shape defined by its points `a`, `b`, and `c`.
    ///
    /// If the triangle is oriented clockwise, it will be reversed to be counterclockwise
    /// by swapping `b` and `c`. This is needed for collision detection.
    ///
    /// If you know that the given points produce a counterclockwise triangle,
    /// consider using [`Collider::triangle_unchecked`] instead.
    pub fn triangle(a: Vector, b: Vector, c: Vector) -> Self {
        let mut triangle = parry2d::shape::Triangle::new(a.into(), b.into(), c.into());

        // Make sure the triangle is counterclockwise. This is needed for collision detection.
        if triangle.orientation(1e-8) == parry2d::shape::TriangleOrientation::Clockwise {
            triangle.reverse();
        }

        SharedShape::new(triangle).into()
    }

    /// Creates a collider with a triangle shape defined by its points `a`, `b`, and `c`.
    ///
    /// The orientation of the triangle is assumed to be counterclockwise.
    /// This is needed for collision detection.
    ///
    /// If you are unsure about the orientation of the triangle, consider using [`Collider::triangle`] instead.
    pub fn triangle_unchecked(a: Vector, b: Vector, c: Vector) -> Self {
        SharedShape::triangle(a.into(), b.into(), c.into()).into()
    }

    /// Creates a collider with a regular polygon shape defined by the circumradius and the number of sides.
    pub fn regular_polygon(circumradius: f32, sides: u32) -> Self {
        SharedShape::new(RegularPolygonColliderShape(RegularPolygon::new(
            circumradius,
            sides,
        )))
        .into()
    }

    /// Creates a collider with a polyline shape defined by its vertices and optionally an index buffer.
    pub fn polyline(vertices: Vec<Vector>, indices: Option<Vec<[u32; 2]>>) -> Self {
        let vertices = vertices.into_iter().map(|v| v.into()).collect();
        SharedShape::polyline(vertices, indices).into()
    }
}
pub fn scale_shape(
    shape: &SharedShape,
    scale: Vector,
    num_subdivisions: u32,
) -> Result<SharedShape, Unsupported> {
    let scale = scale.abs();
    match shape.as_typed_shape() {
        TypedShape::Cuboid(s) => Ok(SharedShape::new(s.scaled(&scale.abs().into()))),
        TypedShape::RoundCuboid(s) => Ok(SharedShape::new(RoundShape {
            border_radius: s.border_radius,
            inner_shape: s.inner_shape.scaled(&scale.abs().into()),
        })),
        TypedShape::Capsule(c) => match c.scaled(&scale.abs().into(), num_subdivisions) {
            None => {
                log::error!("Failed to apply scale {} to Capsule shape.", scale);
                Ok(SharedShape::ball(0.0))
            }
            Some(Either::Left(b)) => Ok(SharedShape::new(b)),
            Some(Either::Right(b)) => Ok(SharedShape::new(b)),
        },
        TypedShape::Ball(b) => {
            {
                if scale.x == scale.y {
                    Ok(SharedShape::ball(b.radius * scale.x.abs()))
                } else {
                    // A 2D circle becomes an ellipse when scaled non-uniformly.
                    Ok(SharedShape::new(EllipseColliderShape(Ellipse {
                        half_size: Vec2::splat(b.radius) * scale.f32().abs(),
                    })))
                }
            }
        }
        TypedShape::Segment(s) => Ok(SharedShape::new(s.scaled(&scale.into()))),
        TypedShape::Triangle(t) => Ok(SharedShape::new(t.scaled(&scale.into()))),
        TypedShape::RoundTriangle(t) => Ok(SharedShape::new(RoundShape {
            border_radius: t.border_radius,
            inner_shape: t.inner_shape.scaled(&scale.into()),
        })),
        TypedShape::TriMesh(t) => Ok(SharedShape::new(t.clone().scaled(&scale.into()))),
        TypedShape::Polyline(p) => Ok(SharedShape::new(p.clone().scaled(&scale.into()))),
        TypedShape::HalfSpace(h) => match h.scaled(&scale.into()) {
            None => {
                log::error!("Failed to apply scale {} to HalfSpace shape.", scale);
                Ok(SharedShape::ball(0.0))
            }
            Some(scaled) => Ok(SharedShape::new(scaled)),
        },
        TypedShape::HeightField(h) => Ok(SharedShape::new(h.clone().scaled(&scale.into()))),
        TypedShape::ConvexPolygon(cp) => match cp.clone().scaled(&scale.into()) {
            None => {
                log::error!("Failed to apply scale {} to ConvexPolygon shape.", scale);
                Ok(SharedShape::ball(0.0))
            }
            Some(scaled) => Ok(SharedShape::new(scaled)),
        },
        TypedShape::RoundConvexPolygon(cp) => match cp.inner_shape.clone().scaled(&scale.into()) {
            None => {
                log::error!(
                    "Failed to apply scale {} to RoundConvexPolygon shape.",
                    scale
                );
                Ok(SharedShape::ball(0.0))
            }
            Some(scaled) => Ok(SharedShape::new(RoundShape {
                border_radius: cp.border_radius,
                inner_shape: scaled,
            })),
        },

        TypedShape::Compound(c) => {
            let mut scaled = Vec::with_capacity(c.shapes().len());

            for (iso, shape) in c.shapes() {
                scaled.push((
                    make_isometry(
                        Vector::from(iso.translation) * scale,
                        Rotation::radians(iso.rotation.angle()),
                    ),
                    scale_shape(shape, scale, num_subdivisions)?,
                ));
            }
            Ok(SharedShape::compound(scaled))
        }
        TypedShape::Custom(_shape) => {
            {
                if let Some(ellipse) = _shape.as_shape::<EllipseColliderShape>() {
                    return Ok(SharedShape::new(EllipseColliderShape(Ellipse {
                        half_size: ellipse.half_size * scale.f32().abs(),
                    })));
                }
                if let Some(polygon) = _shape.as_shape::<RegularPolygonColliderShape>() {
                    if scale.x == scale.y {
                        return Ok(SharedShape::new(RegularPolygonColliderShape(
                            RegularPolygon::new(
                                polygon.circumradius() * scale.x.abs(),
                                polygon.sides,
                            ),
                        )));
                    } else {
                        let vertices = polygon
                            .vertices(0.0)
                            .into_iter()
                            .map(|v| v.adjust_precision().into())
                            .collect::<Vec<_>>();

                        return scale_shape(
                            &SharedShape::convex_hull(&vertices).unwrap(),
                            scale,
                            num_subdivisions,
                        );
                    }
                }
            }
            Err(Unsupported)
        }
        TypedShape::Voxels(_) => Err(Unsupported),
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
            TypedShape::Voxels(_) => {
                warn!("Custom collider not supported for NavMesh obstacle generation");
                vec![]
            }
        }
    }
}

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
