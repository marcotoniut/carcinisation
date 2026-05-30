use bevy::prelude::*;

/// Map view rendering parameters, computed from screen + map dimensions at init.
#[derive(Resource)]
pub struct MapViewConfig {
    /// Pixels per map cell (computed to fill screen).
    pub tile_size: u32,
    /// Marker sprite size in pixels (scaled from tile_size).
    pub marker_size: u32,
}

impl Default for MapViewConfig {
    fn default() -> Self {
        Self {
            tile_size: 4,
            marker_size: 10,
        }
    }
}
