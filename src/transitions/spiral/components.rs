use bevy::prelude::*;

#[derive(Component)]
pub struct TransitionVenetian {
    pub row: u32,
}

pub const TRANSITION_UPDATE_TIME: f32 = 0.015;
