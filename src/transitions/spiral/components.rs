use bevy::prelude::*;

#[derive(Component)]
pub struct TransitionVenetianRow {
    pub row: u32,
}

#[derive(Component)]
pub struct TransitionVenetian;

pub const TRANSITION_UPDATE_TIME: f32 = 0.015;
