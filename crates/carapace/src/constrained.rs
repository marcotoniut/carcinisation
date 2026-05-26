//! Lightweight constrained newtypes for serde-boundary validation.
//!
//! These types reject invalid values during deserialisation so configs
//! are guaranteed valid by construction — no post-load `validate()` needed
//! for the constraints they encode.

use bevy_reflect::{ReflectDeserialize, ReflectSerialize, impl_reflect_opaque};
use serde::de;
use std::fmt;

// ---------------------------------------------------------------------------
// FiniteF32
// ---------------------------------------------------------------------------

/// An `f32` that is guaranteed finite (not NaN, not ±infinity).
///
/// Deserialisation rejects non-finite values. RON syntax is unchanged:
/// just write the number normally.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct FiniteF32(f32);

impl FiniteF32 {
    /// Create a `FiniteF32`, returning `None` if the value is non-finite.
    #[must_use]
    pub fn new(value: f32) -> Option<Self> {
        value.is_finite().then_some(Self(value))
    }

    /// Return the inner `f32`.
    #[inline]
    #[must_use]
    pub const fn get(self) -> f32 {
        self.0
    }
}

impl From<FiniteF32> for f32 {
    #[inline]
    fn from(v: FiniteF32) -> Self {
        v.0
    }
}

impl fmt::Display for FiniteF32 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<'de> de::Deserialize<'de> for FiniteF32 {
    fn deserialize<D: de::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = f32::deserialize(deserializer)?;
        Self::new(value)
            .ok_or_else(|| de::Error::custom(format!("expected finite f32, got {value}")))
    }
}

impl serde::Serialize for FiniteF32 {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl_reflect_opaque!(::carapace::constrained::FiniteF32(
    Clone,
    Debug,
    PartialEq,
    Serialize,
    Deserialize,
));

// ---------------------------------------------------------------------------
// PositiveFiniteF32
// ---------------------------------------------------------------------------

/// An `f32` that is guaranteed positive (> 0.0) and finite.
///
/// Deserialisation rejects zero, negative, NaN, and ±infinity.
/// RON syntax is unchanged: just write the number normally.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct PositiveFiniteF32(f32);

impl PositiveFiniteF32 {
    /// Create a `PositiveFiniteF32`, returning `None` if the value is not
    /// positive and finite.
    #[must_use]
    pub fn new(value: f32) -> Option<Self> {
        (value.is_finite() && value > 0.0).then_some(Self(value))
    }

    /// Return the inner `f32`.
    #[inline]
    #[must_use]
    pub const fn get(self) -> f32 {
        self.0
    }
}

impl From<PositiveFiniteF32> for f32 {
    #[inline]
    fn from(v: PositiveFiniteF32) -> Self {
        v.0
    }
}

impl fmt::Display for PositiveFiniteF32 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<'de> de::Deserialize<'de> for PositiveFiniteF32 {
    fn deserialize<D: de::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = f32::deserialize(deserializer)?;
        Self::new(value)
            .ok_or_else(|| de::Error::custom(format!("expected positive finite f32, got {value}")))
    }
}

impl serde::Serialize for PositiveFiniteF32 {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl_reflect_opaque!(::carapace::constrained::PositiveFiniteF32(
    Clone,
    Debug,
    PartialEq,
    Serialize,
    Deserialize,
));

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finite_f32_rejects_nan() {
        assert!(FiniteF32::new(f32::NAN).is_none());
    }

    #[test]
    fn finite_f32_rejects_infinity() {
        assert!(FiniteF32::new(f32::INFINITY).is_none());
        assert!(FiniteF32::new(f32::NEG_INFINITY).is_none());
    }

    #[test]
    fn finite_f32_accepts_normal() {
        assert_eq!(FiniteF32::new(1.5).unwrap().get(), 1.5);
        assert_eq!(FiniteF32::new(0.0).unwrap().get(), 0.0);
        assert_eq!(FiniteF32::new(-3.0).unwrap().get(), -3.0);
    }

    #[test]
    fn finite_f32_ron_round_trip() {
        let v = FiniteF32::new(42.5).unwrap();
        let s = ron::to_string(&v).unwrap();
        let parsed: FiniteF32 = ron::from_str(&s).unwrap();
        assert_eq!(v, parsed);
    }

    #[test]
    fn positive_finite_f32_rejects_zero() {
        assert!(PositiveFiniteF32::new(0.0).is_none());
    }

    #[test]
    fn positive_finite_f32_rejects_negative() {
        assert!(PositiveFiniteF32::new(-1.0).is_none());
    }

    #[test]
    fn positive_finite_f32_rejects_nan() {
        assert!(PositiveFiniteF32::new(f32::NAN).is_none());
    }

    #[test]
    fn positive_finite_f32_rejects_infinity() {
        assert!(PositiveFiniteF32::new(f32::INFINITY).is_none());
    }

    #[test]
    fn positive_finite_f32_accepts_positive() {
        assert_eq!(PositiveFiniteF32::new(1.5).unwrap().get(), 1.5);
        assert_eq!(PositiveFiniteF32::new(0.001).unwrap().get(), 0.001);
    }

    #[test]
    fn positive_finite_f32_ron_round_trip() {
        let v = PositiveFiniteF32::new(42.5).unwrap();
        let s = ron::to_string(&v).unwrap();
        let parsed: PositiveFiniteF32 = ron::from_str(&s).unwrap();
        assert_eq!(v, parsed);
    }
}
