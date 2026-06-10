//! Temporary hand-authored per-part collision fixtures for FPS enemies.
//!
//! Phase 3 hitscan integration consumes these fixtures through
//! [`crate::hitscan::hitscan_parts_from_pose`]. They are deliberately simple
//! and authored by hand — no sprite-mask or asset-pipeline generation yet.
//!
//! # Facing
//!
//! Enemy yaw **is** authoritative as of Phase 3.5: the server writes
//! `NetEnemy.angle` (oriented toward the engaged player) and single-player
//! derives the same basis via [`crate::facing_yaw_toward`], so hit-test code
//! receives a real per-target yaw — not `0.0`. That yaw rotates the local part
//! layout (e.g. the Spidey head orients toward the engaged player) and selects
//! the [`BillboardFacing8`] bucket.
//!
//! What is **not** yet authored is per-facing *geometry*: every fixture still
//! registers the **same frame for all 8 facings**, so the selected bucket maps
//! to one silhouette regardless of angle. Distinct per-facing frames can be
//! authored here without any call-site change — but only once enemy sprites
//! render directionally, so collision silhouettes match what the player sees.
//!
//! # Coordinate convention
//!
//! Part colliders are authored in target-local space: local `+X` is the target
//! forward direction, local `+Y` is 90° CCW. With `yaw = 0.0` this coincides
//! with world axes.

use std::sync::LazyLock;

use bevy_math::Vec2;

use crate::collision::{
    BillboardFacing8, Circle, Collider, CollisionFrameKey, MaterialId, PartCollider2d, PartId,
    PartMetadata, TargetCollisionFrame, TargetCollisionSet,
};
use crate::config::FpsCombatConfig;
use crate::enemy::FpsEnemyKind;

/// Default animation key used until per-animation collision frames exist.
pub const DEFAULT_ANIMATION: crate::collision::AnimationKey = crate::collision::AnimationKey(0);
/// Default frame index used until runtime frame selection exists.
pub const DEFAULT_FRAME: u16 = 0;

/// Whole-body fallback radius for `Basic` enemies. Matches the legacy
/// `Enemy::new` hitscan radius.
pub const BASIC_FALLBACK_RADIUS: f32 = 0.3;

/// Type-specific whole-body fallback radius, used when a target has no authored
/// collision frame for the requested animation/frame/facing.
///
/// This is the shared source of truth for both server hitscan and server flame
/// so they agree with each other and with single-player (which derives the same
/// values from per-instance configs). For `Mosquiton`/`Spidey` it tracks the
/// gameplay collision radius from [`FpsCombatConfig`] (e.g. Spidey is 0.25, not
/// the uniform 0.3 that earlier server code used).
#[must_use]
pub fn enemy_fallback_radius(kind: FpsEnemyKind, combat: &FpsCombatConfig) -> f32 {
    match kind {
        FpsEnemyKind::Basic => BASIC_FALLBACK_RADIUS,
        FpsEnemyKind::Mosquiton => combat.mosquiton_collision_radius,
        FpsEnemyKind::Spidey => combat.spidey.collision_radius,
    }
}

/// Whole-body part shared by every fixture.
pub const PART_BODY: PartId = PartId(1);
/// Head part (currently only the Spidey demo fixture).
pub const PART_HEAD: PartId = PartId(2);

const MATERIAL_FLESH: MaterialId = MaterialId(1);
const MATERIAL_HEAD: MaterialId = MaterialId(2);

/// Register `frame` for all 8 facings under the default animation/frame.
fn all_facings(set: &mut TargetCollisionSet, parts: &[PartCollider2d]) {
    for facing in BillboardFacing8::ALL {
        set.insert_frame(
            CollisionFrameKey {
                animation: DEFAULT_ANIMATION,
                frame: DEFAULT_FRAME,
                facing,
            },
            TargetCollisionFrame::new(parts.iter().copied()),
        );
    }
}

fn basic_set() -> TargetCollisionSet {
    // Single body circle matching the legacy `Enemy` hitscan radius (0.3).
    let mut set = TargetCollisionSet::new();
    all_facings(
        &mut set,
        &[PartCollider2d {
            part_id: PART_BODY,
            collider: Collider::Circle(Circle::new(Vec2::ZERO, 0.3)),
        }],
    );
    set.insert_part_metadata(
        PART_BODY,
        PartMetadata {
            material: MATERIAL_FLESH,
            damage_scale: 1.0,
            targetable: true,
            armour: 0.0,
        },
    );
    set
}

fn mosquiton_set() -> TargetCollisionSet {
    // Single body circle matching the default Mosquiton collision radius (0.3).
    let mut set = TargetCollisionSet::new();
    all_facings(
        &mut set,
        &[PartCollider2d {
            part_id: PART_BODY,
            collider: Collider::Circle(Circle::new(Vec2::ZERO, 0.3)),
        }],
    );
    set.insert_part_metadata(
        PART_BODY,
        PartMetadata {
            material: MATERIAL_FLESH,
            damage_scale: 1.0,
            targetable: true,
            armour: 0.0,
        },
    );
    set
}

fn spidey_set() -> TargetCollisionSet {
    // Multi-part demo: a body circle plus a smaller head that sticks out
    // forward (local +X). A shot from the front reaches the head first; a shot
    // from behind reaches the body first (head is occluded behind the body).
    let mut set = TargetCollisionSet::new();
    all_facings(
        &mut set,
        &[
            PartCollider2d {
                part_id: PART_BODY,
                collider: Collider::Circle(Circle::new(Vec2::ZERO, 0.28)),
            },
            PartCollider2d {
                part_id: PART_HEAD,
                collider: Collider::Circle(Circle::new(Vec2::new(0.22, 0.0), 0.12)),
            },
        ],
    );
    set.insert_part_metadata(
        PART_BODY,
        PartMetadata {
            material: MATERIAL_FLESH,
            damage_scale: 1.0,
            targetable: true,
            armour: 0.0,
        },
    );
    set.insert_part_metadata(
        PART_HEAD,
        PartMetadata {
            material: MATERIAL_HEAD,
            // Headshot multiplier: a forward shot that reaches the head deals
            // double damage. 2.0 is the classic headshot value; tune here.
            damage_scale: 2.0,
            targetable: true,
            armour: 0.0,
        },
    );
    set
}

static BASIC: LazyLock<TargetCollisionSet> = LazyLock::new(basic_set);
static MOSQUITON: LazyLock<TargetCollisionSet> = LazyLock::new(mosquiton_set);
static SPIDEY: LazyLock<TargetCollisionSet> = LazyLock::new(spidey_set);

/// Shared collision fixture for an enemy kind.
#[must_use]
pub fn collision_set(kind: FpsEnemyKind) -> &'static TargetCollisionSet {
    match kind {
        FpsEnemyKind::Basic => &BASIC,
        FpsEnemyKind::Mosquiton => &MOSQUITON,
        FpsEnemyKind::Spidey => &SPIDEY,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collision::TargetQueryPose2d;

    #[test]
    fn fallback_radius_is_type_specific() {
        let combat = FpsCombatConfig::default();
        // Spidey must use its own (smaller) collision radius, not the uniform
        // 0.3 the earlier server code applied to every kind.
        let spidey = enemy_fallback_radius(FpsEnemyKind::Spidey, &combat);
        assert!(
            (spidey - 0.25).abs() < 1e-6,
            "spidey fallback should be 0.25, got {spidey}"
        );
        assert!(
            (spidey - BASIC_FALLBACK_RADIUS).abs() > 1e-6,
            "spidey fallback must differ from the basic 0.3"
        );
        assert!((enemy_fallback_radius(FpsEnemyKind::Basic, &combat) - 0.3).abs() < 1e-6);
        assert!(
            (enemy_fallback_radius(FpsEnemyKind::Mosquiton, &combat)
                - combat.mosquiton_collision_radius)
                .abs()
                < 1e-6
        );
    }

    #[test]
    fn every_kind_has_all_facings() {
        for kind in [
            FpsEnemyKind::Basic,
            FpsEnemyKind::Mosquiton,
            FpsEnemyKind::Spidey,
        ] {
            let set = collision_set(kind);
            for facing in BillboardFacing8::ALL {
                assert!(
                    set.lookup(DEFAULT_ANIMATION, DEFAULT_FRAME, facing)
                        .is_some(),
                    "{kind:?} missing facing {facing:?}"
                );
            }
        }
    }

    #[test]
    fn basic_matches_legacy_radius() {
        // Body circle radius 0.3, centred — a +X ray from 2 units away on the
        // axis hits the near surface at distance 1.7.
        let set = collision_set(FpsEnemyKind::Basic);
        let frame = set
            .lookup(DEFAULT_ANIMATION, DEFAULT_FRAME, BillboardFacing8::Front)
            .unwrap();
        let pose = TargetQueryPose2d::new(Vec2::new(2.0, 0.0), 0.0);
        let hit = frame
            .nearest_world_ray_hit(pose, Vec2::ZERO, Vec2::X)
            .unwrap();
        assert_eq!(hit.id, PART_BODY);
        assert!((hit.distance - 1.7).abs() < 1e-4);
    }

    #[test]
    fn spidey_front_shot_hits_head() {
        let set = collision_set(FpsEnemyKind::Spidey);
        let frame = set
            .lookup(DEFAULT_ANIMATION, DEFAULT_FRAME, BillboardFacing8::Front)
            .unwrap();
        // Enemy faces +X; shot travels -X from the front. Head (local +X) first.
        let pose = TargetQueryPose2d::new(Vec2::new(0.0, 0.0), 0.0);
        let hit = frame
            .nearest_world_ray_hit(pose, Vec2::new(5.0, 0.0), Vec2::NEG_X)
            .unwrap();
        assert_eq!(hit.id, PART_HEAD);
    }

    #[test]
    fn spidey_rear_shot_hits_body() {
        let set = collision_set(FpsEnemyKind::Spidey);
        let frame = set
            .lookup(DEFAULT_ANIMATION, DEFAULT_FRAME, BillboardFacing8::Front)
            .unwrap();
        // Shot travels +X from behind; body occludes the head.
        let pose = TargetQueryPose2d::new(Vec2::new(0.0, 0.0), 0.0);
        let hit = frame
            .nearest_world_ray_hit(pose, Vec2::new(-5.0, 0.0), Vec2::X)
            .unwrap();
        assert_eq!(hit.id, PART_BODY);
    }
}
