pub mod extra;

use bevy::prelude::*;
use derive_new::new;
use std::marker::PhantomData;

use crate::{
    core::time::DeltaTime,
    plugins::movement::structs::{Constructor, Magnitude, MovementDirection},
};

// TODO From
#[derive(new, Clone, Component, Debug, Reflect)]
pub struct TargetingPositionX(pub f32);

#[derive(new, Clone, Component, Debug, Reflect)]
pub struct TargetingPositionY(pub f32);

#[derive(new, Clone, Component, Debug, Reflect)]
pub struct TargetingPositionZ(pub f32);

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

        impl Constructor<f32> for $type {
            fn new(x: f32) -> Self {
                Self(x)
            }
        }
    };
}

impl_magnitude!(TargetingPositionX);
impl_magnitude!(TargetingPositionY);
impl_magnitude!(TargetingPositionZ);

#[derive(new, Component, Debug, Clone)]
pub struct LinearDirection<T: DeltaTime + Send + Sync + 'static, P> {
    #[new(default)]
    _marker_time: PhantomData<T>,
    #[new(default)]
    _marker_position: PhantomData<P>,
    pub value: MovementDirection,
}

impl<T: DeltaTime + Send + Sync + 'static, P> LinearDirection<T, P> {
    pub fn from_delta(value: f32) -> Self {
        Self::new(if value > 0.0 {
            MovementDirection::Positive
        } else {
            MovementDirection::Negative
        })
    }
}

#[derive(new, Component, Debug, Clone)]
pub struct LinearTargetPosition<T: DeltaTime + Send + Sync + 'static, P> {
    #[new(default)]
    _marker_time: PhantomData<T>,
    #[new(default)]
    _marker_position: PhantomData<P>,
    pub value: f32,
}

// TODO split into LinearX, Y, Z
#[derive(new, Component, Debug, Clone)]
pub struct LinearSpeed<T: DeltaTime + Send + Sync + 'static, P: Magnitude> {
    #[new(default)]
    _marker_position: PhantomData<P>,
    #[new(default)]
    _marker_time: PhantomData<T>,
    pub value: f32,
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

#[derive(new, Component, Debug, Clone)]
pub struct LinearAcceleration<T: DeltaTime + Send + Sync + 'static, P: Magnitude> {
    #[new(default)]
    _marker_position: PhantomData<P>,
    #[new(default)]
    _marker_time: PhantomData<T>,
    pub value: f32,
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

#[derive(new, Component, Debug, Clone)]
pub struct LinearTargetReached<T: DeltaTime + Send + Sync + 'static, P: Magnitude> {
    #[new(default)]
    _marker_position: PhantomData<P>,
    #[new(default)]
    _marker_time: PhantomData<T>,
}

#[derive(Bundle, Clone, Debug)]
pub struct LinearMovementBundle<
    T: DeltaTime + Send + Sync + 'static,
    P: Constructor<f32> + Component + Magnitude,
> {
    pub direction: LinearDirection<T, P>,
    pub position: P,
    pub speed: LinearSpeed<T, P>,
    pub target_position: LinearTargetPosition<T, P>,
    // TODO check if Option<LinearTargetReached> = None will auto-remove
}

impl<T: DeltaTime + Send + Sync + 'static, P: Constructor<f32> + Component + Magnitude>
    LinearMovementBundle<T, P>
{
    pub fn new(current_position: f32, target_position: f32, speed: f32) -> Self {
        Self {
            direction: LinearDirection::<T, P>::from_delta(target_position - current_position),
            position: P::new(current_position),
            speed: LinearSpeed::<T, P>::new(speed),
            target_position: LinearTargetPosition::<T, P>::new(target_position),
        }
    }
}

#[derive(Bundle, Clone, Debug)]
pub struct LinearMovementAcceleratedBundle<
    T: DeltaTime + Send + Sync + 'static,
    P: Constructor<f32> + Component + Magnitude,
> {
    pub acceleration: LinearAcceleration<T, P>,
    pub direction: LinearDirection<T, P>,
    pub position: P,
    pub speed: LinearSpeed<T, P>,
    pub target_position: LinearTargetPosition<T, P>,
}

impl<T: DeltaTime + Send + Sync + 'static, P: Constructor<f32> + Component + Magnitude>
    LinearMovementAcceleratedBundle<T, P>
{
    pub fn new(current_position: f32, target_position: f32, speed: f32, acceleration: f32) -> Self {
        Self {
            direction: LinearDirection::<T, P>::from_delta(target_position - current_position),
            position: P::new(current_position),
            speed: LinearSpeed::<T, P>::new(speed),
            target_position: LinearTargetPosition::<T, P>::new(target_position),
            acceleration: LinearAcceleration::<T, P>::new(acceleration),
        }
    }
}

#[derive(Bundle)]
pub struct LinearPositionRemovalBundle<
    T: DeltaTime + Send + Sync + 'static,
    P: Component + Magnitude,
> {
    pub position: P,
    pub acceleration: LinearAcceleration<T, P>,
    pub direction: LinearDirection<T, P>,
    pub speed: LinearSpeed<T, P>,
    pub target_position: LinearTargetPosition<T, P>,
    pub target_reached: LinearTargetReached<T, P>,
}
