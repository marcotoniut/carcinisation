use bevy::{prelude::*, utils::HashMap};

use crate::plugins::movement::structs::MovementVec2Position;

#[derive(Component, Debug, Clone, Copy)]
pub struct Depth(pub u8);

#[derive(Component, Debug, Clone, Copy)]
pub struct Floor(pub f32);

#[derive(Component, Debug, Clone, Copy)]
pub struct Speed(pub f32);

#[derive(Component, Debug)]
pub struct InView {}

#[derive(Component, Debug)]
pub struct LinearUpdateDisabled;

// DEPRECATED
#[derive(Clone, Component, Debug)]
pub struct RailPosition(pub Vec2);

// DEPRECATED
impl MovementVec2Position for RailPosition {
    fn get(&self) -> Vec2 {
        self.0
    }
    fn set(&mut self, position: Vec2) {
        self.0 = position;
    }
    fn add(&mut self, position: Vec2) {
        self.0 += position;
    }
}

pub fn spawn_floor_depths(commands: &mut Commands, floor_depths: &HashMap<u8, f32>) {
    for (floor, depth) in floor_depths.iter() {
        commands.spawn((Floor(*depth), Depth(*floor)));
    }
}
