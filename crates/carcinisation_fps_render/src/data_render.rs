//! Carapace-dependent data helpers for rendering.

use carcinisation_fps_core::data::{MapData, WallTextureSpec};
use carcinisation_fps_core::render::{make_brick_texture, make_checker_texture};

/// Extension trait to add carapace-dependent methods to MapData.
pub trait MapDataRenderExt {
    /// Generate wall textures from specs using carapace.
    fn build_wall_textures(&self) -> Vec<carapace::image::CxImage>;
}

impl MapDataRenderExt for MapData {
    fn build_wall_textures(&self) -> Vec<carapace::image::CxImage> {
        self.wall_textures
            .iter()
            .map(|spec| match spec {
                WallTextureSpec::Brick {
                    color,
                    mortar,
                    size,
                } => make_brick_texture(*size, *color, *mortar),
                WallTextureSpec::Checker {
                    color_a,
                    color_b,
                    block,
                    size,
                } => make_checker_texture(*size, *block, *color_a, *color_b),
            })
            .collect()
    }
}
