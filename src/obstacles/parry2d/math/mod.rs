//! Math types and traits used by the crate.
//!
//! Most of the math types are feature-dependent, so they will be different for `2d`/`3d` and `f32`/`f64`.

mod single;
pub use single::*;

use bevy::math::{prelude::*, *};

/// The active dimension.
pub const DIM: usize = 2;
/// The active dimension.

/// The ray type chosen based on the dimension.
pub(crate) type Ray = Ray2d;

// Note: This is called `Dir` instead of `Direction` because Bevy has a conflicting `Direction` type.
/// The direction type chosen based on the dimension.
pub(crate) type Dir = Dir2;

/// The vector type for angular values chosen based on the dimension.
pub(crate) type AngularVector = Scalar;

/// The symmetric tensor type chosen based on the dimension.
/// Often used for angular inertia.
///
/// In 2D, this is a scalar, while in 3D, it is a 3x3 matrix.
pub(crate) type SymmetricTensor = Scalar;

/// Adjust the precision of the math construct to the precision chosen for compilation.
pub trait AdjustPrecision {
    /// A math construct type with the desired precision.
    type Adjusted;
    /// Adjusts the precision of [`self`] to [`Self::Adjusted`](#associatedtype.Adjusted).
    fn adjust_precision(&self) -> Self::Adjusted;
}

/// Adjust the precision down to `f32` regardless of compilation.
pub trait AsF32 {
    /// The `f32` version of a math construct.
    type F32;
    /// Returns the `f32` version of this type.
    fn f32(&self) -> Self::F32;
}

impl AsF32 for DVec3 {
    type F32 = Vec3;
    fn f32(&self) -> Self::F32 {
        self.as_vec3()
    }
}

impl AsF32 for Vec3 {
    type F32 = Self;
    fn f32(&self) -> Self::F32 {
        *self
    }
}

impl AsF32 for DVec2 {
    type F32 = Vec2;
    fn f32(&self) -> Self::F32 {
        self.as_vec2()
    }
}

impl AsF32 for Vec2 {
    type F32 = Self;
    fn f32(&self) -> Self::F32 {
        *self
    }
}

impl AsF32 for Vec4 {
    type F32 = Self;
    fn f32(&self) -> Self::F32 {
        *self
    }
}

impl AsF32 for DQuat {
    type F32 = Quat;
    fn f32(&self) -> Self::F32 {
        self.as_quat()
    }
}

impl AsF32 for Quat {
    type F32 = Self;
    fn f32(&self) -> Self::F32 {
        *self
    }
}

impl AsF32 for DMat2 {
    type F32 = Mat2;
    fn f32(&self) -> Self::F32 {
        self.as_mat2()
    }
}

impl AsF32 for Mat2 {
    type F32 = Self;
    fn f32(&self) -> Self::F32 {
        *self
    }
}

impl AsF32 for DMat3 {
    type F32 = Mat3;
    fn f32(&self) -> Self::F32 {
        self.as_mat3()
    }
}

impl AsF32 for Mat3 {
    type F32 = Self;
    fn f32(&self) -> Self::F32 {
        *self
    }
}

pub(crate) fn cross(a: Vec2, b: Vec2) -> Scalar {
    a.perp_dot(b)
}

/// An extension trait for computing reciprocals without division by zero.
pub trait RecipOrZero {
    /// Computes the reciprocal of `self` if `self` is not zero,
    /// and returns zero otherwise to avoid division by zero.
    fn recip_or_zero(self) -> Self;
}

impl RecipOrZero for f32 {
    #[inline]
    fn recip_or_zero(self) -> Self {
        if self != 0.0 && self.is_finite() {
            self.recip()
        } else {
            0.0
        }
    }
}

impl RecipOrZero for f64 {
    #[inline]
    fn recip_or_zero(self) -> Self {
        if self != 0.0 && self.is_finite() {
            self.recip()
        } else {
            0.0
        }
    }
}

impl RecipOrZero for Vec2 {
    #[inline]
    fn recip_or_zero(self) -> Self {
        Self::new(self.x.recip_or_zero(), self.y.recip_or_zero())
    }
}

impl RecipOrZero for Vec3 {
    #[inline]
    fn recip_or_zero(self) -> Self {
        Self::new(
            self.x.recip_or_zero(),
            self.y.recip_or_zero(),
            self.z.recip_or_zero(),
        )
    }
}

impl RecipOrZero for DVec2 {
    #[inline]
    fn recip_or_zero(self) -> Self {
        Self::new(self.x.recip_or_zero(), self.y.recip_or_zero())
    }
}

impl RecipOrZero for DVec3 {
    #[inline]
    fn recip_or_zero(self) -> Self {
        Self::new(
            self.x.recip_or_zero(),
            self.y.recip_or_zero(),
            self.z.recip_or_zero(),
        )
    }
}

#[expect(clippy::unnecessary_cast)]
pub(crate) fn na_iso_to_iso(isometry: &Isometry<Scalar>) -> Isometry2d {
    Isometry2d::new(
        Vector::from(isometry.translation).f32(),
        Rot2::from_sin_cos(isometry.rotation.im as f32, isometry.rotation.re as f32),
    )
}

use parry2d::math::Isometry;

use crate::obstacles::parry2d::transform::{Position, Rotation};

pub(crate) fn make_isometry(
    position: impl Into<Position>,
    rotation: impl Into<Rotation>,
) -> Isometry<Scalar> {
    let position: Position = position.into();
    let rotation: Rotation = rotation.into();
    Isometry::<Scalar>::new(position.0.into(), rotation.into())
}
