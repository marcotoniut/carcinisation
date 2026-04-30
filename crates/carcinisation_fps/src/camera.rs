//! First-person camera state.

use bevy_math::Vec2;

/// First-person camera: position in map space, facing angle, field of view.
#[derive(Clone, Debug)]
pub struct FpCamera {
    /// Position in map-space units (1.0 = one grid cell).
    pub position: Vec2,
    /// Facing angle in radians. 0 = east (+X), PI/2 = north (+Y).
    pub angle: f32,
    /// Horizontal field of view in radians.
    pub fov: f32,
}

impl Default for FpCamera {
    fn default() -> Self {
        Self {
            position: Vec2::new(4.0, 4.0),
            angle: 0.0,
            fov: 66.0_f32.to_radians(), // ~66° horizontal, close to Wolf3D
        }
    }
}

impl FpCamera {
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
        let cam = FpCamera {
            position: Vec2::ZERO,
            angle: 0.5,
            fov: 1.0,
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
        let cam = FpCamera {
            position: Vec2::ZERO,
            angle: 0.0,
            fov: std::f32::consts::FRAC_PI_2,
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
        let cam = FpCamera {
            position: Vec2::ZERO,
            angle: 0.0,
            fov,
        };
        let expected_len = (fov / 2.0).tan();
        let actual_len = cam.plane().length();
        assert!(
            (actual_len - expected_len).abs() < 1e-5,
            "plane length {actual_len} != expected {expected_len}"
        );
    }
}
