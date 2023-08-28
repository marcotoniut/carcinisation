use bevy::prelude::*;

#[derive(Component)]
pub struct Enemy {
    pub direction: Vec2,
}

pub const ENEMY_SPEED: f32 = 100.0;

pub const ENEMY_SIZE: f32 = 12.0;
pub const NUMBER_OF_ENEMIES: usize = 3;

pub const ENEMY_SPAWN_TIME: f32 = 5.0;