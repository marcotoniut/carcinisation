//! First-person camera state.

use bevy_math::Vec2;

/// Compute distance-banded bob attenuation.
/// Returns 1.0 for close, 0.5 for mid, 0.0 for far.
#[must_use]
pub fn view_bob_strength(distance: f32, near: f32, mid: f32) -> f32 {
    if distance < near {
        1.0
    } else if distance < mid {
        0.5
    } else {
        0.0
    }
}

/// First-person camera: position in map space, facing angle, field of view.
#[derive(Clone, Debug)]
pub struct Camera {
    /// Position in map-space units (1.0 = one grid cell).
    pub position: Vec2,
    /// Facing angle in radians. 0 = east (+X), PI/2 = north (+Y).
    pub angle: f32,
    /// Horizontal field of view in radians.
    pub fov: f32,
    /// Vertical view bob offset in pixels (positive = look up).
    /// Driven by walk animation, shifts the horizon line.
    pub view_bob: f32,
    /// Distance threshold for full view bob (map units).
    pub view_bob_near: f32,
    /// Distance threshold for half view bob (map units). Beyond this, bob is zero.
    pub view_bob_mid: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            position: Vec2::new(4.0, 4.0),
            angle: 0.0,
            fov: 66.0_f32.to_radians(), // ~66° horizontal, close to Wolf3D
            view_bob: 0.0,
            view_bob_near: 3.0,
            view_bob_mid: 6.0,
        }
    }
}

impl Camera {
    /// Unit direction vector the camera is facing.
    #[must_use]
    pub fn direction(&self) -> Vec2 {
        Vec2::new(self.angle.cos(), self.angle.sin())
    }

    /// Camera plane vector (perpendicular to direction, scaled by FOV).
    /// Points to the **right** of the view direction.
    #[must_use]
    pub fn plane(&self) -> Vec2 {
        let dir = self.direction();
        let plane_len = (self.fov / 2.0).tan();
        // Right-hand perpendicular: rotate dir 90° clockwise.
        Vec2::new(dir.y, -dir.x) * plane_len
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plane_is_perpendicular_to_direction() {
        let cam = Camera {
            position: Vec2::ZERO,
            angle: 0.5,
            fov: 1.0,
            ..Default::default()
        };
        let dot = cam.direction().dot(cam.plane());
        assert!(
            dot.abs() < 1e-5,
            "plane must be perpendicular to dir, dot={dot}"
        );
    }

    #[test]
    fn plane_points_right_of_direction() {
        // Facing east (angle=0): right is south (-Y in math Y-up).
        let cam = Camera {
            position: Vec2::ZERO,
            angle: 0.0,
            fov: std::f32::consts::FRAC_PI_2,
            ..Default::default()
        };
        let plane = cam.plane();
        assert!(
            plane.y < 0.0,
            "plane should point south (right of east), got {plane}"
        );
    }

    #[test]
    fn plane_magnitude_matches_fov() {
        let fov = 1.2_f32;
        let cam = Camera {
            position: Vec2::ZERO,
            angle: 0.0,
            fov,
            ..Default::default()
        };
        let expected_len = (fov / 2.0).tan();
        let actual_len = cam.plane().length();
        assert!(
            (actual_len - expected_len).abs() < 1e-5,
            "plane length {actual_len} != expected {expected_len}"
        );
    }
}
