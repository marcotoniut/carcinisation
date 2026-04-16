//! Spatial placement components: depth, floor, speed, and position types.
#![allow(clippy::wrong_self_convention)]

use crate::layer::{Layer, MidDepth, PreBackgroundDepth};
use bevy::prelude::*;
use num_enum::TryFromPrimitive;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    ops::{Add, Sub},
};
use strum_macros::EnumIter;

#[derive(
    Component,
    Debug,
    Deserialize,
    Clone,
    Copy,
    PartialEq,
    EnumIter,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Reflect,
    TryFromPrimitive,
    Serialize,
)]
#[repr(i8)]
pub enum Depth {
    Nine = 9,
    Eight = 8,
    Seven = 7,
    Six = 6,
    Five = 5,
    Four = 4,
    Three = 3,
    Two = 2,
    One = 1,
    Zero = 0,
}

impl Default for Depth {
    fn default() -> Self {
        Self::MAX
    }
}

impl Add<i8> for Depth {
    type Output = Depth;

    fn add(self, other: i8) -> Depth {
        let value = (self as i8 + other)
            .min(Depth::MAX.to_i8())
            .max(Depth::MIN.to_i8());
        Depth::try_from(value).unwrap_or(Depth::MAX)
    }
}

impl Sub<i8> for Depth {
    type Output = Depth;

    fn sub(self, other: i8) -> Depth {
        let value = (self as i8 - other)
            .min(Depth::MAX.to_i8())
            .max(Depth::MIN.to_i8());
        Depth::try_from(value).unwrap_or(Depth::MIN)
    }
}

impl Depth {
    pub const MAX: Self = Self::Nine;
    pub const MIN: Self = Self::Zero;

    #[must_use]
    pub fn to_f32(&self) -> f32 {
        f32::from(self.to_i8())
    }

    #[must_use]
    pub fn to_i8(&self) -> i8 {
        *self as i8
    }

    #[must_use]
    pub fn to_layer(&self) -> Layer {
        match self {
            Self::Nine => Layer::PreBackgroundDepth(PreBackgroundDepth::Nine),
            Self::Eight => Layer::PreBackgroundDepth(PreBackgroundDepth::Eight),
            Self::Seven => Layer::PreBackgroundDepth(PreBackgroundDepth::Seven),
            Self::Six => Layer::MidDepth(MidDepth::Six),
            Self::Five => Layer::MidDepth(MidDepth::Five),
            Self::Four => Layer::MidDepth(MidDepth::Four),
            Self::Three => Layer::MidDepth(MidDepth::Three),
            Self::Two => Layer::MidDepth(MidDepth::Two),
            Self::One => Layer::MidDepth(MidDepth::One),
            Self::Zero => Layer::MidDepth(MidDepth::Zero),
        }
    }
}

/// The set of visible depths for which an entity has authored/hand-made visuals.
///
/// Used by the fallback depth-scale system: when an entity's current [`Depth`]
/// is **not** in this set, a render-only presentation scale is applied based on
/// the [`DepthScaleConfig`](crate::stage::depth_scale::DepthScaleConfig).
///
/// The fallback reference depth is chosen as:
/// 1. The nearest **shallower** (numerically smaller) authored depth, or
/// 2. If none exists, the nearest **deeper** (numerically larger) authored depth.
///
/// Only meaningful for visible depths 1..=9. Depth 0 is excluded from
/// normal fallback scaling.
///
/// # Examples
///
/// ```ignore
/// // Entity with visuals authored for depth 3 only:
/// AuthoredDepths::single(Depth::Three)
///
/// // Entity with visuals for depths 3 and 6:
/// AuthoredDepths::new(vec![Depth::Three, Depth::Six])
/// ```
#[derive(Component, Debug, Clone, PartialEq, Eq, Reflect, Serialize, Deserialize)]
pub struct AuthoredDepths(pub Vec<Depth>);

impl AuthoredDepths {
    /// Create from a list of authored depths. Duplicates are removed and
    /// the list is sorted (shallowest first, i.e. numerically ascending).
    #[must_use]
    pub fn new(mut depths: Vec<Depth>) -> Self {
        depths.sort();
        depths.dedup();
        Self(depths)
    }

    /// Convenience: a single authored depth.
    #[must_use]
    pub fn single(depth: Depth) -> Self {
        Self(vec![depth])
    }

    /// Returns `true` if the given depth has authored visuals.
    #[must_use]
    pub fn contains(&self, depth: Depth) -> bool {
        self.0.contains(&depth)
    }

    /// Find the best fallback reference depth for a target depth that is
    /// **not** in the authored set.
    ///
    /// Prefers the nearest shallower (numerically smaller) authored depth.
    /// Falls back to the nearest deeper (numerically larger) if no shallower
    /// one exists.
    ///
    /// Returns `None` if the set is empty.
    #[must_use]
    pub fn resolve_reference(&self, target: Depth) -> Option<Depth> {
        let target_i = target.to_i8();
        let mut nearest_shallower: Option<Depth> = None;
        let mut nearest_deeper: Option<Depth> = None;

        for &d in &self.0 {
            let d_i = d.to_i8();
            if d_i <= target_i {
                // Shallower or equal — keep the closest (largest that's ≤ target).
                if nearest_shallower.is_none_or(|prev| d_i > prev.to_i8()) {
                    nearest_shallower = Some(d);
                }
            } else {
                // Deeper — keep the closest (smallest that's > target).
                if nearest_deeper.is_none_or(|prev| d_i < prev.to_i8()) {
                    nearest_deeper = Some(d);
                }
            }
        }

        nearest_shallower.or(nearest_deeper)
    }

    /// Returns `true` if the set is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[derive(Component, Debug, Clone, Copy, Reflect)]
pub struct Floor(pub f32);

/// State-based anchor offsets for composed entities.
///
/// Both values are distances **downward from the composition origin** in
/// canvas pixels (positive = below origin).  Each defines a semantically
/// meaningful local reference point on the sprite.
///
/// Inserted by [`update_composed_enemy_visuals`] when the atlas is first
/// resolved.  Not present before that — consumers should query with
/// `Option<&AnchorOffsets>`.  For `BottomOrigin` entities both values are
/// zero (entity position IS the ground contact).
///
/// # Placement
///
/// - **Grounded states** (walk, idle on land):
///   `entity_y = floor_y + ground × scale`
///   — the ground anchor sits on the floor.
/// - **Airborne states** (fly, hover):
///   `entity_y = airborne_ref_y + air × scale`
///   — the air anchor sits at the flight reference point.
///
/// How `airborne_ref_y` is determined is up to the caller.  In
/// `depth_traverse` it is still floor-relative (`floor_y + altitude ×
/// scale`); a future stage runner could supply an independent target.
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct AnchorOffsets {
    /// Distance from composition origin to ground contact point (positive =
    /// below origin).  Grounded states place this point on the floor.
    pub ground: f32,
    /// Distance from composition origin to airborne pivot (positive = below
    /// origin).  Derived from the body part centre (e.g. collision hurtbox
    /// centre Y offset).
    pub air: f32,
}

impl AnchorOffsets {
    /// Returns the active anchor offset based on whether the entity is airborne.
    #[must_use]
    pub fn active_offset(&self, airborne: bool) -> f32 {
        if airborne { self.air } else { self.ground }
    }
}

/// Marker for entities that are currently airborne.
///
/// When present, placement and debug systems use [`AnchorOffsets::air`]
/// instead of [`AnchorOffsets::ground`].  Movement systems add this
/// marker on liftoff and remove it on landing.
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct Airborne;

#[derive(Component, Debug, Clone, Copy, Reflect)]
pub struct Speed(pub f32);

#[derive(Component, Debug)]
pub struct InView;

#[derive(Component, Debug)]
pub struct LinearUpdateDisabled;

pub fn spawn_floor_depths<S: std::hash::BuildHasher>(
    commands: &mut Commands,
    floor_depths: &HashMap<Depth, f32, S>,
) {
    for (depth, y) in floor_depths {
        commands.spawn((Floor(*y), *depth));
    }
}
