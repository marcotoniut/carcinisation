//! Shared first-person movement logic used by both singleplayer and server.

use bevy_math::Vec2;

use crate::collision::try_move;
use crate::map::Map;

// ---------------------------------------------------------------------------
// Speed modifier (e.g. web slow)
// ---------------------------------------------------------------------------

/// Temporary speed multiplier applied to player movement.
///
/// Drains over time. Moving accelerates the drain — the more you move,
/// the faster you break free. Refreshes on re-application (budget resets,
/// multiplier replaced). Does NOT stack multiplicatively.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpeedModifier {
    /// Multiplier applied to base movement speed. Clamped to `0.1..=1.0`.
    pub multiplier: f32,
    /// Remaining budget (arbitrary units). Drained by time + movement.
    pub remaining: f32,
    /// How fast the budget drains per second while standing still.
    pub base_drain_rate: f32,
    /// Extra drain per map-unit of movement distance.
    pub movement_drain_rate: f32,
}

impl SpeedModifier {
    /// Create a new modifier, clamping `multiplier` defensively.
    ///
    /// `budget` is the total drain budget. At `base_drain_rate = 1.0`,
    /// it expires in `budget` seconds while standing still.
    #[must_use]
    pub fn new(multiplier: f32, budget: f32) -> Self {
        Self {
            multiplier: multiplier.clamp(0.1, 1.0),
            remaining: budget.max(0.0),
            base_drain_rate: 1.0,
            movement_drain_rate: 2.0,
        }
    }

    /// Tick the modifier. `movement_distance` is the distance moved this frame.
    /// Returns `true` if still active after ticking.
    pub fn tick(&mut self, dt: f32, movement_distance: f32) -> bool {
        let drain = self.base_drain_rate * dt + self.movement_drain_rate * movement_distance;
        self.remaining -= drain;
        self.remaining > 0.0
    }

    /// Refresh (re-apply) — resets budget, replaces multiplier. Does not stack.
    pub fn refresh(&mut self, multiplier: f32, budget: f32) {
        self.multiplier = multiplier.clamp(0.1, 1.0);
        self.remaining = budget.max(0.0);
    }

    /// Effective multiplier (1.0 if expired).
    #[must_use]
    pub fn effective(&self) -> f32 {
        if self.remaining > 0.0 {
            self.multiplier
        } else {
            1.0
        }
    }
}

/// Unit direction vector for a given facing angle.
/// 0 = east (+X), PI/2 = north (+Y).
#[must_use]
pub fn direction_from_angle(angle: f32) -> Vec2 {
    Vec2::new(angle.cos(), angle.sin())
}

/// Right-hand perpendicular (points right of the facing direction).
#[must_use]
pub fn right_from_angle(angle: f32) -> Vec2 {
    let dir = direction_from_angle(angle);
    Vec2::new(dir.y, -dir.x)
}

/// Transform player-local intent to a world-space movement vector.
///
/// `local_intent.y` = forward (+) / backward (-).
/// `local_intent.x` = strafe right (+) / strafe left (-).
#[must_use]
pub fn local_to_world(angle: f32, local_intent: Vec2) -> Vec2 {
    let dir = direction_from_angle(angle);
    let right = Vec2::new(dir.y, -dir.x);
    dir * local_intent.y + right * local_intent.x
}

/// Apply movement to a position with collision.
///
/// Transforms `local_intent` from player-local space to world-space using
/// `angle`, scales by `speed * delta_time`, then runs axis-separated
/// collision against `map`.
pub fn apply_movement(
    position: &mut Vec2,
    angle: f32,
    local_intent: Vec2,
    speed: f32,
    delta_time: f32,
    map: &Map,
    collision_margin: f32,
) {
    let world_move = local_to_world(angle, local_intent);
    let delta = world_move * speed * delta_time;
    try_move(position, delta, collision_margin, map);
}

/// Apply movement with an optional speed modifier (e.g. web slow).
///
/// Identical to [`apply_movement`] but multiplies speed by the modifier's
/// effective value. Call sites that don't use modifiers keep using
/// `apply_movement` directly.
#[allow(clippy::too_many_arguments)]
pub fn apply_movement_with_modifier(
    position: &mut Vec2,
    angle: f32,
    local_intent: Vec2,
    speed: f32,
    modifier: Option<&SpeedModifier>,
    delta_time: f32,
    map: &Map,
    collision_margin: f32,
) {
    let effective_speed = speed * modifier.map_or(1.0, SpeedModifier::effective);
    apply_movement(
        position,
        angle,
        local_intent,
        effective_speed,
        delta_time,
        map,
        collision_margin,
    );
}

// ---------------------------------------------------------------------------
// Snap / Quick Turn
// ---------------------------------------------------------------------------

/// Snap turn kind — shared between SP and server.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SnapTurnKind {
    /// 180° turn left.
    QuickTurn,
    /// 90° turn left.
    Left,
    /// 90° turn right.
    Right,
}

/// Parameters for starting a snap turn animation.
///
/// Returned by [`snap_turn_params`]; the caller stores these in its own
/// adapter state (`QuickTurnState`, `ServerQuickTurn`, etc.).
#[derive(Clone, Copy, Debug)]
pub struct SnapTurnParams {
    pub remaining_radians: f32,
    pub speed: f32,
    pub direction: f32,
}

/// Compute the parameters for a snap turn of the given kind.
///
/// `quick_turn_duration_secs` is the duration of a full 180° turn;
/// 90° turns complete in half this time at the same angular speed.
#[must_use]
pub fn snap_turn_params(kind: SnapTurnKind, quick_turn_duration_secs: f32) -> SnapTurnParams {
    let angular_speed = std::f32::consts::PI / quick_turn_duration_secs;
    match kind {
        SnapTurnKind::QuickTurn => SnapTurnParams {
            remaining_radians: std::f32::consts::PI,
            speed: angular_speed,
            direction: 1.0,
        },
        SnapTurnKind::Left => SnapTurnParams {
            remaining_radians: std::f32::consts::FRAC_PI_2,
            speed: angular_speed,
            direction: 1.0,
        },
        SnapTurnKind::Right => SnapTurnParams {
            remaining_radians: std::f32::consts::FRAC_PI_2,
            speed: angular_speed,
            direction: -1.0,
        },
    }
}

/// Tick a snap turn animation. Advances `angle` by the appropriate step,
/// clamped to `remaining_radians`. Returns the updated remaining radians.
///
/// When the return value is `<= 0.0`, the turn is complete.
pub fn tick_snap_turn(
    angle: &mut f32,
    remaining_radians: &mut f32,
    speed: f32,
    direction: f32,
    dt: f32,
) {
    if *remaining_radians <= 0.0 {
        return;
    }
    let step = (speed * dt).min(*remaining_radians).max(0.0);
    *angle = (*angle + step * direction).rem_euclid(std::f32::consts::TAU);
    *remaining_radians -= step;
}

/// Maximum angular velocity for derived turn values (rad/s).
/// Prevents extreme spikes from teleports or snap turns.
const MAX_DERIVED_TURN_VELOCITY: f32 = 12.0;

/// Compute angular velocity from two consecutive angles, clamped to a safe range.
///
/// Handles wrapping correctly. Returns 0 when `dt` is near zero.
#[must_use]
pub fn angular_velocity_clamped(current: f32, previous: f32, dt: f32) -> f32 {
    if dt <= f32::EPSILON {
        return 0.0;
    }
    let diff = (current - previous + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
        - std::f32::consts::PI;
    (diff / dt).clamp(-MAX_DERIVED_TURN_VELOCITY, MAX_DERIVED_TURN_VELOCITY)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::FpsMovementConfig;
    use std::f32::consts::{FRAC_PI_2, PI};

    // -- SpeedModifier tests --

    #[test]
    fn speed_modifier_applies_multiplier() {
        let map = crate::map::test_map();
        let defaults = FpsMovementConfig::default();
        let modifier = SpeedModifier::new(0.7, 5.0);

        // With modifier.
        let mut pos_slow = Vec2::new(3.5, 3.5);
        apply_movement_with_modifier(
            &mut pos_slow,
            0.0,
            Vec2::new(0.0, 1.0),
            defaults.move_speed,
            Some(&modifier),
            1.0,
            &map,
            defaults.collision_margin,
        );

        // Without modifier.
        let mut pos_normal = Vec2::new(3.5, 3.5);
        apply_movement(
            &mut pos_normal,
            0.0,
            Vec2::new(0.0, 1.0),
            defaults.move_speed,
            1.0,
            &map,
            defaults.collision_margin,
        );

        let slow_dist = pos_slow.x - 3.5;
        let normal_dist = pos_normal.x - 3.5;
        let ratio = slow_dist / normal_dist;
        assert!(
            (ratio - 0.7).abs() < 0.01,
            "modifier should apply 0.7x speed: ratio={ratio}"
        );
    }

    #[test]
    fn speed_modifier_expires() {
        let mut modifier = SpeedModifier::new(0.7, 1.0);

        assert!(modifier.tick(0.5, 0.0));
        assert_eq!(modifier.effective(), 0.7);

        assert!(!modifier.tick(0.6, 0.0));
        assert_eq!(modifier.effective(), 1.0);
    }

    #[test]
    fn speed_modifier_drains_faster_with_movement() {
        let mut standing = SpeedModifier::new(0.7, 3.0);
        let mut moving = SpeedModifier::new(0.7, 3.0);

        // Same dt, but moving player covers 0.5 units.
        standing.tick(1.0, 0.0);
        moving.tick(1.0, 0.5);

        assert!(
            moving.remaining < standing.remaining,
            "movement should drain faster: standing={} moving={}",
            standing.remaining,
            moving.remaining,
        );
    }

    #[test]
    fn speed_modifier_refresh_does_not_stack() {
        let mut modifier = SpeedModifier::new(0.7, 3.0);

        // Tick partway.
        modifier.tick(1.0, 0.0);
        assert!((modifier.remaining - 2.0).abs() < f32::EPSILON);

        // Refresh with same multiplier — budget resets, does NOT become 0.49.
        modifier.refresh(0.7, 3.0);
        assert_eq!(modifier.multiplier, 0.7);
        assert!((modifier.remaining - 3.0).abs() < f32::EPSILON);
    }

    #[test]
    fn speed_modifier_clamps_multiplier() {
        let low = SpeedModifier::new(0.0, 1.0);
        assert_eq!(low.multiplier, 0.1);

        let high = SpeedModifier::new(2.0, 1.0);
        assert_eq!(high.multiplier, 1.0);
    }

    #[test]
    fn no_modifier_means_full_speed() {
        let map = crate::map::test_map();
        let defaults = FpsMovementConfig::default();

        let mut pos_a = Vec2::new(3.5, 3.5);
        apply_movement_with_modifier(
            &mut pos_a,
            0.0,
            Vec2::new(0.0, 1.0),
            defaults.move_speed,
            None,
            1.0,
            &map,
            defaults.collision_margin,
        );

        let mut pos_b = Vec2::new(3.5, 3.5);
        apply_movement(
            &mut pos_b,
            0.0,
            Vec2::new(0.0, 1.0),
            defaults.move_speed,
            1.0,
            &map,
            defaults.collision_margin,
        );

        assert!(
            (pos_a - pos_b).length() < 1e-6,
            "None modifier should match normal: a={pos_a:?} b={pos_b:?}"
        );
    }

    #[test]
    fn forward_at_angle_zero_is_east() {
        let world = local_to_world(0.0, Vec2::new(0.0, 1.0));
        assert!((world.x - 1.0).abs() < 1e-5, "x should be 1: {world:?}");
        assert!(world.y.abs() < 1e-5, "y should be 0: {world:?}");
    }

    #[test]
    fn forward_at_pi_half_is_north() {
        let world = local_to_world(FRAC_PI_2, Vec2::new(0.0, 1.0));
        assert!(world.x.abs() < 1e-5, "x should be 0: {world:?}");
        assert!((world.y - 1.0).abs() < 1e-5, "y should be 1: {world:?}");
    }

    #[test]
    fn strafe_right_at_angle_zero_is_south() {
        let world = local_to_world(0.0, Vec2::new(1.0, 0.0));
        assert!(world.x.abs() < 1e-5, "x should be 0: {world:?}");
        assert!((world.y - (-1.0)).abs() < 1e-5, "y should be -1: {world:?}");
    }

    #[test]
    fn backward_at_pi_is_east() {
        // Facing west (PI), backward = east (+X).
        let world = local_to_world(PI, Vec2::new(0.0, -1.0));
        assert!((world.x - 1.0).abs() < 1e-4, "x should be ~1: {world:?}");
        assert!(world.y.abs() < 1e-4, "y should be ~0: {world:?}");
    }

    #[test]
    fn apply_movement_uses_collision() {
        use crate::map::test_map;
        let defaults = FpsMovementConfig::default();
        let map = test_map();
        let mut pos = Vec2::new(1.1, 1.5);
        // Face west (PI), move forward with small dt → blocked by west wall at x=0.
        apply_movement(
            &mut pos,
            PI,
            Vec2::new(0.0, 1.0),
            defaults.move_speed,
            0.033,
            &map,
            defaults.collision_margin,
        );
        assert!(
            (pos.x - 1.1).abs() < 0.01,
            "x should be blocked by wall: {pos:?}"
        );
    }

    /// Verify that `apply_movement` produces the same result as manually computing
    /// world-space delta + `try_move`. This ensures SP (which now calls
    /// `apply_movement`) and any manual delta path produce identical positions.
    #[test]
    fn apply_movement_matches_manual_delta() {
        use crate::collision::try_move;
        use crate::map::test_map;

        let defaults = FpsMovementConfig::default();
        let map = test_map();
        let angle = 0.5_f32;
        let intent = Vec2::new(-0.7, 0.8); // strafe left + forward
        let speed = defaults.move_speed;
        let dt = 0.033;

        // Path A: apply_movement (shared function).
        let mut pos_a = Vec2::new(3.5, 3.5);
        apply_movement(
            &mut pos_a,
            angle,
            intent,
            speed,
            dt,
            &map,
            defaults.collision_margin,
        );

        // Path B: manual world-space delta + try_move (old SP pattern).
        let mut pos_b = Vec2::new(3.5, 3.5);
        let world_move = local_to_world(angle, intent);
        let delta = world_move * speed * dt;
        try_move(&mut pos_b, delta, defaults.collision_margin, &map);

        assert!(
            (pos_a - pos_b).length() < 1e-6,
            "apply_movement and manual delta should produce identical positions: \
             a={pos_a:?} b={pos_b:?}"
        );
    }

    // -- Snap turn tests --

    #[test]
    fn quick_turn_completes_pi_radians() {
        let params = snap_turn_params(SnapTurnKind::QuickTurn, 0.4);
        let mut angle = 0.0_f32;
        let mut remaining = params.remaining_radians;

        // Tick for full duration.
        tick_snap_turn(
            &mut angle,
            &mut remaining,
            params.speed,
            params.direction,
            0.4,
        );

        assert!(remaining <= 0.0, "turn should be complete");
        assert!(
            (angle - PI).abs() < 1e-4,
            "should have turned PI radians: {angle}"
        );
    }

    #[test]
    fn side_turn_left_completes_half_pi() {
        let params = snap_turn_params(SnapTurnKind::Left, 0.4);
        let mut angle = 0.0_f32;
        let mut remaining = params.remaining_radians;

        tick_snap_turn(
            &mut angle,
            &mut remaining,
            params.speed,
            params.direction,
            0.2,
        );

        assert!(
            remaining <= 0.0,
            "90° turn should complete in half duration"
        );
        assert!(
            (angle - FRAC_PI_2).abs() < 1e-4,
            "should have turned PI/2 left: {angle}"
        );
    }

    #[test]
    fn side_turn_right_is_negative_direction() {
        let params = snap_turn_params(SnapTurnKind::Right, 0.4);
        let mut angle = FRAC_PI_2;
        let mut remaining = params.remaining_radians;

        tick_snap_turn(
            &mut angle,
            &mut remaining,
            params.speed,
            params.direction,
            0.2,
        );

        assert!(remaining <= 0.0);
        assert!(angle.abs() < 1e-4, "PI/2 - PI/2 = 0: {angle}");
    }

    #[test]
    fn partial_tick_leaves_remaining() {
        let params = snap_turn_params(SnapTurnKind::QuickTurn, 0.4);
        let mut angle = 0.0_f32;
        let mut remaining = params.remaining_radians;

        // Tick half the duration.
        tick_snap_turn(
            &mut angle,
            &mut remaining,
            params.speed,
            params.direction,
            0.2,
        );

        assert!(remaining > 0.0, "should have remaining radians");
        assert!(
            (angle - FRAC_PI_2).abs() < 1e-4,
            "should have turned half: {angle}"
        );
    }

    #[test]
    fn overshoot_clamps_to_remaining() {
        let params = snap_turn_params(SnapTurnKind::Left, 0.4);
        let mut angle = 0.0_f32;
        let mut remaining = params.remaining_radians;

        // Tick way past completion.
        tick_snap_turn(
            &mut angle,
            &mut remaining,
            params.speed,
            params.direction,
            10.0,
        );

        assert!(remaining <= 0.0);
        assert!(
            (angle - FRAC_PI_2).abs() < 1e-4,
            "should clamp to exactly PI/2: {angle}"
        );
    }

    #[test]
    fn zero_remaining_is_noop() {
        let mut angle = 1.0_f32;
        let mut remaining = 0.0_f32;

        tick_snap_turn(&mut angle, &mut remaining, 10.0, 1.0, 1.0);

        assert!(
            (angle - 1.0).abs() < f32::EPSILON,
            "should not change angle when remaining is 0"
        );
    }

    #[test]
    fn angle_wraps_past_tau() {
        let params = snap_turn_params(SnapTurnKind::QuickTurn, 0.4);
        let mut angle = std::f32::consts::TAU - 0.1;
        let mut remaining = params.remaining_radians;

        tick_snap_turn(
            &mut angle,
            &mut remaining,
            params.speed,
            params.direction,
            0.4,
        );

        assert!(
            angle < std::f32::consts::TAU,
            "angle should wrap via rem_euclid"
        );
        assert!(angle >= 0.0);
    }
}
