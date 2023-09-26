use std::marker::PhantomData;

use bevy::prelude::*;
use seldom_pixel::prelude::PxSubPosition;

pub trait DeltaTime: Send + Sync + 'static {
    fn delta_seconds(&self) -> f32;
}

pub trait LinearPosition: Send + Sync + 'static {
    fn get(&self) -> Vec2;
    fn set(&mut self, value: Vec2);
    fn add(&mut self, value: Vec2);
}

impl LinearPosition for PxSubPosition {
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

#[derive(Component, Debug, Clone)]
pub struct LinearTargetPosition<T: DeltaTime + Send + Sync + 'static> {
    _marker: PhantomData<T>,
    pub value: Vec2,
}

impl<T: DeltaTime + Send + Sync + 'static> LinearTargetPosition<T> {
    pub fn new(value: Vec2) -> Self {
        Self {
            _marker: PhantomData,
            value,
        }
    }
}

// TODO split into LinearX, Y, Z
#[derive(Component, Debug, Clone)]
pub struct LinearSpeed<T: DeltaTime + Send + Sync + 'static> {
    _marker: PhantomData<T>,
    pub value: Vec2,
}

impl<T: DeltaTime + Send + Sync + 'static> LinearSpeed<T> {
    pub fn new(value: Vec2) -> Self {
        Self {
            _marker: PhantomData,
            value,
        }
    }
}

#[derive(Component, Debug, Clone)]
pub struct LinearTargetXReached<T: DeltaTime + Send + Sync + 'static> {
    _marker: PhantomData<T>,
}

impl<T: DeltaTime + Send + Sync + 'static> LinearTargetXReached<T> {
    pub fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

#[derive(Component, Debug, Clone)]
pub struct LinearTargetYReached<T: DeltaTime + Send + Sync + 'static> {
    _marker: PhantomData<T>,
}

impl<T: DeltaTime + Send + Sync + 'static> LinearTargetYReached<T> {
    pub fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

#[derive(Component, Debug, Clone)]
pub struct LinearTargetReached<T: DeltaTime + Send + Sync + 'static> {
    _marker: PhantomData<T>,
}

impl<T: DeltaTime + Send + Sync + 'static> LinearTargetReached<T> {
    pub fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

// TODO LinearBundle (which should also clear up any previous TargetReached that the component may have)
// The other components shouldn't be used on their own
