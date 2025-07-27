//! Math types and traits used by the crate.
//!
//! Most of the math types are feature-dependent, so they will be different for `2d`/`3d` and `f32`/`f64`.

mod single;
pub use single::*;

use bevy::math::prelude::*;

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

impl AsF32 for Vec3 {
    type F32 = Self;
    fn f32(&self) -> Self::F32 {
        *self
    }
}

impl AsF32 for Vec2 {
    type F32 = Self;
    fn f32(&self) -> Self::F32 {
        *self
    }
}

#[expect(clippy::unnecessary_cast)]
pub(crate) fn na_iso_to_iso(isometry: &Isometry<Scalar>) -> Isometry2d {
    Isometry2d::new(
        Vector::from(isometry.translation).f32(),
        Rot2::from_sin_cos(isometry.rotation.im, isometry.rotation.re),
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
