use bevy::prelude::*;

#[derive(Event)]
pub struct GameOver {
    pub score: u32,
}

#[derive(Event)]
pub struct GameStartupEvent;
