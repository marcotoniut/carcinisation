use super::*;
use crate::collision::{
    BillboardFacing8, Circle, Collider, CollisionFrameKey, MaterialId, PartCollider2d, PartId,
    PartMetadata, TargetCollisionFrame, TargetCollisionSet,
};
use crate::map::test_map;

const ANIM: AnimationKey = AnimationKey(0);
const BODY: PartId = PartId(1);
const HEAD: PartId = PartId(2);
const LEG: PartId = PartId(3);

const EPS: f32 = 1e-4;

fn approx(a: f32, b: f32) -> bool {
    (a - b).abs() < EPS
}

/// Pose firing along +X from `origin`. Visual pitch defaults to zero.
fn pose_east(origin: Vec2) -> FirePose2d {
    FirePose2d::new(origin, 0.0, 0.0)
}

/// Single body-circle set registered for every facing under ANIM/frame 0.
fn single_body_set(radius: f32) -> TargetCollisionSet {
    let mut set = TargetCollisionSet::new();
    for facing in BillboardFacing8::ALL {
        set.insert_frame(
            CollisionFrameKey {
                animation: ANIM,
                frame: 0,
                facing,
            },
            TargetCollisionFrame::new([PartCollider2d {
                part_id: BODY,
                collider: Collider::Circle(Circle::new(Vec2::ZERO, radius)),
            }]),
        );
    }
    set.insert_part_metadata(
        BODY,
        PartMetadata {
            material: MaterialId(7),
            damage_scale: 1.0,
        },
    );
    set
}

/// Multi-facing demo: Front-ish facings have a HEAD in front, Right/Left have a
/// LEG to the side, Back facings have body only. Target-local +X = forward.
fn multi_facing_set() -> TargetCollisionSet {
    let body = PartCollider2d {
        part_id: BODY,
        collider: Collider::Circle(Circle::new(Vec2::ZERO, 0.25)),
    };
    let head = PartCollider2d {
        part_id: HEAD,
        collider: Collider::Circle(Circle::new(Vec2::new(0.3, 0.0), 0.15)),
    };
    let leg = PartCollider2d {
        part_id: LEG,
        collider: Collider::Circle(Circle::new(Vec2::new(0.0, 0.3), 0.15)),
    };

    let mut set = TargetCollisionSet::new();
    for facing in BillboardFacing8::ALL {
        let parts: Vec<PartCollider2d> = match facing {
            BillboardFacing8::Front
            | BillboardFacing8::FrontLeft
            | BillboardFacing8::FrontRight => {
                vec![body, head]
            }
            BillboardFacing8::Left | BillboardFacing8::Right => vec![body, leg],
            BillboardFacing8::Back | BillboardFacing8::BackLeft | BillboardFacing8::BackRight => {
                vec![body]
            }
        };
        set.insert_frame(
            CollisionFrameKey {
                animation: ANIM,
                frame: 0,
                facing,
            },
            TargetCollisionFrame::new(parts),
        );
    }
    set
}

fn target<'a>(
    set: &'a TargetCollisionSet,
    position: Vec2,
    fallback_radius: f32,
) -> PartHitscanTarget<'a> {
    PartHitscanTarget {
        position,
        yaw: 0.0,
        alive: true,
        set,
        animation: ANIM,
        frame: 0,
        fallback_radius,
    }
}

// --- nearest surface, not centre ---

#[test]
fn nearest_surface_wins_not_nearest_centre() {
    // A: centre far (x=5) but large radius → near surface at 3.0.
    // B: centre near (x=4) but small radius → near surface at 3.8.
    // Legacy centre-projection ordering would pick B (centre 4 < 5);
    // surface ordering picks A (surface 3.0 < 3.8).
    let map = test_map();
    let set_a = single_body_set(2.0);
    let set_b = single_body_set(0.2);
    let targets = [
        target(&set_a, Vec2::new(5.0, 1.5), 2.0),
        target(&set_b, Vec2::new(4.0, 1.5), 0.2),
    ];
    let hit =
        hitscan_parts_from_pose(pose_east(Vec2::new(1.5, 1.5)), &map, targets.into_iter()).unwrap();
    assert_eq!(hit.target_idx, 0, "large far circle has nearer surface");
    assert!(approx(hit.distance, 1.5), "surface at x=3.0 from x=1.5");
}

// --- wall obstruction ---

#[test]
fn enemy_before_wall_is_hit() {
    // Interior wall at cell (3,2). Fire +X along y=2.5 from x=1.5.
    let map = test_map();
    let set = single_body_set(0.3);
    let targets = [target(&set, Vec2::new(2.5, 2.5), 0.3)];
    let hit = hitscan_parts_from_pose(pose_east(Vec2::new(1.5, 2.5)), &map, targets.into_iter());
    assert!(hit.is_some(), "enemy before wall should be hit");
    assert_eq!(hit.unwrap().part_id, BODY);
}

#[test]
fn enemy_behind_wall_is_blocked() {
    let map = test_map();
    let set = single_body_set(0.3);
    let targets = [target(&set, Vec2::new(5.5, 2.5), 0.3)];
    let hit = hitscan_parts_from_pose(pose_east(Vec2::new(1.5, 2.5)), &map, targets.into_iter());
    assert!(hit.is_none(), "enemy behind wall (3,2) must be blocked");
}

// --- multiple enemies ---

#[test]
fn two_enemies_nearer_surface_wins() {
    let map = test_map();
    let set = single_body_set(0.3);
    let targets = [
        target(&set, Vec2::new(4.5, 1.5), 0.3),
        target(&set, Vec2::new(2.5, 1.5), 0.3),
    ];
    let hit =
        hitscan_parts_from_pose(pose_east(Vec2::new(1.5, 1.5)), &map, targets.into_iter()).unwrap();
    assert_eq!(hit.target_idx, 1, "nearer enemy wins");
}

// --- per-part ids ---

#[test]
fn head_and_body_part_ids_returned() {
    let map = test_map();
    let set = multi_facing_set();
    // Attacker east of enemy, enemy faces east (yaw 0) → Front facing.
    // Head sticks out forward (+X) so it is hit before body.
    let targets = [target(&set, Vec2::new(3.5, 1.5), 0.3)];
    let hit =
        hitscan_parts_from_pose(pose_west(Vec2::new(5.5, 1.5)), &map, targets.into_iter()).unwrap();
    assert_eq!(hit.part_id, HEAD, "front shot hits head first");
    assert_eq!(hit.material, None, "demo set has no head metadata");
}

/// Pose firing along -X from `origin`.
fn pose_west(origin: Vec2) -> FirePose2d {
    FirePose2d::new(origin, std::f32::consts::PI, 0.0)
}

// --- facing selection ---

#[test]
fn attacker_front_rear_side_select_matching_facing_frame() {
    let map = test_map();
    let set = multi_facing_set();
    let enemy = Vec2::new(2.5, 1.5);

    // Front: attacker east, fire -X. Front frame has head.
    let front = hitscan_parts_from_pose(
        pose_west(Vec2::new(5.5, 1.5)),
        &map,
        [target(&set, enemy, 0.3)].into_iter(),
    )
    .unwrap();
    assert_eq!(front.part_id, HEAD, "front facing exposes head");

    // Rear: attacker west, fire +X. Back frame is body only.
    let rear = hitscan_parts_from_pose(
        pose_east(Vec2::new(1.5, 1.5)),
        &map,
        [target(&set, enemy, 0.3)].into_iter(),
    )
    .unwrap();
    assert_eq!(rear.part_id, BODY, "back facing is body only");

    // Side: attacker north, fire -Y. Right frame has a leg.
    let pose_south = FirePose2d::new(Vec2::new(2.5, 3.5), -std::f32::consts::FRAC_PI_2, 0.0);
    let side =
        hitscan_parts_from_pose(pose_south, &map, [target(&set, enemy, 0.3)].into_iter()).unwrap();
    assert_eq!(side.part_id, LEG, "side facing exposes leg");
}

#[test]
fn target_yaw_changes_selected_part() {
    // Same attacker and enemy position; only the target yaw changes.
    // Enemy facing the attacker (head toward attacker) → front shot hits head.
    // Enemy facing away → Back frame has no head, and the head collider is now
    // behind the body anyway → body is hit.
    let map = test_map();
    let set = multi_facing_set();
    let enemy = Vec2::new(2.5, 1.5);
    let attacker = Vec2::new(5.5, 1.5); // east of enemy, fires -X

    // Yaw toward attacker (east, 0 rad): Front facing, head exposed forward.
    let facing_player = PartHitscanTarget {
        position: enemy,
        yaw: 0.0,
        alive: true,
        set: &set,
        animation: ANIM,
        frame: 0,
        fallback_radius: 0.3,
    };
    let hit_front =
        hitscan_parts_from_pose(pose_west(attacker), &map, [facing_player].into_iter()).unwrap();
    assert_eq!(hit_front.part_id, HEAD, "enemy facing attacker → head");

    // Yaw facing away (west, π rad): Back facing → body only.
    let facing_away = PartHitscanTarget {
        position: enemy,
        yaw: std::f32::consts::PI,
        alive: true,
        set: &set,
        animation: ANIM,
        frame: 0,
        fallback_radius: 0.3,
    };
    let hit_back =
        hitscan_parts_from_pose(pose_west(attacker), &map, [facing_away].into_iter()).unwrap();
    assert_eq!(hit_back.part_id, BODY, "enemy facing away → body");

    assert_ne!(
        hit_front.part_id, hit_back.part_id,
        "yaw changed the hit part"
    );
}

#[test]
fn two_attackers_resolve_different_facings_same_target() {
    let map = test_map();
    let set = multi_facing_set();
    let enemy = Vec2::new(2.5, 1.5);

    let from_front = hitscan_parts_from_pose(
        pose_west(Vec2::new(5.5, 1.5)),
        &map,
        [target(&set, enemy, 0.3)].into_iter(),
    )
    .unwrap();
    let from_side = hitscan_parts_from_pose(
        FirePose2d::new(Vec2::new(2.5, 3.5), -std::f32::consts::FRAC_PI_2, 0.0),
        &map,
        [target(&set, enemy, 0.3)].into_iter(),
    )
    .unwrap();

    assert_eq!(from_front.part_id, HEAD);
    assert_eq!(from_side.part_id, LEG);
    assert_ne!(from_front.part_id, from_side.part_id);
}

// --- fallback ---

#[test]
fn missing_frame_falls_back_to_circle() {
    let map = test_map();
    let set = single_body_set(0.3);
    // Query an animation key that has no registered frame.
    let mut t = target(&set, Vec2::new(2.5, 1.5), 0.3);
    t.animation = AnimationKey(99);
    let hit =
        hitscan_parts_from_pose(pose_east(Vec2::new(1.5, 1.5)), &map, [t].into_iter()).unwrap();
    assert_eq!(hit.part_id, PartId::FALLBACK, "no frame → fallback circle");
    assert_eq!(hit.material, None);
    assert!(
        approx(hit.distance, 0.7),
        "circle surface at x=2.2 from x=1.5"
    );
}

#[test]
fn fallback_zero_radius_does_not_hit() {
    let map = test_map();
    let set = single_body_set(0.3);
    let mut t = target(&set, Vec2::new(2.5, 1.5), 0.0);
    t.animation = AnimationKey(99);
    assert!(
        hitscan_parts_from_pose(pose_east(Vec2::new(1.5, 1.5)), &map, [t].into_iter()).is_none()
    );
}

// --- visual pitch is ignored ---

#[test]
fn visual_pitch_does_not_affect_selection() {
    let map = test_map();
    let set = multi_facing_set();
    let enemy = Vec2::new(3.5, 1.5);

    let flat = hitscan_parts_from_pose(
        FirePose2d::new(Vec2::new(5.5, 1.5), std::f32::consts::PI, 0.0),
        &map,
        [target(&set, enemy, 0.3)].into_iter(),
    )
    .unwrap();
    let pitched = hitscan_parts_from_pose(
        FirePose2d::new(Vec2::new(5.5, 1.5), std::f32::consts::PI, 9999.0),
        &map,
        [target(&set, enemy, 0.3)].into_iter(),
    )
    .unwrap();

    assert_eq!(flat, pitched, "visual_pitch_px must not change hit result");
}

// --- dead targets skipped ---

#[test]
fn dead_target_is_skipped() {
    let map = test_map();
    let set = single_body_set(0.3);
    let mut t = target(&set, Vec2::new(2.5, 1.5), 0.3);
    t.alive = false;
    assert!(
        hitscan_parts_from_pose(pose_east(Vec2::new(1.5, 1.5)), &map, [t].into_iter()).is_none()
    );
}

// --- empty frame handling ---

/// Set whose frame exists for every facing but contains zero parts.
fn empty_frame_set() -> TargetCollisionSet {
    let mut set = TargetCollisionSet::new();
    for facing in BillboardFacing8::ALL {
        set.insert_frame(
            CollisionFrameKey {
                animation: ANIM,
                frame: 0,
                facing,
            },
            TargetCollisionFrame::default(),
        );
    }
    set
}

#[test]
fn empty_frame_falls_back_to_circle() {
    // A registered-but-empty frame must not make the target invulnerable: it
    // is treated as unauthored and falls back to the whole-body circle.
    let map = test_map();
    let set = empty_frame_set();
    let hit = hitscan_parts_from_pose(
        pose_east(Vec2::new(1.5, 1.5)),
        &map,
        [target(&set, Vec2::new(2.5, 1.5), 0.3)].into_iter(),
    )
    .unwrap();
    assert_eq!(
        hit.part_id,
        PartId::FALLBACK,
        "empty frame → fallback circle"
    );
    assert!(
        approx(hit.distance, 0.7),
        "circle surface at x=2.2 from x=1.5"
    );
}

#[test]
fn non_empty_frame_miss_is_genuine_not_fallback() {
    // A non-empty frame whose parts the ray misses is a real miss — the large
    // fallback radius must NOT rescue it (only missing/empty frames fall back).
    let map = test_map();
    let set = single_body_set(0.1); // tiny body circle
    let mut t = target(&set, Vec2::new(2.5, 1.9), 5.0); // big fallback radius
    t.yaw = 0.0;
    // Fire along y=1.5; perpendicular offset to the body centre (y=1.9) is 0.4,
    // well outside the 0.1 body but inside the 5.0 fallback.
    let hit = hitscan_parts_from_pose(pose_east(Vec2::new(1.5, 1.5)), &map, [t].into_iter());
    assert!(
        hit.is_none(),
        "present non-empty frame that misses must not fall back"
    );
}

// --- deterministic tie-break across targets ---

#[test]
fn equal_distance_targets_break_by_lower_index() {
    // Two identical targets at the same position → identical distance. The
    // lower target index wins deterministically (first-seen, strict-less).
    let map = test_map();
    let set = single_body_set(0.3);
    let targets = [
        target(&set, Vec2::new(2.5, 1.5), 0.3),
        target(&set, Vec2::new(2.5, 1.5), 0.3),
    ];
    let hit =
        hitscan_parts_from_pose(pose_east(Vec2::new(1.5, 1.5)), &map, targets.into_iter()).unwrap();
    assert_eq!(hit.target_idx, 0, "equal distance → lower index wins");
}

// ---------------------------------------------------------------------------
// Flamethrower per-part overlap
// ---------------------------------------------------------------------------

/// Body at centre + a "wing" part offset laterally (local +Y). Lets a narrow
/// flame strip intersect the wing while the body centre is outside the strip.
fn flame_lateral_set() -> TargetCollisionSet {
    let mut set = TargetCollisionSet::new();
    for facing in BillboardFacing8::ALL {
        set.insert_frame(
            CollisionFrameKey {
                animation: ANIM,
                frame: 0,
                facing,
            },
            TargetCollisionFrame::new([
                PartCollider2d {
                    part_id: BODY,
                    collider: Collider::Circle(Circle::new(Vec2::ZERO, 0.2)),
                },
                PartCollider2d {
                    part_id: LEG,
                    collider: Collider::Circle(Circle::new(Vec2::new(0.0, 0.6), 0.2)),
                },
            ]),
        );
    }
    set
}

#[test]
fn flame_damages_part_not_centre_only() {
    // Enemy body centre is 0.5 off the flame axis (outside strip+body reach),
    // but the laterally-offset wing sits on the axis and is hit.
    let map = test_map();
    let set = flame_lateral_set();
    let target = PartHitscanTarget {
        position: Vec2::new(3.0, 1.0), // body centre at y=1.0; wing at y=1.6
        yaw: 0.0,
        alive: true,
        set: &set,
        animation: ANIM,
        frame: 0,
        fallback_radius: 0.3,
    };
    // Flame axis y=1.5: body centre (y=1.0) is 0.5 away (> 0.2+0.15); wing
    // (y=1.6) is 0.1 away (< 0.35).
    let hit = flame_hits_target_parts(
        FirePose2d::new(Vec2::new(1.5, 1.5), 0.0, 0.0),
        5.0,
        0.15,
        &map,
        target,
    )
    .expect("wing should be hit even though the centre is outside the strip");
    assert_eq!(
        hit.part_id, LEG,
        "flame hits the offset part, not the centre"
    );
}

#[test]
fn flame_centre_inside_but_wall_before_part_blocks() {
    // Enemy centre on the flame axis (inside the strip) but behind wall (3,2).
    let map = test_map();
    let set = single_body_set(0.3);
    let target = PartHitscanTarget {
        position: Vec2::new(5.5, 2.5),
        yaw: 0.0,
        alive: true,
        set: &set,
        animation: ANIM,
        frame: 0,
        fallback_radius: 0.3,
    };
    let hit = flame_hits_target_parts(
        FirePose2d::new(Vec2::new(1.5, 2.5), 0.0, 0.0),
        8.0,
        0.3,
        &map,
        target,
    );
    assert!(hit.is_none(), "wall before the part blocks flame damage");
}

#[test]
fn flame_wall_stops_damage_like_visuals() {
    // Before the wall → hit; behind the wall → no hit. Damage stop distance
    // equals the visual stop distance (both use wall obstruction).
    let map = test_map();
    let set = single_body_set(0.3);
    let pose = FirePose2d::new(Vec2::new(1.5, 2.5), 0.0, 0.0);

    let before = PartHitscanTarget {
        position: Vec2::new(2.5, 2.5),
        yaw: 0.0,
        alive: true,
        set: &set,
        animation: ANIM,
        frame: 0,
        fallback_radius: 0.3,
    };
    let behind = PartHitscanTarget {
        position: Vec2::new(5.5, 2.5),
        ..before
    };

    assert!(
        flame_hits_target_parts(pose, 8.0, 0.3, &map, before).is_some(),
        "enemy before wall is burned"
    );
    assert!(
        flame_hits_target_parts(pose, 8.0, 0.3, &map, behind).is_none(),
        "enemy behind wall is not burned"
    );
    // Visual stop distance is the same wall obstruction the damage uses.
    let visual = crate::combat::flame_visual_max_distance(&map, Vec2::new(1.5, 2.5), Vec2::X, 8.0);
    assert!(
        approx(visual, 1.5),
        "visuals stop at the wall (x=3.0): {visual}"
    );
}

#[test]
fn flame_visual_pitch_ignored() {
    let map = test_map();
    let set = single_body_set(0.3);
    let mk = || PartHitscanTarget {
        position: Vec2::new(3.0, 1.5),
        yaw: 0.0,
        alive: true,
        set: &set,
        animation: ANIM,
        frame: 0,
        fallback_radius: 0.3,
    };
    let flat = flame_hits_target_parts(
        FirePose2d::new(Vec2::new(1.5, 1.5), 0.0, 0.0),
        5.0,
        0.3,
        &map,
        mk(),
    );
    let pitched = flame_hits_target_parts(
        FirePose2d::new(Vec2::new(1.5, 1.5), 0.0, 9999.0),
        5.0,
        0.3,
        &map,
        mk(),
    );
    assert_eq!(
        flat, pitched,
        "visual_pitch_px must not change flame damage"
    );
}

#[test]
fn flame_yaw_changes_selected_part() {
    // Flame fired from the east at a multi-facing enemy. Facing toward the
    // attacker exposes the head; facing away leaves only the body.
    let map = test_map();
    let set = multi_facing_set();
    let enemy = Vec2::new(3.0, 1.5);
    let pose = pose_west(Vec2::new(5.5, 1.5)); // fire -X from the east

    let facing_attacker = PartHitscanTarget {
        position: enemy,
        yaw: 0.0, // faces +X (east) = toward the attacker
        alive: true,
        set: &set,
        animation: ANIM,
        frame: 0,
        fallback_radius: 0.3,
    };
    let facing_away = PartHitscanTarget {
        yaw: std::f32::consts::PI, // faces -X (west) = away
        ..facing_attacker
    };

    let front = flame_hits_target_parts(pose, 8.0, 0.2, &map, facing_attacker).unwrap();
    let back = flame_hits_target_parts(pose, 8.0, 0.2, &map, facing_away).unwrap();
    assert_eq!(front.part_id, HEAD, "facing attacker → head");
    assert_eq!(back.part_id, BODY, "facing away → body only");
}

#[test]
fn flame_fallback_circle_when_frame_missing() {
    // No frame for the queried animation → whole-body swept-circle fallback.
    let map = test_map();
    let set = single_body_set(0.3);
    let mut t = PartHitscanTarget {
        position: Vec2::new(3.0, 1.5),
        yaw: 0.0,
        alive: true,
        set: &set,
        animation: AnimationKey(99), // unregistered
        frame: 0,
        fallback_radius: 0.3,
    };
    let hit = flame_hits_target_parts(
        FirePose2d::new(Vec2::new(1.5, 1.5), 0.0, 0.0),
        5.0,
        0.2,
        &map,
        t,
    )
    .expect("fallback circle should be hit");
    assert_eq!(hit.part_id, PartId::FALLBACK);

    // A target well off-axis with zero fallback radius is not hit.
    t.fallback_radius = 0.0;
    assert!(
        flame_hits_target_parts(
            FirePose2d::new(Vec2::new(1.5, 1.5), 0.0, 0.0),
            5.0,
            0.2,
            &map,
            t
        )
        .is_none(),
        "zero fallback radius → no hit"
    );
}

#[test]
fn flame_origin_inside_wall_does_no_damage() {
    // Origin in the west border wall (cell (0,*)) → wall-capped length is 0,
    // so no part can be reached regardless of the target.
    let map = test_map();
    let set = single_body_set(0.3);
    let target = PartHitscanTarget {
        position: Vec2::new(2.5, 1.5),
        yaw: 0.0,
        alive: true,
        set: &set,
        animation: ANIM,
        frame: 0,
        fallback_radius: 0.3,
    };
    let hit = flame_hits_target_parts(
        FirePose2d::new(Vec2::new(0.5, 1.5), 0.0, 0.0), // x=0.5 is inside the border wall
        8.0,
        0.3,
        &map,
        target,
    );
    assert!(hit.is_none(), "flame from inside a wall deals no damage");
}

/// Single part offset laterally (local +Y), no on-axis body — lets a wide flame
/// strip reach a part that sits behind a wall corner off the fire axis.
fn flame_offset_only_set() -> TargetCollisionSet {
    let mut set = TargetCollisionSet::new();
    for facing in BillboardFacing8::ALL {
        set.insert_frame(
            CollisionFrameKey {
                animation: ANIM,
                frame: 0,
                facing,
            },
            TargetCollisionFrame::new([PartCollider2d {
                part_id: LEG,
                collider: Collider::Circle(Circle::new(Vec2::new(0.0, 0.8), 0.2)),
            }]),
        );
    }
    set
}

#[test]
fn flame_off_axis_part_behind_wall_corner_is_blocked() {
    // Flame axis runs along open row y=1.5; the part sits at world (3.5, 2.3),
    // fully behind wall (3,2). The wide strip reaches it geometrically, but the
    // contact point is inside the wall cell so per-contact LOS blocks it.
    let map = test_map();
    let set = flame_offset_only_set();
    let pose = FirePose2d::new(Vec2::new(1.5, 1.5), 0.0, 0.0); // +X along y=1.5

    let behind_corner = PartHitscanTarget {
        position: Vec2::new(3.5, 1.5), // part local (0,0.8) → world (3.5, 2.3)
        yaw: 0.0,
        alive: true,
        set: &set,
        animation: ANIM,
        frame: 0,
        fallback_radius: 0.0,
    };
    assert!(
        flame_hits_target_parts(pose, 8.0, 0.7, &map, behind_corner).is_none(),
        "off-axis part behind a wall corner is LOS-blocked"
    );

    // Control: same fixture in fully open space (part at world (3.5, 1.8)) IS
    // hit — proving the block above is LOS, not a geometric miss.
    let in_open = PartHitscanTarget {
        position: Vec2::new(3.5, 1.0), // part → world (3.5, 1.8), all open rows
        ..behind_corner
    };
    let hit = flame_hits_target_parts(pose, 8.0, 0.7, &map, in_open)
        .expect("clear LOS to the offset part should hit");
    assert_eq!(hit.part_id, LEG);
}

// ---------------------------------------------------------------------------
// Part damage routing (Phase 5)
// ---------------------------------------------------------------------------

const WEAK: PartId = PartId(4);

#[test]
fn scaled_damage_rule() {
    assert!(
        (scaled_damage(37.0, 2.0) - 74.0).abs() < 1e-4,
        "headshot 2x"
    );
    assert!((scaled_damage(37.0, 1.0) - 37.0).abs() < 1e-4, "neutral");
    assert!(
        (scaled_damage(37.0, 0.75) - 27.75).abs() < 1e-4,
        "limb 0.75"
    );
    assert!(
        (scaled_damage(37.0, f32::NAN) - 37.0).abs() < 1e-4,
        "non-finite → base"
    );
    assert!(
        scaled_damage(37.0, -1.0).abs() < 1e-4,
        "negative clamped to 0"
    );
}

/// Body at centre + a weak point offset forward (local +X), with metadata:
/// body 1.0, weak point 3.0.
fn routing_set() -> TargetCollisionSet {
    let mut set = TargetCollisionSet::new();
    for facing in BillboardFacing8::ALL {
        set.insert_frame(
            CollisionFrameKey {
                animation: ANIM,
                frame: 0,
                facing,
            },
            TargetCollisionFrame::new([
                PartCollider2d {
                    part_id: BODY,
                    collider: Collider::Circle(Circle::new(Vec2::ZERO, 0.25)),
                },
                PartCollider2d {
                    part_id: WEAK,
                    collider: Collider::Circle(Circle::new(Vec2::new(0.3, 0.0), 0.15)),
                },
            ]),
        );
    }
    set.insert_part_metadata(
        BODY,
        PartMetadata {
            material: MaterialId(1),
            damage_scale: 1.0,
        },
    );
    set.insert_part_metadata(
        WEAK,
        PartMetadata {
            material: MaterialId(9),
            damage_scale: 3.0,
        },
    );
    set
}

#[test]
fn hitscan_returns_part_damage_scale() {
    let map = test_map();
    let set = routing_set();
    let enemy = Vec2::new(3.5, 1.5);

    // Enemy faces +X (toward the east shooter): weak point is forward → hit.
    let weak = hitscan_parts_from_pose(
        pose_west(Vec2::new(5.5, 1.5)),
        &map,
        [target(&set, enemy, 0.3)].into_iter(),
    )
    .unwrap();
    assert_eq!(weak.part_id, WEAK);
    assert!(
        (weak.damage_scale - 3.0).abs() < 1e-4,
        "weak point scale 3.0"
    );
    assert_eq!(weak.material, Some(MaterialId(9)));

    // From the west, the body occludes the (eastward) weak point → body hit.
    let body = hitscan_parts_from_pose(
        pose_east(Vec2::new(1.5, 1.5)),
        &map,
        [target(&set, enemy, 0.3)].into_iter(),
    )
    .unwrap();
    assert_eq!(body.part_id, BODY);
    assert!((body.damage_scale - 1.0).abs() < 1e-4, "body neutral scale");
}

#[test]
fn hitscan_fallback_uses_neutral_scale() {
    let map = test_map();
    let set = routing_set();
    let mut t = target(&set, Vec2::new(2.5, 1.5), 0.3);
    t.animation = AnimationKey(99); // unregistered → fallback circle
    let hit =
        hitscan_parts_from_pose(pose_east(Vec2::new(1.5, 1.5)), &map, [t].into_iter()).unwrap();
    assert_eq!(hit.part_id, PartId::FALLBACK);
    assert!(
        (hit.damage_scale - 1.0).abs() < 1e-4,
        "fallback is neutral 1.0"
    );
    assert_eq!(hit.material, None);
}

#[test]
fn part_with_no_metadata_is_neutral_scale() {
    // multi_facing_set registers parts but NO metadata → neutral 1.0.
    let map = test_map();
    let set = multi_facing_set();
    let hit = hitscan_parts_from_pose(
        pose_west(Vec2::new(5.5, 1.5)),
        &map,
        [target(&set, Vec2::new(3.5, 1.5), 0.3)].into_iter(),
    )
    .unwrap();
    assert_eq!(hit.part_id, HEAD);
    assert!(
        (hit.damage_scale - 1.0).abs() < 1e-4,
        "no metadata → neutral"
    );
    assert_eq!(hit.material, None);
}
