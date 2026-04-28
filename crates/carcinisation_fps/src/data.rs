//! RON-serializable data types for first-person maps.

use serde::Deserialize;

use crate::camera::FpCamera;
use crate::map::FpMap;
use crate::render::FpPalette;

/// Top-level map definition loaded from `.fp_map.ron`.
#[derive(Deserialize, Debug)]
pub struct FpMapData {
    pub width: usize,
    pub height: usize,
    /// Row-major cell grid. 0 = empty, >0 = wall texture index (1-based).
    pub cells: Vec<u8>,
    pub wall_textures: Vec<WallTextureSpec>,
    pub ceiling_color: u8,
    pub floor_color: u8,
    pub player_start: PlayerStart,
    #[serde(default)]
    pub entities: Vec<FpEntitySpawn>,
}

/// How to generate or load a wall texture.
#[derive(Deserialize, Debug)]
pub enum WallTextureSpec {
    /// Procedural brick pattern.
    Brick {
        color: u8,
        mortar: u8,
        #[serde(default = "default_tex_size")]
        size: u32,
    },
    /// Procedural checkerboard.
    Checker {
        color_a: u8,
        color_b: u8,
        #[serde(default = "default_block_size")]
        block: u32,
        #[serde(default = "default_tex_size")]
        size: u32,
    },
}

fn default_tex_size() -> u32 {
    64
}
fn default_block_size() -> u32 {
    8
}

/// Player spawn position and facing.
#[derive(Deserialize, Debug)]
pub struct PlayerStart {
    pub x: f32,
    pub y: f32,
    pub angle_deg: f32,
}

/// An entity to place in the map.
#[derive(Deserialize, Debug)]
pub struct FpEntitySpawn {
    pub kind: FpEntityKind,
    pub x: f32,
    pub y: f32,
}

/// Entity types that can be spawned in an FP map.
#[derive(Deserialize, Debug)]
pub enum FpEntityKind {
    /// Static column/pillar billboard.
    Pillar { color: u8, width: u32, height: u32 },
    /// Enemy with procedural sprite.
    Enemy {
        color: u8,
        health: u32,
        #[serde(default = "default_enemy_speed")]
        speed: f32,
    },
    /// Enemy using an asset-loaded sprite (e.g. Mosquito).
    SpriteEnemy {
        /// Sprite path relative to assets/ (e.g. "sprites/enemies/mosquito_fly_3.px_sprite.png").
        sprite: String,
        /// Death sprite path.
        death_sprite: String,
        health: u32,
        #[serde(default = "default_enemy_speed")]
        speed: f32,
    },
}

fn default_enemy_speed() -> f32 {
    1.5
}

// --- Conversion helpers ---

impl FpMapData {
    /// Build the runtime map from this data.
    ///
    /// # Panics
    ///
    /// `cells` length does not equal `width * height`.
    #[must_use]
    pub fn to_map(&self) -> FpMap {
        let expected = self.width * self.height;
        assert_eq!(
            self.cells.len(),
            expected,
            "FpMapData: cells length ({}) must equal width * height ({}x{} = {expected})",
            self.cells.len(),
            self.width,
            self.height,
        );
        FpMap {
            width: self.width,
            height: self.height,
            cells: self.cells.clone(),
        }
    }

    /// Build the camera from player start.
    #[must_use]
    pub fn to_camera(&self) -> FpCamera {
        FpCamera {
            position: bevy_math::Vec2::new(self.player_start.x, self.player_start.y),
            angle: self.player_start.angle_deg.to_radians(),
            ..Default::default()
        }
    }

    /// Build the palette config.
    #[must_use]
    pub fn to_palette(&self) -> FpPalette {
        FpPalette {
            ceiling: self.ceiling_color,
            floor: self.floor_color,
            ..Default::default()
        }
    }

    /// Generate wall textures from specs.
    #[must_use]
    pub fn build_wall_textures(&self) -> Vec<carapace::image::CxImage> {
        use crate::render::{make_brick_texture, make_checker_texture};
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

    /// Load an `FpMapData` from a RON string.
    ///
    /// # Errors
    ///
    /// Returns a `ron::error::SpannedError` if parsing fails.
    pub fn from_ron(s: &str) -> Result<Self, ron::error::SpannedError> {
        ron::from_str(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINIMAL_RON: &str = r#"
        FpMapData(
            width: 3,
            height: 3,
            cells: [1,0,1, 0,0,0, 1,0,1],
            wall_textures: [Brick(color: 2, mortar: 1)],
            ceiling_color: 1,
            floor_color: 3,
            player_start: (x: 1.5, y: 1.5, angle_deg: 90.0),
        )
    "#;

    #[test]
    fn parse_minimal_ron() {
        let data = FpMapData::from_ron(MINIMAL_RON).unwrap();
        assert_eq!(data.width, 3);
        assert_eq!(data.height, 3);
        assert_eq!(data.cells.len(), 9);
        assert_eq!(data.wall_textures.len(), 1);
        assert!(data.entities.is_empty());
    }

    #[test]
    fn to_map_validates_cells_length() {
        let data = FpMapData {
            width: 4,
            height: 4,
            cells: vec![0; 10], // wrong: 10 != 16
            wall_textures: vec![],
            ceiling_color: 0,
            floor_color: 0,
            player_start: PlayerStart {
                x: 1.0,
                y: 1.0,
                angle_deg: 0.0,
            },
            entities: vec![],
        };
        let result = std::panic::catch_unwind(|| data.to_map());
        assert!(result.is_err());
    }

    #[test]
    fn to_camera_converts_degrees_to_radians() {
        let data = FpMapData::from_ron(MINIMAL_RON).unwrap();
        let cam = data.to_camera();
        assert!((cam.angle - 90.0_f32.to_radians()).abs() < 1e-5);
    }

    #[test]
    fn invalid_ron_returns_error() {
        assert!(FpMapData::from_ron("not valid ron {{{").is_err());
    }
}
