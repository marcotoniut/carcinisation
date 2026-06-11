//! Per-part damage routing: live server e2e + SP/server parity + armour + the
//! SP/server rounding contract.
//!
//! These tests close the highest-value gaps from the pre-Phase-12 review:
//!   * the server actually *deals* per-part scaled damage (headshot 2×), not a
//!     whole-body fallback (`server_spidey_headshot_*`);
//!   * the live server result equals the shared-kernel result that single-
//!     player routes through, under both authorities' target construction
//!     (`sp_server_parity_*`) — the previous parity test only called
//!     `routed_damage` twice;
//!   * armour flows through the exact routing boundary the server calls
//!     (`armour_reduces_damage_*`);
//!   * shipped part data resolves to integer damage so SP (`u32`, rounds) and
//!     server (`f32`) stay bit-identical (`rounding_contract_*`).
//!
//! Deterministic: `build_deterministic_server_*` runs exactly one FixedUpdate
//! per `update()` at 30 Hz. The Spidey is spawned sim-less and static (see
//! `spawn_static_spidey`) so head/body geometry is exact.
#![allow(clippy::doc_markdown, clippy::float_cmp)]

mod common;

use std::f32::consts::PI;

use bevy::prelude::Vec2;
use carcinisation_fps_core::collision::{
    BillboardFacing8, Circle, Collider, CollisionFrameKey, MaterialId, PartCollider2d, PartId,
    PartMetadata, TargetCollisionFrame, TargetCollisionSet,
};
use carcinisation_fps_core::enemy_collision::{
    DEFAULT_ANIMATION, DEFAULT_FRAME, enemy_fallback_radius,
};
use carcinisation_fps_core::hitscan::{PartHitscanTarget, hitscan_parts_from_pose, routed_damage};
use carcinisation_fps_core::map::test_map;
use carcinisation_fps_core::{
    FirePose2d, FpsCombatConfig, FpsEnemyKind, collision_set, facing_yaw_toward,
};

use common::{
    build_deterministic_server_with_enemies, get_enemy_health, inject_fire, spawn_alive_player,
    spawn_static_spidey,
};

// Shared geometry: player at (1.5,1.5) facing east (+X); enemy at (3.5,1.5) on
// the same row. The Spidey head collider sits forward (local +X). Facing the
// player (angle = PI, head points west toward the shooter) → head is reached
// first (2.0×). Facing away (angle = 0, head points east, body occludes) →
// body (1.0×). Mirrors the SP unit test `pistol_headshot_scales_spidey_damage`.
const PLAYER_POS: Vec2 = Vec2::new(1.5, 1.5);
const ENEMY_POS: Vec2 = Vec2::new(3.5, 1.5);
const ENEMY_HP: f32 = 1000.0;

/// Fire one pistol shot at the static enemy and return the health delta dealt.
fn fire_one_shot_delta(angle_facing: f32) -> f32 {
    let mut server = build_deterministic_server_with_enemies(test_map(), vec![]);
    server.update();
    spawn_static_spidey(&mut server, 1, ENEMY_POS, angle_facing, ENEMY_HP);
    spawn_alive_player(&mut server, 1, PLAYER_POS.x, PLAYER_POS.y);

    let before = get_enemy_health(&mut server).expect("static spidey present");
    inject_fire(&mut server, 1);
    // 3 ticks (~0.1 s) < pistol cooldown (~0.33 s) → exactly one shot lands.
    for _ in 0..3 {
        server.update();
    }
    let after = get_enemy_health(&mut server).expect("static spidey present");
    before - after
}

// ---------------------------------------------------------------------------
// Task 4: server-side Spidey headshot e2e
// ---------------------------------------------------------------------------

#[test]
fn server_spidey_headshot_scales_damage_through_live_combat() {
    let base = FpsCombatConfig::default().hitscan_damage; // 37.0

    let body = fire_one_shot_delta(0.0); // facing away → body
    let head = fire_one_shot_delta(PI); // facing player → head

    assert_eq!(body, base, "body shot deals neutral (1.0×) damage");
    assert_eq!(
        head,
        base * 2.0,
        "head shot deals the authored 2.0× multiplier"
    );
    assert_eq!(head, body * 2.0, "head is exactly double the body");
    assert!(
        head != body,
        "per-part routing is live: head and body differ"
    );
    // A whole-body fallback would route at the neutral 1.0× for BOTH facings.
    // head == 2×body proves the authored HEAD part was selected, not fallback.
    assert!(
        head > body,
        "head > body ⇒ authored per-part collision, not whole-body fallback"
    );
}

// ---------------------------------------------------------------------------
// Task 3: true SP/server e2e parity (supersedes the `routed_damage`-twice test)
// ---------------------------------------------------------------------------

/// Build the per-part hit result the way each authority constructs its query,
/// then route it. Returns `(part_id, damage_scale, armour, dealt_f32)`.
///
/// `yaw_source` + `fallback_radius` are the two places SP and server
/// constructions differ (SP re-derives yaw via `facing_yaw_toward` and uses the
/// instance collision radius; the server uses the replicated `NetEnemy.angle`
/// and `enemy_fallback_radius`). For a head hit the fallback radius is unused,
/// so both must agree.
fn route_via_kernel(yaw: f32, fallback_radius: f32, base: f32) -> (PartId, f32, f32, f32) {
    let map = test_map();
    let set = collision_set(FpsEnemyKind::Spidey);
    let target = PartHitscanTarget {
        position: ENEMY_POS,
        yaw,
        alive: true,
        set,
        animation: DEFAULT_ANIMATION,
        frame: DEFAULT_FRAME,
        fallback_radius,
    };
    let r = hitscan_parts_from_pose(
        FirePose2d::new(PLAYER_POS, 0.0, 0.0),
        &map,
        [target].into_iter(),
    )
    .expect("centre shot hits the spidey");
    (
        r.part_id,
        r.damage_scale,
        r.armour,
        routed_damage(base, r.damage_scale, r.armour),
    )
}

#[test]
fn sp_server_parity_spidey_headshot() {
    let combat = FpsCombatConfig::default();
    let base = combat.hitscan_damage;

    // SP-style construction: yaw re-derived toward the player, instance radius.
    let sp_yaw = facing_yaw_toward(ENEMY_POS, PLAYER_POS).unwrap_or(0.0);
    let (sp_part, sp_scale, sp_armour, sp_dealt_f32) = route_via_kernel(sp_yaw, 0.25, base);
    // SP rounds at its integer-health boundary.
    let sp_dealt: u32 = sp_dealt_f32.round() as u32;

    // Server-style construction: replicated angle (= PI here), config radius.
    let srv_yaw = PI;
    let srv_radius = enemy_fallback_radius(FpsEnemyKind::Spidey, &combat);
    let (srv_part, srv_scale, srv_armour, srv_dealt_f32) =
        route_via_kernel(srv_yaw, srv_radius, base);

    // Both constructions select the same part with the same routing.
    assert_eq!(sp_part, srv_part, "same part id under both constructions");
    assert_eq!(sp_scale, srv_scale, "same damage_scale");
    assert_eq!(sp_armour, srv_armour, "same armour");
    assert_eq!(sp_dealt_f32, srv_dealt_f32, "same routed damage (f32)");

    // It is the authored HEAD (2.0×), not the whole-body fallback.
    assert_ne!(sp_part, PartId::FALLBACK, "authored part, not fallback");
    assert_eq!(sp_scale, 2.0, "front shot selects the 2.0× head");
    assert_eq!(sp_armour, 0.0, "shipped head has no armour");

    // The LIVE server, driven end-to-end, deals exactly the kernel-routed value
    // (and SP's rounded value, since the data is integer-resolving).
    let server_live = fire_one_shot_delta(PI);
    assert_eq!(
        server_live, srv_dealt_f32,
        "live server == shared kernel route"
    );
    assert_eq!(
        server_live, sp_dealt as f32,
        "live server == SP routed (u32)"
    );
    assert_eq!(server_live, base * 2.0);
}

// ---------------------------------------------------------------------------
// Task 5: armour reduces damage through the routing boundary the server calls
// ---------------------------------------------------------------------------

const ARMOUR_PART: PartId = PartId(10);

/// Single targetable body collider carrying flat armour. No shipped enemy sets
/// non-zero armour yet (and `collision_set` is hard-authored per kind, so a
/// custom set cannot be injected into the live server), so armour is exercised
/// at the exact routing boundary the server calls: `hitscan_parts_from_pose`
/// → `routed_damage`.
fn armoured_test_set(armour: f32) -> TargetCollisionSet {
    let mut set = TargetCollisionSet::new();
    for facing in BillboardFacing8::ALL {
        set.insert_frame(
            CollisionFrameKey {
                animation: DEFAULT_ANIMATION,
                frame: DEFAULT_FRAME,
                facing,
            },
            TargetCollisionFrame::new([PartCollider2d {
                part_id: ARMOUR_PART,
                collider: Collider::Circle(Circle::new(Vec2::ZERO, 0.3)),
            }]),
        );
    }
    set.insert_part_metadata(
        ARMOUR_PART,
        PartMetadata {
            material: MaterialId(1),
            damage_scale: 1.0,
            targetable: true,
            armour,
        },
    );
    set
}

#[test]
fn armour_reduces_damage_through_routing_boundary() {
    let map = test_map();
    let base = FpsCombatConfig::default().hitscan_damage; // 37.0
    let set = armoured_test_set(10.0);

    let r = hitscan_parts_from_pose(
        FirePose2d::new(PLAYER_POS, 0.0, 0.0),
        &map,
        [PartHitscanTarget {
            position: ENEMY_POS,
            yaw: 0.0,
            alive: true,
            set: &set,
            animation: DEFAULT_ANIMATION,
            frame: DEFAULT_FRAME,
            fallback_radius: 0.3,
        }]
        .into_iter(),
    )
    .expect("centre shot hits the armoured body");

    assert_eq!(r.part_id, ARMOUR_PART);
    assert_eq!(r.armour, 10.0, "armour surfaced on the hit result");

    // Exactly the call the server makes: routed_damage(base, scale, armour).
    let dealt = routed_damage(base, r.damage_scale, r.armour);
    assert_eq!(
        dealt,
        base - 10.0,
        "flat armour subtracted after scaling (37 → 27)"
    );

    // Armour can never heal: huge armour clamps to 0, never negative.
    let over = armoured_test_set(1_000.0);
    let r2 = hitscan_parts_from_pose(
        FirePose2d::new(PLAYER_POS, 0.0, 0.0),
        &map,
        [PartHitscanTarget {
            position: ENEMY_POS,
            yaw: 0.0,
            alive: true,
            set: &over,
            animation: DEFAULT_ANIMATION,
            frame: DEFAULT_FRAME,
            fallback_radius: 0.3,
        }]
        .into_iter(),
    )
    .unwrap();
    assert_eq!(
        routed_damage(base, r2.damage_scale, r2.armour),
        0.0,
        "clamped ≥ 0"
    );
}

// ---------------------------------------------------------------------------
// Task 6: SP/server rounding contract
// ---------------------------------------------------------------------------
//
// CONTRACT: single-player health is `u32` and rounds the routed `f32` at its
// boundary (`routed_damage(...).round() as u32`); the server keeps `f32`
// (`NetHealth`). These two agree bit-for-bit **iff** the routed damage is
// integer-valued. We therefore enforce that all shipped part data resolves to
// integer damage under the default weapon. Any future Phase 12 modifier that
// introduces a fractional multiplier MUST either keep results integral or pick
// an explicit rounding alignment — the tests below will fail until it does.

#[test]
fn rounding_contract_shipped_part_data_is_integer_resolving() {
    let combat = FpsCombatConfig::default();
    let base = combat.hitscan_damage;
    assert_eq!(
        base.fract(),
        0.0,
        "shipped base hitscan_damage must be integral"
    );

    for kind in [
        FpsEnemyKind::Basic,
        FpsEnemyKind::Mosquiton,
        FpsEnemyKind::Spidey,
    ] {
        let set = collision_set(kind);
        let frame = set
            .frame(&CollisionFrameKey {
                animation: DEFAULT_ANIMATION,
                frame: DEFAULT_FRAME,
                facing: BillboardFacing8::Front,
            })
            .expect("every shipped kind authors the default frame");

        for part in frame.parts() {
            let (scale, armour) = set
                .part_metadata(part.part_id)
                .map_or((1.0, 0.0), |m| (m.damage_scale, m.armour));
            let dealt = routed_damage(base, scale, armour);
            assert_eq!(
                dealt.fract(),
                0.0,
                "kind {kind:?} part {:?} (scale {scale}, armour {armour}) → {dealt} is not \
                 integer: SP (round) and server (f32) would diverge",
                part.part_id,
            );
            // SP path: round-to-u32 is a no-op on integral damage.
            assert_eq!(
                dealt,
                (dealt.round() as u32) as f32,
                "SP == server for shipped data"
            );
        }
    }
}

#[test]
fn rounding_contract_fractional_scale_documents_divergence() {
    // PINS the boundary: a hypothetical fractional multiplier breaks SP/server
    // parity. This is the case the contract above forbids in shipped data.
    let base = 37.0;
    let server = routed_damage(base, 1.5, 0.0); // server applies 55.5 to f32 health
    let sp = routed_damage(base, 1.5, 0.0).round(); // SP rounds 55.5 → 56 at u32 boundary
    assert_eq!(server, 55.5);
    assert_eq!(sp, 56.0);
    assert_ne!(
        server, sp,
        "fractional scale diverges — must not ship without alignment"
    );
}
