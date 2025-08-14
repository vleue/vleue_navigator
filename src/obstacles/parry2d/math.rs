//! Math types and traits used by the crate.
//!
//! Most of the math types are feature-dependent, so they will be different for `2d`/`3d` and `f32`/`f64`.

use bevy::math::prelude::*;
use parry2d::math::Real;

pub(crate) fn na_iso_to_iso(isometry: &Isometry<Real>) -> Isometry2d {
    Isometry2d::new(
        Vec2::from(isometry.translation),
        Rot2::from_sin_cos(isometry.rotation.im, isometry.rotation.re),
    )
}

use parry2d::math::Isometry;

use crate::obstacles::parry2d::transform::{Position, Rotation};

pub(crate) fn make_isometry(
    position: impl Into<Position>,
    rotation: impl Into<Rotation>,
) -> Isometry<Real> {
    let position: Position = position.into();
    let rotation: Rotation = rotation.into();
    Isometry::<Real>::new(position.0.into(), rotation.into())
}
