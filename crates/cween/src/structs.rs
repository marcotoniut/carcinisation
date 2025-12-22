use bevy::prelude::*;
use seldom_pixel::prelude::PxSubPosition;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Reflect, Serialize)]
pub enum TweenDirection {
    Negative,
    Positive,
}

impl TweenDirection {
    pub fn opposite(&self) -> TweenDirection {
        match self {
            TweenDirection::Negative => TweenDirection::Positive,
            TweenDirection::Positive => TweenDirection::Negative,
        }
    }
}

pub trait MovementVec2Position: Send + Sync + 'static {
    fn get(&self) -> Vec2;
    fn set(&mut self, value: Vec2);
    fn add(&mut self, value: Vec2);
}

impl MovementVec2Position for PxSubPosition {
    fn get(&self) -> Vec2 {
        self.0
    }
    fn set(&mut self, value: Vec2) {
        self.0 = value;
    }
    fn add(&mut self, value: Vec2) {
        self.0 += value;
    }
}

pub trait Constructor<T>: Send + Sync + 'static {
    fn new(x: T) -> Self;
}

pub trait Magnitude: Send + Sync + 'static {
    fn get(&self) -> f32;
    fn set(&mut self, value: f32);
    fn add(&mut self, value: f32);
}
