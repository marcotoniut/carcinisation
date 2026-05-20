//! ECS-based interpolation components for remote entities.
//!
//! Smooths replicated position and angle between 30 Hz server snapshots.
//! Attached to remote `NetPlayer` and `NetEnemy` entities so the rendering
//! loop can read interpolated values directly from the ECS.

use bevy::prelude::*;

/// Returns the shortest signed angular delta from `from` to `to`,
/// wrapping around the 0/TAU boundary.
#[must_use]
pub fn shortest_angle_delta(from: f32, to: f32) -> f32 {
    let d = (to - from).rem_euclid(std::f32::consts::TAU);
    if d > std::f32::consts::PI {
        d - std::f32::consts::TAU
    } else {
        d
    }
}

/// Smooths replicated position between 30 Hz snapshots (visual only).
#[derive(Component, Clone, Debug)]
pub struct RemotePositionInterpolation {
    pub prev: Vec2,
    pub target: Vec2,
    pub elapsed: f32,
    pub interval: f32,
}

impl RemotePositionInterpolation {
    #[must_use]
    pub fn new(position: Vec2) -> Self {
        Self {
            prev: position,
            target: position,
            elapsed: 0.0,
            interval: 1.0 / 30.0,
        }
    }

    /// Returns the interpolated visual position.
    #[must_use]
    pub fn interpolated(&self) -> Vec2 {
        let t = (self.elapsed / self.interval).min(1.0);
        self.prev.lerp(self.target, t)
    }

    /// If `new_value` differs from `target`, shifts target to prev,
    /// sets new target, and adapts the interval estimate. Otherwise no-op.
    pub fn update_if_changed(&mut self, new_value: Vec2) {
        if (new_value - self.target).length_squared() > 1e-10 {
            self.prev = self.target;
            self.target = new_value;
            if self.elapsed > 0.001 {
                self.interval = self.elapsed.clamp(0.016, 0.2);
            }
            self.elapsed = 0.0;
        }
    }

    /// Advances elapsed time.
    pub fn tick(&mut self, dt: f32) {
        self.elapsed += dt;
    }
}

/// Smooths replicated angle between 30 Hz snapshots (visual only).
#[derive(Component, Clone, Debug)]
pub struct RemoteAngleInterpolation {
    pub prev: f32,
    pub target: f32,
    pub elapsed: f32,
    pub interval: f32,
}

impl RemoteAngleInterpolation {
    #[must_use]
    pub fn new(angle: f32) -> Self {
        Self {
            prev: angle,
            target: angle,
            elapsed: 0.0,
            interval: 1.0 / 30.0,
        }
    }

    /// Returns the interpolated visual angle using shortest-arc interpolation.
    #[must_use]
    pub fn interpolated(&self) -> f32 {
        let t = (self.elapsed / self.interval).min(1.0);
        self.prev + shortest_angle_delta(self.prev, self.target) * t
    }

    /// If `new_value` differs from `target` (wrapping-aware), shifts target
    /// to prev, sets new target, and adapts the interval estimate. Otherwise no-op.
    pub fn update_if_changed(&mut self, new_value: f32) {
        if shortest_angle_delta(self.target, new_value).abs() > 1e-5 {
            self.prev = self.target;
            self.target = new_value;
            if self.elapsed > 0.001 {
                self.interval = self.elapsed.clamp(0.016, 0.2);
            }
            self.elapsed = 0.0;
        }
    }

    /// Advances elapsed time.
    pub fn tick(&mut self, dt: f32) {
        self.elapsed += dt;
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn position_interpolation_starts_at_initial_value() {
        let interp = RemotePositionInterpolation::new(Vec2::new(1.0, 2.0));
        assert_eq!(interp.interpolated(), Vec2::new(1.0, 2.0));
    }

    #[test]
    fn position_interpolation_lerps_after_update() {
        let mut interp = RemotePositionInterpolation::new(Vec2::new(0.0, 0.0));
        // Simulate some time passing before a new snapshot.
        interp.tick(1.0 / 30.0);
        interp.update_if_changed(Vec2::new(1.0, 0.0));
        // Elapsed just reset, so interpolated should be at prev (0,0).
        assert_eq!(interp.interpolated(), Vec2::new(0.0, 0.0));
        // Advance halfway through the interval.
        interp.tick(interp.interval / 2.0);
        let pos = interp.interpolated();
        assert!((pos.x - 0.5).abs() < 0.01);
    }

    #[test]
    fn position_interpolation_clamps_at_target() {
        let mut interp = RemotePositionInterpolation::new(Vec2::new(0.0, 0.0));
        interp.tick(1.0 / 30.0);
        interp.update_if_changed(Vec2::new(1.0, 0.0));
        // Advance well past the interval.
        interp.tick(1.0);
        assert_eq!(interp.interpolated(), Vec2::new(1.0, 0.0));
    }

    #[test]
    fn angle_interpolation_starts_at_initial_value() {
        let interp = RemoteAngleInterpolation::new(1.5);
        assert!((interp.interpolated() - 1.5).abs() < 1e-6);
    }

    #[test]
    fn angle_interpolation_lerps_after_update() {
        let mut interp = RemoteAngleInterpolation::new(0.0);
        interp.tick(1.0 / 30.0);
        interp.update_if_changed(1.0);
        assert!((interp.interpolated() - 0.0).abs() < 1e-6);
        interp.tick(interp.interval / 2.0);
        assert!((interp.interpolated() - 0.5).abs() < 0.01);
    }

    #[test]
    fn update_if_changed_ignores_same_value() {
        let mut interp = RemotePositionInterpolation::new(Vec2::new(1.0, 2.0));
        interp.tick(0.05);
        let elapsed_before = interp.elapsed;
        interp.update_if_changed(Vec2::new(1.0, 2.0));
        // Elapsed should NOT have been reset.
        assert!((interp.elapsed - elapsed_before).abs() < 1e-6);
    }

    #[test]
    fn interval_adapts_to_elapsed() {
        let mut interp = RemotePositionInterpolation::new(Vec2::ZERO);
        // Simulate 50ms between updates.
        interp.tick(0.05);
        interp.update_if_changed(Vec2::new(1.0, 0.0));
        assert!((interp.interval - 0.05).abs() < 1e-6);
    }

    #[test]
    fn angle_interpolation_wraps_shortest_arc() {
        use std::f32::consts::TAU;
        // From 6.0 rad (~344 deg) to 0.2 rad (~11 deg).
        // The short way is +0.483 rad (forward through 0/TAU), not -5.8 rad.
        let mut interp = RemoteAngleInterpolation::new(6.0);
        interp.tick(1.0 / 30.0);
        interp.update_if_changed(0.2);
        // At t=0, interpolated == prev (6.0).
        assert!((interp.interpolated() - 6.0).abs() < 1e-5);
        // At halfway, should be roughly 6.0 + 0.5 * delta.
        interp.tick(interp.interval / 2.0);
        let mid = interp.interpolated();
        // Expected midpoint: 6.0 + 0.5*(0.2 - 6.0 + TAU) = 6.0 + 0.5*0.483 ~ 6.24
        let expected_delta = (0.2_f32 - 6.0).rem_euclid(TAU);
        let expected_mid = 6.0
            + 0.5
                * if expected_delta > std::f32::consts::PI {
                    expected_delta - TAU
                } else {
                    expected_delta
                };
        assert!(
            (mid - expected_mid).abs() < 0.02,
            "mid={mid:.3} expected={expected_mid:.3} -- should take short arc"
        );
        // At t=1, should reach target.
        interp.tick(interp.interval);
        let end = interp.interpolated();
        // end should be close to 0.2 (or 0.2 + TAU, since we add delta to prev).
        let wrapped_end = end.rem_euclid(TAU);
        assert!(
            (wrapped_end - 0.2).abs() < 0.02,
            "end={end:.3} wrapped={wrapped_end:.3} target=0.2 -- should reach target via short arc"
        );
    }

    #[test]
    fn angle_update_if_changed_detects_wrapping_change() {
        // target=0.1, new_value=6.2 -- these are close across the TAU boundary
        // but far apart in linear distance. Wrapping-aware comparison should
        // detect them as close (~0.18 rad apart).
        let mut interp = RemoteAngleInterpolation::new(0.1);
        interp.tick(1.0 / 30.0);
        // Set target to 0.1 explicitly.
        let elapsed_before = interp.elapsed;
        // Value 6.2 is ~0.08 rad before TAU, so delta from 0.1 is about -0.18.
        // That exceeds the 1e-5 threshold, so it SHOULD update.
        interp.update_if_changed(6.2);
        assert!(
            interp.elapsed < elapsed_before,
            "should have detected wrapping change and reset elapsed"
        );
    }
}
