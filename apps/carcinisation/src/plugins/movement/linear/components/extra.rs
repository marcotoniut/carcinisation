use crate::core::time::DeltaTime;
use bevy::prelude::*;
use derive_new::new;
use std::marker::PhantomData;

#[derive(new, Component, Clone, Debug, Reflect)]
pub struct LinearMovement2DReachCheck<T: DeltaTime + Send + Sync + 'static, X, Y> {
    #[new(default)]
    phantom_t: PhantomData<T>,
    #[new(default)]
    phantom_x: PhantomData<X>,
    #[new(default)]
    phantom_y: PhantomData<Y>,
    #[new(value = "(false, false)")]
    pub reached: (bool, bool),
}

impl<T: DeltaTime + Send + Sync + 'static, X, Y> LinearMovement2DReachCheck<T, X, Y> {
    pub fn reached(&self) -> bool {
        self.reached.0 && self.reached.1
    }
}
