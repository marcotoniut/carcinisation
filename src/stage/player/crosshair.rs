use bevy::prelude::*;

#[derive(Component)]
pub struct Crosshair {
    pub name: String,
}

#[derive(Debug, Clone, Copy, Resource)]
pub struct CrosshairSettings(pub u8);

impl Default for CrosshairSettings {
    fn default() -> Self {
        CrosshairSettings(1)
    }
}
