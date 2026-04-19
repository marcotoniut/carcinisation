//! Top-level game resources (difficulty selection, etc.).

use crate::game::resources::Difficulty;
use bevy::prelude::*;

#[derive(Resource, Debug, Clone, Eq, PartialEq, Default)]
pub struct DifficultySelected(pub Difficulty);

/// Developer flags loaded from `.env` at startup.
///
/// All flags default to `false` (normal behaviour) when unset.
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct DevFlags {
    /// Skip the main menu and boot directly into gameplay.
    pub skip_menu: bool,
    /// Auto-skip cutscenes as soon as they start.
    pub skip_cutscenes: bool,
}
