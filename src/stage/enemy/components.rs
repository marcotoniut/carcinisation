use bevy::prelude::*;

#[derive(Component)]
pub struct Enemy {
    pub direction: Vec2,
}

pub const ENEMY_SPEED: f32 = 10.0;

pub const ENEMY_SIZE: f32 = 6.0;
pub const NUMBER_OF_ENEMIES: usize = 1;

pub const ENEMY_SPAWN_TIME: f32 = 15.0;
