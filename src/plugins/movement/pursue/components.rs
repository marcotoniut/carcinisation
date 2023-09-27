use std::marker::PhantomData;

use bevy::prelude::*;
use seldom_pixel::prelude::PxSubPosition;

use crate::core::time::DeltaTime;

pub trait Pursue: Send + Sync + 'static {
    fn get(&self) -> Vec2;
    fn set(&mut self, value: Vec2);
    fn add(&mut self, value: Vec2);
}

impl Pursue for PxSubPosition {
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
pub struct PursueTargetPosition<T: DeltaTime + Send + Sync + 'static> {
    _marker: PhantomData<T>,
    pub value: Vec2,
}

impl<T: DeltaTime + Send + Sync + 'static> PursueTargetPosition<T> {
    pub fn new(value: Vec2) -> Self {
        Self {
            _marker: PhantomData,
            value,
        }
    }
}

// TODO split into LinearX, Y, Z
#[derive(Component, Debug, Clone)]
pub struct PursueSpeed<T: DeltaTime + Send + Sync + 'static> {
    _marker: PhantomData<T>,
    pub value: Vec2,
}

impl<T: DeltaTime + Send + Sync + 'static> PursueSpeed<T> {
    pub fn new(value: Vec2) -> Self {
        Self {
            _marker: PhantomData,
            value,
        }
    }
}

#[derive(Component, Debug, Clone)]
pub struct PursueTargetXReached<T: DeltaTime + Send + Sync + 'static> {
    _marker: PhantomData<T>,
}

impl<T: DeltaTime + Send + Sync + 'static> PursueTargetXReached<T> {
    pub fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

#[derive(Component, Debug, Clone)]
pub struct PursueTargetYReached<T: DeltaTime + Send + Sync + 'static> {
    _marker: PhantomData<T>,
}

impl<T: DeltaTime + Send + Sync + 'static> PursueTargetYReached<T> {
    pub fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

#[derive(Component, Debug, Clone)]
pub struct PursueTargetReached<T: DeltaTime + Send + Sync + 'static> {
    _marker: PhantomData<T>,
}

impl<T: DeltaTime + Send + Sync + 'static> PursueTargetReached<T> {
    pub fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

// TODO LinearBundle (which should also clear up any previous TargetReached that the component may have)
// The other components shouldn't be used on their own
