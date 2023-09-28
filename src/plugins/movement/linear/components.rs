use std::marker::PhantomData;

use bevy::prelude::*;

use crate::{core::time::DeltaTime, plugins::movement::structs::MovementAxisPosition};

#[derive(Component, Debug, Clone)]
pub struct XAxisPosition(pub f32);

impl MovementAxisPosition for XAxisPosition {
    fn get(&self) -> f32 {
        self.0
    }

    fn set(&mut self, value: f32) {
        self.0 = value;
    }

    fn add(&mut self, value: f32) {
        self.0 += value;
    }
}

#[derive(Component, Debug, Clone)]
pub struct YAxisPosition(pub f32);

impl MovementAxisPosition for YAxisPosition {
    fn get(&self) -> f32 {
        self.0
    }

    fn set(&mut self, value: f32) {
        self.0 = value;
    }

    fn add(&mut self, value: f32) {
        self.0 += value;
    }
}

#[derive(Component, Debug, Clone)]
pub struct ZAxisPosition(pub f32);

impl MovementAxisPosition for ZAxisPosition {
    fn get(&self) -> f32 {
        self.0
    }

    fn set(&mut self, value: f32) {
        self.0 = value;
    }

    fn add(&mut self, value: f32) {
        self.0 += value;
    }
}

// TODO Bundle and on added

#[derive(Component, Debug, Clone)]
pub struct LinearTargetPosition<T: DeltaTime + Send + Sync + 'static, P> {
    _marker_time: PhantomData<T>,
    _marker_position: PhantomData<P>,
    pub value: f32,
}

impl<T: DeltaTime + Send + Sync + 'static, P: MovementAxisPosition> LinearTargetPosition<T, P> {
    pub fn new(value: f32) -> Self {
        Self {
            _marker_time: PhantomData,
            _marker_position: PhantomData,
            value,
        }
    }
}

// TODO split into LinearX, Y, Z
#[derive(Component, Debug, Clone)]
pub struct LinearSpeed<T: DeltaTime + Send + Sync + 'static, P: MovementAxisPosition> {
    _marker_position: PhantomData<P>,
    _marker_time: PhantomData<T>,
    pub value: f32,
}

impl<T: DeltaTime + Send + Sync + 'static, P: MovementAxisPosition> LinearSpeed<T, P> {
    pub fn new(value: f32) -> Self {
        Self {
            _marker_position: PhantomData,
            _marker_time: PhantomData,
            value,
        }
    }
}

#[derive(Component, Debug, Clone)]
pub struct LinearTargetReached<T: DeltaTime + Send + Sync + 'static, P: MovementAxisPosition> {
    _marker_position: PhantomData<P>,
    _marker_time: PhantomData<T>,
}

impl<T: DeltaTime + Send + Sync + 'static, P: MovementAxisPosition> LinearTargetReached<T, P> {
    pub fn new() -> Self {
        Self {
            _marker_position: PhantomData,
            _marker_time: PhantomData,
        }
    }
}
