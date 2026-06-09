//! Per-part, per-facing target collision metadata.
//!
//! Provides the metadata layer between 2D collision primitives and gameplay
//! damage routing. Collision data is indexed by animation key, frame index,
//! and billboard facing — the facing is selected per attacker/query so two
//! players may hit-test the same enemy using different facings in the same
//! server tick.
//!
//! Geometry ([`PartCollider2d`]) is separate from gameplay metadata
//! ([`PartMetadata`]) so the collision kernel stays rendering-independent.

use std::collections::HashMap;

use bevy_math::Vec2;

use super::nearest;
use super::primitives::{Collider, HitResult};

// ---------------------------------------------------------------------------
// Lightweight identifiers
// ---------------------------------------------------------------------------

/// Opaque part identifier assigned by the authoring pipeline.
///
/// Numerically ordered for deterministic tie-breaking in nearest-hit queries.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PartId(pub u16);

/// Opaque material identifier for future damage routing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MaterialId(pub u16);

/// Lightweight animation identifier.
///
/// The caller maps from their domain-specific animation concept (e.g.
/// `EnemyPresentationState`) to this key. The collision system only stores
/// and looks up by key — it does not interpret the value.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AnimationKey(pub u16);

// ---------------------------------------------------------------------------
// 8-direction facing
// ---------------------------------------------------------------------------

/// 8-direction billboard facing for collision selection.
///
/// Uses the same relative-angle bucket order as the directional billboard
/// renderer. `Front` means the observer is in the direction the target faces
/// (i.e. the observer sees the target's front).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum BillboardFacing8 {
    Front = 0,
    FrontRight = 1,
    Right = 2,
    BackLeft = 3,
    Back = 4,
    BackRight = 5,
    Left = 6,
    FrontLeft = 7,
}

impl BillboardFacing8 {
    /// All eight facings in renderer quantization order starting from `Front`.
    pub const ALL: [Self; 8] = [
        Self::Front,
        Self::FrontRight,
        Self::Right,
        Self::BackLeft,
        Self::Back,
        Self::BackRight,
        Self::Left,
        Self::FrontLeft,
    ];

    /// Number of facing directions.
    pub const COUNT: usize = 8;

    /// Sector width in radians (τ/8 = π/4).
    const SECTOR: f32 = std::f32::consts::TAU / 8.0;

    /// Quantize a relative viewing angle into one of 8 facing buckets.
    ///
    /// `angle` is in radians from the target's forward direction, using the
    /// same convention as the directional billboard renderer: positive angles
    /// select right-side rendered facings. Values outside `[0, τ)` are wrapped.
    #[must_use]
    pub fn from_relative_angle(angle: f32) -> Self {
        let half = Self::SECTOR * 0.5;
        let tau = std::f32::consts::TAU;
        // Offset by half-sector so bucket centres align with enum values,
        // then wrap to [0, τ) and divide into sectors.
        let wrapped = (angle + half).rem_euclid(tau);
        let index = (wrapped / Self::SECTOR) as usize % Self::COUNT;
        Self::ALL[index]
    }

    /// Determine facing from world positions and target yaw.
    ///
    /// `target_pos` and `target_yaw` describe the target's 2D pose.
    /// `attacker_pos` is the observer/attacker position used to select
    /// the billboard facing.
    #[must_use]
    pub fn from_positions(target_pos: Vec2, target_yaw: f32, attacker_pos: Vec2) -> Self {
        let delta = attacker_pos - target_pos;
        if delta.length_squared() < f32::EPSILON * f32::EPSILON {
            return Self::Front;
        }
        let angle_to_attacker = delta.y.atan2(delta.x);
        Self::from_relative_angle(angle_to_attacker - target_yaw)
    }

    /// Convert to index `0..8`.
    #[must_use]
    pub const fn index(self) -> usize {
        self as usize
    }
}

// ---------------------------------------------------------------------------
// Query pose
// ---------------------------------------------------------------------------

/// 2D target pose for collision queries.
///
/// Contains the target's position and yaw — no visual pitch, no rendering
/// state. The facing for a specific attacker is computed on demand.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TargetQueryPose2d {
    pub position: Vec2,
    pub yaw: f32,
}

impl TargetQueryPose2d {
    #[must_use]
    pub const fn new(position: Vec2, yaw: f32) -> Self {
        Self { position, yaw }
    }

    /// Compute the billboard facing an attacker would see.
    #[must_use]
    pub fn facing_for_attacker(&self, attacker_pos: Vec2) -> BillboardFacing8 {
        BillboardFacing8::from_positions(self.position, self.yaw, attacker_pos)
    }
}

// ---------------------------------------------------------------------------
// Part collider (geometry)
// ---------------------------------------------------------------------------

/// A collision primitive tagged with a part identifier.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PartCollider2d {
    pub part_id: PartId,
    pub collider: Collider,
}

// ---------------------------------------------------------------------------
// Part metadata (gameplay, separate from geometry)
// ---------------------------------------------------------------------------

/// Gameplay metadata for a collision part.
///
/// Kept separate from [`PartCollider2d`] so the collision kernel does not
/// depend on damage routing rules.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PartMetadata {
    pub material: MaterialId,
    pub damage_scale: f32,
}

// ---------------------------------------------------------------------------
// Collision frame
// ---------------------------------------------------------------------------

/// Collision data for a single animation frame at a specific facing.
///
/// Stores part colliders in a layout optimised for [`nearest_ray_hit_tagged`]
/// and [`nearest_segment_hit_tagged`] queries.
///
/// [`nearest_ray_hit_tagged`]: super::nearest::nearest_ray_hit_tagged
/// [`nearest_segment_hit_tagged`]: super::nearest::nearest_segment_hit_tagged
#[derive(Clone, Debug, Default)]
pub struct TargetCollisionFrame {
    tagged: Vec<(Collider, PartId)>,
}

impl TargetCollisionFrame {
    /// Build a frame from an iterator of part colliders.
    pub fn new(parts: impl IntoIterator<Item = PartCollider2d>) -> Self {
        Self {
            tagged: parts.into_iter().map(|p| (p.collider, p.part_id)).collect(),
        }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tagged.is_empty()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.tagged.len()
    }

    /// Iterate over the part colliders in this frame.
    pub fn parts(&self) -> impl Iterator<Item = PartCollider2d> + '_ {
        self.tagged.iter().map(|(c, id)| PartCollider2d {
            part_id: *id,
            collider: *c,
        })
    }

    /// Nearest ray hit among this frame's parts.
    ///
    /// Returns a [`HitResult`] tagged with the [`PartId`] of the hit
    /// collider. Equal-distance ties are broken by smaller `PartId`.
    #[must_use]
    pub fn nearest_ray_hit(&self, origin: Vec2, direction: Vec2) -> Option<HitResult<PartId>> {
        nearest::nearest_ray_hit_tagged(origin, direction, &self.tagged)
    }

    /// Nearest segment hit among this frame's parts.
    #[must_use]
    pub fn nearest_segment_hit(&self, start: Vec2, end: Vec2) -> Option<HitResult<PartId>> {
        nearest::nearest_segment_hit_tagged(start, end, &self.tagged)
    }
}

// ---------------------------------------------------------------------------
// Collision set (per target type)
// ---------------------------------------------------------------------------

/// Lookup key for a specific collision frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CollisionFrameKey {
    pub animation: AnimationKey,
    pub frame: u16,
    pub facing: BillboardFacing8,
}

/// Complete collision data for one target type (e.g. one enemy variant).
///
/// Indexed by ([`AnimationKey`], frame index, [`BillboardFacing8`]).
/// Part metadata is stored separately and looked up by [`PartId`].
#[derive(Clone, Debug, Default)]
pub struct TargetCollisionSet {
    frames: HashMap<CollisionFrameKey, TargetCollisionFrame>,
    part_metadata: HashMap<PartId, PartMetadata>,
}

impl TargetCollisionSet {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert collision data for a specific frame/facing combination.
    pub fn insert_frame(&mut self, key: CollisionFrameKey, frame: TargetCollisionFrame) {
        self.frames.insert(key, frame);
    }

    /// Register gameplay metadata for a part.
    pub fn insert_part_metadata(&mut self, part_id: PartId, meta: PartMetadata) {
        self.part_metadata.insert(part_id, meta);
    }

    /// Look up a collision frame by composite key.
    #[must_use]
    pub fn frame(&self, key: &CollisionFrameKey) -> Option<&TargetCollisionFrame> {
        self.frames.get(key)
    }

    /// Look up a collision frame by individual components.
    #[must_use]
    pub fn lookup(
        &self,
        animation: AnimationKey,
        frame: u16,
        facing: BillboardFacing8,
    ) -> Option<&TargetCollisionFrame> {
        self.frame(&CollisionFrameKey {
            animation,
            frame,
            facing,
        })
    }

    /// Retrieve gameplay metadata for a part.
    #[must_use]
    pub fn part_metadata(&self, part_id: PartId) -> Option<&PartMetadata> {
        self.part_metadata.get(&part_id)
    }

    /// Number of registered frames.
    #[must_use]
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::collision::primitives::Circle;
    use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, PI, TAU};

    const EPS: f32 = 1e-4;

    fn approx(a: f32, b: f32) -> bool {
        (a - b).abs() < EPS
    }

    // --- BillboardFacing8 ---

    #[test]
    fn facing_sector_centres_are_deterministic() {
        // Matches directional_billboard::standard_8_directions:
        // 0=front, +45=frontright, +90=right, ..., +315=frontleft.
        let expected = [
            (0.0, BillboardFacing8::Front),
            (FRAC_PI_4, BillboardFacing8::FrontRight),
            (FRAC_PI_2, BillboardFacing8::Right),
            (FRAC_PI_2 + FRAC_PI_4, BillboardFacing8::BackLeft),
            (PI, BillboardFacing8::Back),
            (PI + FRAC_PI_4, BillboardFacing8::BackRight),
            (PI + FRAC_PI_2, BillboardFacing8::Left),
            (PI + FRAC_PI_2 + FRAC_PI_4, BillboardFacing8::FrontLeft),
        ];
        for (angle, want) in &expected {
            let got = BillboardFacing8::from_relative_angle(*angle);
            assert_eq!(
                got, *want,
                "angle {angle:.4} should be {want:?}, got {got:?}"
            );
        }
    }

    #[test]
    fn facing_sector_boundaries_match_renderer_quantization() {
        assert_eq!(
            BillboardFacing8::from_relative_angle(PI / 8.0 - 0.001),
            BillboardFacing8::Front,
            "just before +22.5 degrees stays Front"
        );
        assert_eq!(
            BillboardFacing8::from_relative_angle(PI / 8.0),
            BillboardFacing8::FrontRight,
            "exact +22.5 degree boundary rounds to FrontRight"
        );
        assert_eq!(
            BillboardFacing8::from_relative_angle(-PI / 8.0),
            BillboardFacing8::Front,
            "exact -22.5 degree boundary rounds to Front"
        );
        assert_eq!(
            BillboardFacing8::from_relative_angle(-PI / 8.0 - 0.001),
            BillboardFacing8::FrontLeft,
            "just past -22.5 degrees rounds to FrontLeft"
        );
    }

    #[test]
    fn facing_wraps_negative_angles() {
        assert_eq!(
            BillboardFacing8::from_relative_angle(-FRAC_PI_4),
            BillboardFacing8::FrontLeft
        );
        assert_eq!(
            BillboardFacing8::from_relative_angle(-FRAC_PI_2),
            BillboardFacing8::Left
        );
    }

    #[test]
    fn facing_wraps_large_angles() {
        assert_eq!(
            BillboardFacing8::from_relative_angle(TAU + FRAC_PI_4),
            BillboardFacing8::FrontRight
        );
    }

    #[test]
    fn facing_from_positions_front() {
        // Target at origin facing east. Attacker directly east → Front.
        let facing = BillboardFacing8::from_positions(Vec2::ZERO, 0.0, Vec2::new(5.0, 0.0));
        assert_eq!(facing, BillboardFacing8::Front);
    }

    #[test]
    fn facing_from_positions_back() {
        // Target at origin facing east. Attacker directly west → Back.
        let facing = BillboardFacing8::from_positions(Vec2::ZERO, 0.0, Vec2::new(-5.0, 0.0));
        assert_eq!(facing, BillboardFacing8::Back);
    }

    #[test]
    fn facing_from_positions_right_matches_renderer() {
        // Renderer convention: target faces east, attacker north → Right.
        let facing = BillboardFacing8::from_positions(Vec2::ZERO, 0.0, Vec2::new(0.0, 5.0));
        assert_eq!(facing, BillboardFacing8::Right);
    }

    #[test]
    fn facing_from_positions_left_matches_renderer() {
        // Renderer convention: target faces east, attacker south → Left.
        let facing = BillboardFacing8::from_positions(Vec2::ZERO, 0.0, Vec2::new(0.0, -5.0));
        assert_eq!(facing, BillboardFacing8::Left);
    }

    #[test]
    fn facing_from_positions_respects_non_zero_target_yaw() {
        // Target faces north. Attacker north is now directly in front.
        let facing = BillboardFacing8::from_positions(Vec2::ZERO, FRAC_PI_2, Vec2::new(0.0, 5.0));
        assert_eq!(facing, BillboardFacing8::Front);

        // Same target yaw, attacker west is the rendered right-side facing.
        let facing = BillboardFacing8::from_positions(Vec2::ZERO, FRAC_PI_2, Vec2::new(-5.0, 0.0));
        assert_eq!(facing, BillboardFacing8::Right);

        // Same target yaw, attacker east is the rendered left-side facing.
        let facing = BillboardFacing8::from_positions(Vec2::ZERO, FRAC_PI_2, Vec2::new(5.0, 0.0));
        assert_eq!(facing, BillboardFacing8::Left);
    }

    #[test]
    fn same_enemy_different_facings_for_two_attackers() {
        let pose = TargetQueryPose2d::new(Vec2::new(5.0, 5.0), 0.0);
        let attacker_a = Vec2::new(8.0, 5.0); // east → Front
        let attacker_b = Vec2::new(5.0, 8.0); // north → Right

        let facing_a = pose.facing_for_attacker(attacker_a);
        let facing_b = pose.facing_for_attacker(attacker_b);

        assert_eq!(facing_a, BillboardFacing8::Front);
        assert_eq!(facing_b, BillboardFacing8::Right);
        assert_ne!(facing_a, facing_b);
    }

    #[test]
    fn query_pose_uses_target_yaw_for_renderer_facing() {
        let pose = TargetQueryPose2d::new(Vec2::ZERO, FRAC_PI_2);
        assert_eq!(
            pose.facing_for_attacker(Vec2::new(0.0, 5.0)),
            BillboardFacing8::Front
        );
        assert_eq!(
            pose.facing_for_attacker(Vec2::new(-5.0, 0.0)),
            BillboardFacing8::Right
        );
    }

    #[test]
    fn coincident_positions_default_to_front() {
        let facing = BillboardFacing8::from_positions(Vec2::ZERO, 0.0, Vec2::ZERO);
        assert_eq!(facing, BillboardFacing8::Front);
    }

    #[test]
    fn facing_index_matches_enum_value() {
        for (i, f) in BillboardFacing8::ALL.iter().enumerate() {
            assert_eq!(f.index(), i);
        }
    }

    // --- TargetCollisionSet lookup ---

    fn test_set() -> TargetCollisionSet {
        let mut set = TargetCollisionSet::new();
        let anim = AnimationKey(0);
        let frame = TargetCollisionFrame::new([PartCollider2d {
            part_id: PartId(1),
            collider: Collider::Circle(Circle::new(Vec2::ZERO, 0.5)),
        }]);
        set.insert_frame(
            CollisionFrameKey {
                animation: anim,
                frame: 0,
                facing: BillboardFacing8::Front,
            },
            frame,
        );
        set.insert_part_metadata(
            PartId(1),
            PartMetadata {
                material: MaterialId(10),
                damage_scale: 2.0,
            },
        );
        set
    }

    #[test]
    fn lookup_valid_frame() {
        let set = test_set();
        let frame = set.lookup(AnimationKey(0), 0, BillboardFacing8::Front);
        assert!(frame.is_some());
        assert_eq!(frame.unwrap().len(), 1);
    }

    #[test]
    fn lookup_missing_facing_returns_none() {
        let set = test_set();
        assert!(
            set.lookup(AnimationKey(0), 0, BillboardFacing8::Back)
                .is_none()
        );
    }

    #[test]
    fn lookup_missing_animation_returns_none() {
        let set = test_set();
        assert!(
            set.lookup(AnimationKey(99), 0, BillboardFacing8::Front)
                .is_none()
        );
    }

    #[test]
    fn lookup_missing_frame_index_returns_none() {
        let set = test_set();
        assert!(
            set.lookup(AnimationKey(0), 5, BillboardFacing8::Front)
                .is_none()
        );
    }

    #[test]
    fn part_metadata_lookup() {
        let set = test_set();
        let meta = set.part_metadata(PartId(1)).unwrap();
        assert_eq!(meta.material, MaterialId(10));
        assert!(approx(meta.damage_scale, 2.0));
    }

    #[test]
    fn missing_part_metadata_returns_none() {
        let set = test_set();
        assert!(set.part_metadata(PartId(99)).is_none());
    }

    // --- TargetCollisionFrame queries ---

    #[test]
    fn frame_nearest_ray_hit_returns_correct_part() {
        let frame = TargetCollisionFrame::new([
            PartCollider2d {
                part_id: PartId(10),
                collider: Collider::Circle(Circle::new(Vec2::new(3.0, 0.0), 0.5)),
            },
            PartCollider2d {
                part_id: PartId(20),
                collider: Collider::Circle(Circle::new(Vec2::new(6.0, 0.0), 0.5)),
            },
        ]);

        let hit = frame.nearest_ray_hit(Vec2::ZERO, Vec2::X).unwrap();
        assert_eq!(hit.id, PartId(10), "nearer part should be hit");
        assert!(approx(hit.distance, 2.5));
    }

    #[test]
    fn frame_nearest_ray_hit_tie_broken_by_part_id() {
        // Two parts at identical positions → same distance → smaller PartId wins.
        let frame = TargetCollisionFrame::new([
            PartCollider2d {
                part_id: PartId(30),
                collider: Collider::Circle(Circle::new(Vec2::new(5.0, 0.0), 1.0)),
            },
            PartCollider2d {
                part_id: PartId(10),
                collider: Collider::Circle(Circle::new(Vec2::new(5.0, 0.0), 1.0)),
            },
        ]);

        let hit = frame.nearest_ray_hit(Vec2::ZERO, Vec2::X).unwrap();
        assert_eq!(hit.id, PartId(10), "smaller PartId wins tie");
    }

    #[test]
    fn frame_nearest_segment_hit_returns_correct_part() {
        let frame = TargetCollisionFrame::new([
            PartCollider2d {
                part_id: PartId(1),
                collider: Collider::Circle(Circle::new(Vec2::new(3.0, 0.0), 0.5)),
            },
            PartCollider2d {
                part_id: PartId(2),
                collider: Collider::Circle(Circle::new(Vec2::new(6.0, 0.0), 0.5)),
            },
        ]);

        let hit = frame
            .nearest_segment_hit(Vec2::ZERO, Vec2::new(10.0, 0.0))
            .unwrap();
        assert_eq!(hit.id, PartId(1));
    }

    #[test]
    fn frame_segment_miss_when_too_short() {
        let frame = TargetCollisionFrame::new([PartCollider2d {
            part_id: PartId(1),
            collider: Collider::Circle(Circle::new(Vec2::new(5.0, 0.0), 0.5)),
        }]);

        assert!(
            frame
                .nearest_segment_hit(Vec2::ZERO, Vec2::new(2.0, 0.0))
                .is_none()
        );
    }

    #[test]
    fn empty_frame_returns_no_hit() {
        let frame = TargetCollisionFrame::default();
        assert!(frame.nearest_ray_hit(Vec2::ZERO, Vec2::X).is_none());
        assert!(
            frame
                .nearest_segment_hit(Vec2::ZERO, Vec2::new(10.0, 0.0))
                .is_none()
        );
    }

    #[test]
    fn frame_parts_iterator_round_trips() {
        let parts = vec![
            PartCollider2d {
                part_id: PartId(1),
                collider: Collider::Circle(Circle::new(Vec2::ZERO, 1.0)),
            },
            PartCollider2d {
                part_id: PartId(2),
                collider: Collider::Circle(Circle::new(Vec2::X, 0.5)),
            },
        ];
        let frame = TargetCollisionFrame::new(parts.clone());
        let round_tripped: Vec<_> = frame.parts().collect();
        assert_eq!(round_tripped.len(), 2);
        assert_eq!(round_tripped[0].part_id, PartId(1));
        assert_eq!(round_tripped[1].part_id, PartId(2));
    }

    // --- No visual pitch ---

    #[test]
    fn target_query_pose_has_no_pitch_field() {
        // Structural: TargetQueryPose2d only has position + yaw.
        let pose = TargetQueryPose2d::new(Vec2::ZERO, 1.0);
        let _ = pose.position;
        let _ = pose.yaw;
        // No pitch field exists — compile-time guarantee.
    }
}
