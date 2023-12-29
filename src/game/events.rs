use bevy::prelude::*;

#[derive(Event)]
pub struct GameOverEvent {
    /// TODO review score
    pub score: u32,
}

#[derive(Event)]
pub struct GameStartupEvent;
