use crate::core::time::DeltaTime;
use bevy::prelude::*;
use std::marker::PhantomData;

#[derive(Component, Clone, Debug, Reflect)]
pub struct LinearMovement2DReachCheck<T: DeltaTime + Send + Sync + 'static, X, Y> {
    phantom_t: PhantomData<T>,
    phantom_x: PhantomData<X>,
    phantom_y: PhantomData<Y>,
    pub reached: (bool, bool),
}

impl<T: DeltaTime + Send + Sync + 'static, X, Y> LinearMovement2DReachCheck<T, X, Y> {
    pub fn new() -> Self {
        Self {
            phantom_t: PhantomData,
            phantom_x: PhantomData,
            phantom_y: PhantomData,
            reached: (false, false),
        }
    }

    pub fn reached(&self) -> bool {
        self.reached.0 && self.reached.1
    }
}
