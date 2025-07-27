use super::AdjustPrecision;
use bevy::math::*;

/// The floating point number type used by Avian.
pub type Scalar = f32;
/// The PI/2 constant.
pub const FRAC_PI_2: Scalar = core::f32::consts::FRAC_PI_2;
/// The PI constant.
pub const PI: Scalar = core::f32::consts::PI;
/// The TAU constant.
pub const TAU: Scalar = core::f32::consts::TAU;

/// The vector type used by Avian.
pub type Vector = Vec2;
/// The vector type used by Avian. This is always a 2D vector regardless of the chosen dimension.
pub type Vector2 = Vec2;
/// The vector type used by Avian. This is always a 3D vector regardless of the chosen dimension.
pub type Vector3 = Vec3;
/// The `i32` vector type chosen based on the dimension.
pub type IVector = IVec2;

impl AdjustPrecision for f32 {
    type Adjusted = Scalar;
    fn adjust_precision(&self) -> Self::Adjusted {
        *self
    }
}

impl AdjustPrecision for Vec3 {
    type Adjusted = Vector3;
    fn adjust_precision(&self) -> Self::Adjusted {
        *self
    }
}

impl AdjustPrecision for Vec2 {
    type Adjusted = Vector2;
    fn adjust_precision(&self) -> Self::Adjusted {
        *self
    }
}
