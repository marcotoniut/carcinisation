//! Top-level game resources (difficulty selection, etc.).

use crate::game::resources::Difficulty;
use bevy::prelude::*;

#[derive(Resource, Debug, Clone, Eq, PartialEq, Default)]
pub struct DifficultySelected(pub Difficulty);
