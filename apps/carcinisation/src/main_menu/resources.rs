//! Main menu resources (difficulty selection etc.).

use crate::game::resources::Difficulty;
use bevy::prelude::*;

#[derive(Resource, Debug, Clone, Eq, PartialEq, Default)]
/// Stores the difficulty chosen from the menu.
pub struct DifficultySelection(pub Difficulty);
