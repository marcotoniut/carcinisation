use bevy::prelude::*;

use crate::plugins::movement::structs::MovementVec2Position;

#[derive(Component, Debug, Clone, Copy)]
pub struct Depth(pub usize);

#[derive(Component, Debug)]
pub struct InView {}

#[derive(Clone, Component, Debug)]
pub struct RailPosition(pub Vec2);

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
