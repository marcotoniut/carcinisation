//! First-person camera state — re-exported from `carcinisation_fps_core`.

pub use carcinisation_fps_core::camera::*;

#[cfg(test)]
mod tests {
    use super::*;

    /// Smoke test: verify that `Camera` and its methods resolve through the re-export.
    #[test]
    fn re_exported_camera_resolves() {
        let cam = Camera::default();
        let dir = cam.direction();
        let plane = cam.plane();
        assert!(dir.length() > 0.0);
        assert!(plane.length() > 0.0);
    }
}
