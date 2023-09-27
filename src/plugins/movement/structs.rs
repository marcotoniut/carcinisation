use bevy::prelude::Vec2;
use seldom_pixel::prelude::PxSubPosition;

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

pub trait MovementAxisPosition: Send + Sync + 'static {
    fn get(&self) -> f32;
    fn set(&mut self, value: f32);
    fn add(&mut self, value: f32);
}
