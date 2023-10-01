use std::marker::PhantomData;

use bevy::prelude::*;

use crate::{
    core::time::DeltaTime,
    plugins::movement::structs::{Magnitude, MovementDirection},
};

#[derive(Component, Debug, Clone)]
pub struct XAxisPosition(pub f32);

impl XAxisPosition {
    pub fn new(value: f32) -> Self {
        Self(value)
    }
}

#[derive(Component, Debug, Clone)]
pub struct YAxisPosition(pub f32);

impl YAxisPosition {
    pub fn new(value: f32) -> Self {
        Self(value)
    }
}

#[derive(Component, Debug, Clone)]
pub struct ZAxisPosition(pub f32);

impl ZAxisPosition {
    pub fn new(value: f32) -> Self {
        Self(value)
    }
}

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
pub struct LinearDirection<T: DeltaTime + Send + Sync + 'static, P> {
    _marker_time: PhantomData<T>,
    _marker_position: PhantomData<P>,
    pub value: MovementDirection,
}

impl<T: DeltaTime + Send + Sync + 'static, P> LinearDirection<T, P> {
    pub fn new(value: MovementDirection) -> Self {
        Self {
            _marker_time: PhantomData,
            _marker_position: PhantomData,
            value,
        }
    }
    pub fn from_delta(value: f32) -> Self {
        Self::new(if value > 0.0 {
            MovementDirection::Positive
        } else {
            MovementDirection::Negative
        })
    }
}

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

#[derive(Bundle, Clone, Debug)]
pub struct LinearMovementBundle<T: DeltaTime + Send + Sync + 'static, P: Magnitude> {
    pub direction: LinearDirection<T, P>,
    pub speed: LinearSpeed<T, P>,
    pub target_position: LinearTargetPosition<T, P>,
}

impl<T: DeltaTime + Send + Sync + 'static, P: Magnitude> LinearMovementBundle<T, P> {
    pub fn new(current_position: f32, target_position: f32, speed: f32) -> Self {
        Self {
            direction: LinearDirection::<T, P>::from_delta(target_position - current_position),
            speed: LinearSpeed::<T, P>::new(speed),
            target_position: LinearTargetPosition::<T, P>::new(target_position),
        }
    }
}

#[derive(Bundle, Clone, Debug)]
pub struct LinearMovementAcceleratedBundle<T: DeltaTime + Send + Sync + 'static, P: Magnitude> {
    pub acceleration: LinearAcceleration<T, P>,
    pub direction: LinearDirection<T, P>,
    pub speed: LinearSpeed<T, P>,
    pub target_position: LinearTargetPosition<T, P>,
}

impl<T: DeltaTime + Send + Sync + 'static, P: Magnitude> LinearMovementAcceleratedBundle<T, P> {
    pub fn new(current_position: f32, target_position: f32, speed: f32, acceleration: f32) -> Self {
        Self {
            direction: LinearDirection::<T, P>::from_delta(target_position - current_position),
            speed: LinearSpeed::<T, P>::new(speed),
            target_position: LinearTargetPosition::<T, P>::new(target_position),
            acceleration: LinearAcceleration::<T, P>::new(acceleration),
        }
    }
}

#[derive(Bundle)]
pub struct LinearPositionRemovalBundle<T: DeltaTime + Send + Sync + 'static, P: Magnitude> {
    pub acceleration: LinearAcceleration<T, P>,
    pub direction: LinearDirection<T, P>,
    pub speed: LinearSpeed<T, P>,
    pub target_position: LinearTargetPosition<T, P>,
    pub target_reached: LinearTargetReached<T, P>,
}
