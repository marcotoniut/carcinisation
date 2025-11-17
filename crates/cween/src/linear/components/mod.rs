pub mod extra;

use crate::structs::{Constructor, Magnitude, TweenDirection};
use bevy::{ecs::hierarchy::ChildOf, prelude::*};
use derive_more::From;
use derive_new::new;
use std::marker::PhantomData;

#[derive(new, Clone, Component, Debug, From, Reflect)]
pub struct TargetingValueX(pub f32);

#[derive(new, Clone, Component, Debug, From, Reflect)]
pub struct TargetingValueY(pub f32);

#[derive(new, Clone, Component, Debug, From, Reflect)]
pub struct TargetingValueZ(pub f32);

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

impl_magnitude!(TargetingValueX);
impl_magnitude!(TargetingValueY);
impl_magnitude!(TargetingValueZ);

#[derive(new, Component, Debug, Clone)]
pub struct LinearTweenDirection<D: Send + Sync + 'static, P> {
    #[new(default)]
    _marker_time: PhantomData<D>,
    #[new(default)]
    _marker_position: PhantomData<P>,
    pub value: TweenDirection,
}

impl<D: Send + Sync + 'static, P> LinearTweenDirection<D, P> {
    pub fn from_delta(value: f32) -> Self {
        Self::new(if value > 0.0 {
            TweenDirection::Positive
        } else {
            TweenDirection::Negative
        })
    }
}

#[derive(new, Clone, Component, Debug)]
pub struct LinearTargetValue<D: Send + Sync + 'static, P> {
    #[new(default)]
    _marker_time: PhantomData<D>,
    #[new(default)]
    _marker_position: PhantomData<P>,
    pub value: f32,
}

#[derive(new, Clone, Component, Debug)]
pub struct LinearSpeed<D: Send + Sync + 'static, P: Magnitude> {
    #[new(default)]
    _marker_position: PhantomData<P>,
    #[new(default)]
    _marker_time: PhantomData<D>,
    pub value: f32,
}

impl<D: Send + Sync + 'static, P: Magnitude> Magnitude for LinearSpeed<D, P> {
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
pub struct LinearAcceleration<D: Send + Sync + 'static, P: Magnitude> {
    #[new(default)]
    _marker_position: PhantomData<P>,
    #[new(default)]
    _marker_time: PhantomData<D>,
    pub value: f32,
}

impl<D: Send + Sync + 'static, P: Magnitude> Magnitude for LinearAcceleration<D, P> {
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
pub struct LinearValueReached<D: Send + Sync + 'static, P: Magnitude> {
    #[new(default)]
    _marker_position: PhantomData<P>,
    #[new(default)]
    _marker_time: PhantomData<D>,
}

#[derive(Bundle, Clone, Debug)]
pub struct LinearTweenBundle<D: Send + Sync + 'static, P: Constructor<f32> + Component + Magnitude>
{
    pub direction: LinearTweenDirection<D, P>,
    pub value: P,
    pub speed: LinearSpeed<D, P>,
    pub target_value: LinearTargetValue<D, P>,
    // TODO check if Option<LinearValueReached> = None will auto-remove
}

impl<D: Send + Sync + 'static, P: Constructor<f32> + Component + Magnitude>
    LinearTweenBundle<D, P>
{
    pub fn new(current_value: f32, target_value: f32, speed: f32) -> Self {
        Self {
            direction: LinearTweenDirection::<D, P>::from_delta(target_value - current_value),
            value: P::new(current_value),
            speed: LinearSpeed::<D, P>::new(speed),
            target_value: LinearTargetValue::<D, P>::new(target_value),
        }
    }
}

#[derive(Bundle, Clone, Debug)]
pub struct LinearTweenAcceleratedBundle<
    D: Send + Sync + 'static,
    P: Constructor<f32> + Component + Magnitude,
> {
    pub acceleration: LinearAcceleration<D, P>,
    pub direction: LinearTweenDirection<D, P>,
    pub value: P,
    pub speed: LinearSpeed<D, P>,
    pub target_value: LinearTargetValue<D, P>,
}

impl<D: Send + Sync + 'static, P: Constructor<f32> + Component + Magnitude>
    LinearTweenAcceleratedBundle<D, P>
{
    pub fn new(current_value: f32, target_value: f32, speed: f32, acceleration: f32) -> Self {
        Self {
            direction: LinearTweenDirection::<D, P>::from_delta(target_value - current_value),
            value: P::new(current_value),
            speed: LinearSpeed::<D, P>::new(speed),
            target_value: LinearTargetValue::<D, P>::new(target_value),
            acceleration: LinearAcceleration::<D, P>::new(acceleration),
        }
    }
}

/// Marker component indicating this entity is a tween child that affects its parent's value.
/// Tween children express tween intent and are aggregated by the parent.
#[derive(Component, Debug, Clone, Copy, Reflect)]
pub struct TweenChild;

/// Bundle for spawning a tween child entity.
/// Tween children own the linear tween components and affect the parent entity's value.
#[derive(Bundle, Clone, Debug)]
pub struct TweenChildBundle<D: Send + Sync + 'static, P: Constructor<f32> + Component + Magnitude> {
    pub child_of: ChildOf,
    pub tween_child: TweenChild,
    pub linear_tween: LinearTweenBundle<D, P>,
}

impl<D: Send + Sync + 'static, P: Constructor<f32> + Component + Magnitude> TweenChildBundle<D, P> {
    pub fn new(parent: Entity, current_value: f32, target_value: f32, speed: f32) -> Self {
        Self {
            child_of: ChildOf(parent),
            tween_child: TweenChild,
            linear_tween: LinearTweenBundle::<D, P>::new(current_value, target_value, speed),
        }
    }
}

/// Bundle for spawning an accelerated tween child entity.
#[derive(Bundle, Clone, Debug)]
pub struct TweenChildAcceleratedBundle<
    D: Send + Sync + 'static,
    P: Constructor<f32> + Component + Magnitude,
> {
    pub child_of: ChildOf,
    pub tween_child: TweenChild,
    pub linear_tween: LinearTweenAcceleratedBundle<D, P>,
}

impl<D: Send + Sync + 'static, P: Constructor<f32> + Component + Magnitude>
    TweenChildAcceleratedBundle<D, P>
{
    pub fn new(
        parent: Entity,
        current_value: f32,
        target_value: f32,
        speed: f32,
        acceleration: f32,
    ) -> Self {
        Self {
            child_of: ChildOf(parent),
            tween_child: TweenChild,
            linear_tween: LinearTweenAcceleratedBundle::<D, P>::new(
                current_value,
                target_value,
                speed,
                acceleration,
            ),
        }
    }
}

#[derive(Bundle)]
pub struct LinearValueRemovalBundle<D: Send + Sync + 'static, P: Component + Magnitude> {
    pub value: P,
    pub acceleration: LinearAcceleration<D, P>,
    pub direction: LinearTweenDirection<D, P>,
    pub speed: LinearSpeed<D, P>,
    pub target_value: LinearTargetValue<D, P>,
    pub target_reached: LinearValueReached<D, P>,
}
