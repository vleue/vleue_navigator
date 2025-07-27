use std::f32::consts::{FRAC_PI_2, PI, TAU};

use crate::world_to_mesh;
use bevy::{
    math::{bounding::Bounded2d, vec3, Vec3Swizzles},
    prelude::*,
};
use itertools::Either;
use parry2d::{
    mass_properties::MassProperties,
    math::Isometry,
    na::{ArrayStorage, Const, Matrix, OPoint, Point2, U1, U2, UnitVector2},
    query::{
        PointQuery, RayCast, Unsupported,
        details::local_ray_intersection_with_support_map_with_params, gjk::VoronoiSimplex,
        point::local_point_projection_on_support_map,
    },
    shape::*,
};

use super::{ObstacleSource, RESOLUTION};
#[derive(Reflect, Clone, Copy, Component, Debug, PartialEq)]
#[reflect(Debug, Component, PartialEq)]
pub struct Rotation {
    /// The cosine of the rotation angle in radians.
    ///
    /// This is the real part of the unit complex number representing the rotation.
    pub cos: Scalar,
    /// The sine of the rotation angle in radians.
    ///
    /// This is the imaginary part of the unit complex number representing the rotation.
    pub sin: Scalar,
}
#[derive(Reflect, Clone, Copy, Component, Debug, Default, Deref, DerefMut, PartialEq)]
#[reflect(Debug, Component, Default, PartialEq)]
pub struct Position(pub Vec2);

pub type UnsupportedShape = Unsupported;
pub type Scalar = f32;
pub type Vector2<T> = Matrix<T, U2, U1, ArrayStorage<T, 2, 1>>;
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

fn scale_shape(
    shape: &SharedShape,
    scale: Vec2,
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
                        half_size: Vec2::splat(b.radius as f32) * scale.f32().abs(),
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
                        Vec2::from(iso.translation) * scale,
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
                                polygon.circumradius() * scale.x.abs() as f32,
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

pub(crate) fn make_isometry(
    position: impl Into<Position>,
    rotation: impl Into<Rotation>,
) -> Isometry<Scalar> {
    let position: Position = position.into();
    let rotation: Rotation = rotation.into();
    Isometry::<Scalar>::new(position.0.into(), rotation.into())
}

/// Adjust the precision of the math construct to the precision chosen for compilation.
pub trait AdjustPrecision {
    /// A math construct type with the desired precision.
    type Adjusted;
    /// Adjusts the precision of [`self`] to [`Self::Adjusted`](#associatedtype.Adjusted).
    fn adjust_precision(&self) -> Self::Adjusted;
}

impl AdjustPrecision for f32 {
    type Adjusted = Scalar;
    fn adjust_precision(&self) -> Self::Adjusted {
        *self
    }
}
impl AdjustPrecision for Vec2 {
    type Adjusted = Vec2;
    fn adjust_precision(&self) -> Self::Adjusted {
        *self
    }
}
/// Adjust the precision down to `f32` regardless of compilation.
pub trait AsF32 {
    /// The `f32` version of a math construct.
    type F32;
    /// Returns the `f32` version of this type.
    fn f32(&self) -> Self::F32;
}
impl AsF32 for Vec2 {
    type F32 = Self;
    fn f32(&self) -> Self::F32 {
        *self
    }
}
