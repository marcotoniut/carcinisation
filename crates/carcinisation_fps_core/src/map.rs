//! Grid-based map representation for first-person stages.

/// Error type for map loading.
#[derive(Debug)]
pub enum MapError {
    Ron(ron::error::SpannedError),
    Validation(String),
}

impl std::fmt::Display for MapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ron(e) => write!(f, "RON parse error: {e}"),
            Self::Validation(msg) => write!(f, "map validation error: {msg}"),
        }
    }
}

impl std::error::Error for MapError {}

impl From<ron::error::SpannedError> for MapError {
    fn from(e: ron::error::SpannedError) -> Self {
        Self::Ron(e)
    }
}

/// A 2D grid map where each cell is either empty (0) or a wall type (>0).
#[derive(Clone, Debug)]
pub struct Map {
    pub width: usize,
    pub height: usize,
    /// Row-major cell data. `cells[y * width + x]`.
    /// 0 = empty, >0 = wall texture ID.
    pub cells: Vec<u8>,
}

impl Map {
    /// Look up the cell at grid position `(x, y)`.
    /// Returns 1 (solid wall) for out-of-bounds coordinates so that
    /// collision treats the map boundary as impassable.
    #[must_use]
    pub fn get(&self, x: i32, y: i32) -> u8 {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return 1;
        }
        self.cells[y as usize * self.width + x as usize]
    }

    /// Load a map from a RON string (the `MapData(...)` format used by `.fp_map.ron` files).
    /// Only reads `width`, `height`, and `cells`; rendering-specific fields are ignored.
    ///
    /// # Errors
    /// Returns `MapError::Parse` if RON deserialization fails, or `MapError::Validation`
    /// if cell count doesn't match width × height.
    pub fn from_ron(ron_str: &str) -> Result<Self, MapError> {
        #[derive(serde::Deserialize)]
        #[allow(dead_code)]
        struct MapData {
            width: usize,
            height: usize,
            cells: Vec<u8>,
        }

        let data: MapData = ron::from_str(ron_str)?;
        let expected = data.width * data.height;
        if data.cells.len() != expected {
            return Err(MapError::Validation(format!(
                "cells length ({}) != width * height ({}x{}={expected})",
                data.cells.len(),
                data.width,
                data.height,
            )));
        }
        Ok(Map {
            width: data.width,
            height: data.height,
            cells: data.cells,
        })
    }

    /// Load a map AND its entity spawn list from a RON string.
    ///
    /// # Errors
    /// Returns `MapError::Parse` if RON deserialization fails, or `MapError::Validation`
    /// if cell count doesn't match width × height.
    pub fn from_ron_with_entities(ron_str: &str) -> Result<(Self, Vec<EntitySpawnData>), MapError> {
        let data = MapLoadData::from_ron(ron_str)?;
        Ok((data.map, data.entities))
    }

    /// Load map geometry, entity spawns, and player starts from a RON string.
    ///
    /// # Errors
    /// Returns `MapError::Parse` if RON deserialization fails, or `MapError::Validation`
    /// if cell count doesn't match width × height.
    pub fn load_data(ron_str: &str) -> Result<MapLoadData, MapError> {
        #[derive(serde::Deserialize)]
        #[allow(dead_code)]
        struct MapData {
            width: usize,
            height: usize,
            cells: Vec<u8>,
            #[serde(default)]
            player_start: PlayerStartData,
            #[serde(default)]
            player_starts: Vec<PlayerStartData>,
            #[serde(default)]
            entities: Vec<EntitySpawnData>,
        }

        let data: MapData = ron::from_str(ron_str)?;
        let expected = data.width * data.height;
        if data.cells.len() != expected {
            return Err(MapError::Validation(format!(
                "cells length ({}) != width * height ({}x{}={expected})",
                data.cells.len(),
                data.width,
                data.height,
            )));
        }
        let map = Map {
            width: data.width,
            height: data.height,
            cells: data.cells,
        };

        let mut player_starts = data.player_starts;
        if player_starts.is_empty()
            && map.get(
                data.player_start.x.floor() as i32,
                data.player_start.y.floor() as i32,
            ) == 0
        {
            player_starts.push(data.player_start);
        }

        Ok(MapLoadData {
            map,
            entities: data.entities,
            player_starts,
        })
    }
}

/// Runtime data parsed from a first-person map file.
#[derive(Clone, Debug)]
pub struct MapLoadData {
    pub map: Map,
    pub entities: Vec<EntitySpawnData>,
    pub player_starts: Vec<PlayerStartData>,
}

impl MapLoadData {
    /// Parse a map file into `MapLoadData`.
    ///
    /// # Errors
    /// Returns `MapError::Parse` if RON deserialization fails, or `MapError::Validation`
    /// if cell count doesn't match width × height.
    pub fn from_ron(ron_str: &str) -> Result<Self, MapError> {
        Map::load_data(ron_str)
    }
}

/// A player spawn point parsed from `player_start` or `player_starts`.
#[derive(serde::Deserialize, Debug, Clone, Copy, Default)]
pub struct PlayerStartData {
    pub x: f32,
    pub y: f32,
    pub angle_deg: f32,
}

// ---------------------------------------------------------------------------
// Entity spawn data (parsed from .fp_map.ron)
// ---------------------------------------------------------------------------

/// An entity to place in the map.
#[derive(serde::Deserialize, Debug, Clone)]
pub struct EntitySpawnData {
    pub kind: EntitySpawnKind,
    pub x: f32,
    pub y: f32,
}

/// Entity types that can appear in a map file.
/// Field names match the legacy `carcinisation_fps::data::EntityKind` so the
/// same RON files parse identically.
#[derive(serde::Deserialize, Debug, Clone)]
pub enum EntitySpawnKind {
    Pillar {
        color: u8,
        width: u32,
        height: u32,
    },
    Enemy {
        color: u8,
        health: u32,
        #[serde(default = "default_enemy_speed")]
        speed: f32,
    },
    SpriteEnemy {
        sprite: String,
        death_sprite: String,
        health: u32,
        #[serde(default = "default_enemy_speed")]
        speed: f32,
    },
    Mosquiton {
        #[serde(default = "default_mosquiton_health")]
        health: u32,
        #[serde(default = "default_enemy_speed")]
        speed: f32,
    },
}

fn default_mosquiton_health() -> u32 {
    40
}
fn default_enemy_speed() -> f32 {
    1.5
}

impl EntitySpawnKind {
    /// Health value for enemy-like kinds. Returns `None` for decorative entities.
    #[must_use]
    pub fn health(&self) -> Option<u32> {
        match self {
            Self::Pillar { .. } => None,
            Self::Enemy { health, .. }
            | Self::SpriteEnemy { health, .. }
            | Self::Mosquiton { health, .. } => Some(*health),
        }
    }

    /// Whether this is an enemy (not decorative).
    #[must_use]
    pub fn is_enemy(&self) -> bool {
        !matches!(self, Self::Pillar { .. })
    }
}

/// Hardcoded 8x8 test map for M0.
///
/// ```text
/// 1 1 1 1 1 1 1 1
/// 1 . . . . . . 1
/// 1 . . 2 2 . . 1
/// 1 . . . . . . 1
/// 1 . 2 . . 2 . 1
/// 1 . . . . . . 1
/// 1 . . . . . . 1
/// 1 1 1 1 1 1 1 1
/// ```
#[must_use]
pub fn test_map() -> Map {
    #[rustfmt::skip]
    let cells = vec![
        1, 1, 1, 1, 1, 1, 1, 1,
        1, 0, 0, 0, 0, 0, 0, 1,
        1, 0, 0, 2, 2, 0, 0, 1,
        1, 0, 0, 0, 0, 0, 0, 1,
        1, 0, 2, 0, 0, 2, 0, 1,
        1, 0, 0, 0, 0, 0, 0, 1,
        1, 0, 0, 0, 0, 0, 0, 1,
        1, 1, 1, 1, 1, 1, 1, 1,
    ];
    Map {
        width: 8,
        height: 8,
        cells,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_boundaries() {
        let map = test_map();
        // Corners are walls.
        assert_eq!(map.get(0, 0), 1);
        assert_eq!(map.get(7, 7), 1);
        // Interior is empty.
        assert_eq!(map.get(1, 1), 0);
        // Interior wall.
        assert_eq!(map.get(3, 2), 2);
        // Out of bounds → solid wall.
        assert_eq!(map.get(-1, 0), 1);
        assert_eq!(map.get(8, 0), 1);
    }

    #[test]
    fn from_ron_with_entities_parses_spawns() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../carcinisation_fps/../../assets/config/fp/test_room.fp_map.ron"
        );
        let ron = std::fs::read_to_string(path).expect("read test_room.fp_map.ron");
        let data = Map::load_data(&ron).expect("parse");
        let map = data.map;
        let entities = data.entities;

        assert_eq!(map.width, 12);
        assert_eq!(map.height, 12);
        assert_eq!(data.player_starts.len(), 1);
        assert!((data.player_starts[0].x - 6.0).abs() < 0.001);
        assert!((data.player_starts[0].y - 6.0).abs() < 0.001);
        assert!((data.player_starts[0].angle_deg - 0.0).abs() < 0.001);

        let enemies: Vec<_> = entities.iter().filter(|e| e.kind.is_enemy()).collect();
        let pillars: Vec<_> = entities.iter().filter(|e| !e.kind.is_enemy()).collect();

        assert_eq!(enemies.len(), 6, "expected 6 Mosquitons");
        assert_eq!(pillars.len(), 4, "expected 4 Pillars");

        // All enemies should have health.
        for e in &enemies {
            assert!(e.kind.health().is_some());
        }
    }

    #[test]
    fn from_ron_without_entities_still_works() {
        let ron = r"MapData(width: 2, height: 2, cells: [1,0,0,1], wall_textures: [], ceiling_color: 0, floor_color: 0, player_start: (x: 0.5, y: 0.5, angle_deg: 0.0))";
        let map = Map::from_ron(ron).expect("parse");
        assert_eq!(map.width, 2);
    }

    #[test]
    fn out_of_bounds_returns_wall_all_edges() {
        let map = test_map();
        // Negative coordinates.
        assert_ne!(map.get(-1, 0), 0, "OOB west should be wall");
        assert_ne!(map.get(0, -1), 0, "OOB north should be wall");
        assert_ne!(map.get(-1, -1), 0, "OOB corner NW should be wall");
        // Beyond dimensions.
        assert_ne!(map.get(map.width as i32, 0), 0, "OOB east should be wall");
        assert_ne!(map.get(0, map.height as i32), 0, "OOB south should be wall");
        assert_ne!(
            map.get(map.width as i32, map.height as i32),
            0,
            "OOB corner SE should be wall"
        );
    }
}
