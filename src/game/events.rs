use bevy::prelude::*;

#[derive(Clone, Event)]
pub struct GameOverTrigger {
    /// TODO review score
    pub score: u32,
}

#[derive(Event)]
pub struct GameStartupTrigger;
