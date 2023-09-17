use bevy::prelude::*;

#[derive(Component)]
pub struct Crosshair {
    pub name: String
}

pub struct CrosshairBundle {
    pub crosshair: Crosshair
}

#[derive(Resource)]
pub struct CrosshairSettings(pub u8);

impl Default for CrosshairSettings {
    fn default() -> Self {
        CrosshairSettings(1)
    }
} 