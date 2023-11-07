use bevy::prelude::*;

#[derive(Event)]
pub struct LetterboxMoveEvent {
    pub speed: f32,
    pub row: f32,
}
