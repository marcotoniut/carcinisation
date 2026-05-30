//! Runtime rendering configuration for the map view.
//!
//! Values are computed from screen and map dimensions at plugin init and
//! remain constant for the lifetime of the map.

use bevy::prelude::*;

/// Map view rendering parameters, computed from screen + map dimensions at init.
#[derive(Resource)]
pub struct MapViewConfig {
    /// Pixels per map cell (computed to fill screen).
    pub tile_size: u32,
    /// Marker sprite size in pixels (scaled from `tile_size`).
    pub marker_size: u32,
}

impl Default for MapViewConfig {
    fn default() -> Self {
        Self {
            tile_size: 4,
            marker_size: 4,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_marker_size_matches_tile_size() {
        let config = MapViewConfig::default();
        assert_eq!(config.tile_size, config.marker_size);
    }
}
