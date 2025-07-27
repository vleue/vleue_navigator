use bevy::prelude::*;

use crate::obstacles::parry2d::math::{AdjustPrecision, Scalar, Vector, Vector2};

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
pub struct Position(pub Vector2);
impl Position {
    /// A placeholder position. This is an invalid position and should *not*
    /// be used to an actually position entities in the world, but can be used
    /// to indicate that a position has not yet been initialized.
    pub const PLACEHOLDER: Self = Self(Vector::MAX);

    /// Creates a [`Position`] component with the given global `position`.
    pub fn new(position: Vector) -> Self {
        Self(position)
    }

    /// Creates a [`Position`] component with the global position `(x, y)`.
    pub fn from_xy(x: Scalar, y: Scalar) -> Self {
        Self(Vector::new(x, y))
    }
}
impl From<Rotation> for Scalar {
    fn from(rot: Rotation) -> Self {
        rot.as_radians()
    }
}

impl Rotation {
    /// Returns the rotation in radians in the `(-pi, pi]` range.
    #[inline]
    pub fn as_radians(self) -> Scalar {
        Scalar::atan2(self.sin, self.cos)
    }
    /// Creates a [`Rotation`] from a counterclockwise angle in radians.
    #[inline]
    pub fn radians(radians: Scalar) -> Self {
        let (sin, cos) = radians.sin_cos();

        Self::from_sin_cos(sin, cos)
    }

    /// Creates a [`Rotation`] from the sine and cosine of an angle in radians.
    ///
    /// The rotation is only valid if `sin * sin + cos * cos == 1.0`.
    ///
    /// # Panics
    ///
    /// Panics if `sin * sin + cos * cos != 1.0` when `debug_assertions` are enabled.
    #[inline]
    pub fn from_sin_cos(sin: Scalar, cos: Scalar) -> Self {
        let rotation = Self { sin, cos };
        debug_assert!(
            rotation.is_normalized(),
            "the given sine and cosine produce an invalid rotation"
        );
        rotation
    }

    /// Returns whether `self` has a length of `1.0` or not.
    ///
    /// Uses a precision threshold of approximately `1e-4`.
    #[inline]
    pub fn is_normalized(self) -> bool {
        // The allowed length is 1 +/- 1e-4, so the largest allowed
        // squared length is (1 + 1e-4)^2 = 1.00020001, which makes
        // the threshold for the squared length approximately 2e-4.
        (self.length_squared() - 1.0).abs() <= 2e-4
    }
    /// Computes the squared length or norm of the complex number used to represent the rotation.
    ///
    /// This is generally faster than [`Rotation::length()`], as it avoids a square
    /// root operation.
    ///
    /// The length is typically expected to be `1.0`. Unexpectedly denormalized rotations
    /// can be a result of incorrect construction or floating point error caused by
    /// successive operations.
    #[inline]
    pub fn length_squared(self) -> Scalar {
        Vector::new(self.sin, self.cos).length_squared()
    }
}
impl From<GlobalTransform> for Position {
    fn from(value: GlobalTransform) -> Self {
        Self::from_xy(
            value.translation().adjust_precision().x,
            value.translation().adjust_precision().y,
        )
    }
}

impl From<&GlobalTransform> for Position {
    fn from(value: &GlobalTransform) -> Self {
        Self::from_xy(
            value.translation().adjust_precision().x,
            value.translation().adjust_precision().y,
        )
    }
}

impl Ease for Position {
    fn interpolating_curve_unbounded(start: Self, end: Self) -> impl Curve<Self> {
        FunctionCurve::new(Interval::UNIT, move |t| {
            Position(Vector::lerp(start.0, end.0, t))
        })
    }
}

impl From<Vector> for Position {
    fn from(val: Vector) -> Self {
        Position(val.xy())
    }
}
