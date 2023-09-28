use std::marker::PhantomData;

use bevy::prelude::*;

use crate::{core::time::DeltaTime, plugins::movement::structs::Magnitude};

#[derive(Component, Debug, Clone)]
pub struct XAxisPosition(pub f32);

#[derive(Component, Debug, Clone)]
pub struct YAxisPosition(pub f32);

#[derive(Component, Debug, Clone)]
pub struct ZAxisPosition(pub f32);

macro_rules! impl_magnitude {
    ($type:ty) => {
        impl Magnitude for $type {
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
    };
}

impl_magnitude!(XAxisPosition);
impl_magnitude!(YAxisPosition);
impl_magnitude!(ZAxisPosition);

// TODO Bundle and on added

#[derive(Component, Debug, Clone)]
pub struct LinearTargetPosition<T: DeltaTime + Send + Sync + 'static, P> {
    _marker_time: PhantomData<T>,
    _marker_position: PhantomData<P>,
    pub value: f32,
}

impl<T: DeltaTime + Send + Sync + 'static, P: Magnitude> LinearTargetPosition<T, P> {
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
pub struct LinearSpeed<T: DeltaTime + Send + Sync + 'static, P: Magnitude> {
    _marker_position: PhantomData<P>,
    _marker_time: PhantomData<T>,
    pub value: f32,
}

impl<T: DeltaTime + Send + Sync + 'static, P: Magnitude> LinearSpeed<T, P> {
    pub fn new(value: f32) -> Self {
        Self {
            _marker_position: PhantomData,
            _marker_time: PhantomData,
            value,
        }
    }
}

impl<T: DeltaTime + Send + Sync + 'static, P: Magnitude> Magnitude for LinearSpeed<T, P> {
    fn get(&self) -> f32 {
        self.value
    }

    fn set(&mut self, value: f32) {
        self.value = value;
    }

    fn add(&mut self, value: f32) {
        self.value += value;
    }
}

#[derive(Component, Debug, Clone)]
pub struct LinearAcceleration<T: DeltaTime + Send + Sync + 'static, P: Magnitude> {
    _marker_position: PhantomData<P>,
    _marker_time: PhantomData<T>,
    pub value: f32,
}

impl<T: DeltaTime + Send + Sync + 'static, P: Magnitude> LinearAcceleration<T, P> {
    pub fn new(value: f32) -> Self {
        Self {
            _marker_position: PhantomData,
            _marker_time: PhantomData,
            value,
        }
    }
}

impl<T: DeltaTime + Send + Sync + 'static, P: Magnitude> Magnitude for LinearAcceleration<T, P> {
    fn get(&self) -> f32 {
        self.value
    }

    fn set(&mut self, value: f32) {
        self.value = value;
    }

    fn add(&mut self, value: f32) {
        self.value += value;
    }
}

#[derive(Component, Debug, Clone)]
pub struct LinearTargetReached<T: DeltaTime + Send + Sync + 'static, P: Magnitude> {
    _marker_position: PhantomData<P>,
    _marker_time: PhantomData<T>,
}

impl<T: DeltaTime + Send + Sync + 'static, P: Magnitude> LinearTargetReached<T, P> {
    pub fn new() -> Self {
        Self {
            _marker_position: PhantomData,
            _marker_time: PhantomData,
        }
    }
}
