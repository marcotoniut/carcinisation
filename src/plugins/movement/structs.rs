use bevy::prelude::Vec2;
use seldom_pixel::prelude::PxSubPosition;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum MovementDirection {
    // TODO should this implement default at all?
    #[default]
    Negative,
    Positive,
}

impl MovementDirection {
    pub fn opposite(&self) -> MovementDirection {
        match self {
            MovementDirection::Negative => MovementDirection::Positive,
            MovementDirection::Positive => MovementDirection::Negative,
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
