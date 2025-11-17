pub mod extra;

use crate::structs::{Constructor, Magnitude, MovementDirection};
use bevy::{ecs::hierarchy::ChildOf, prelude::*};
use derive_more::From;
use derive_new::new;
use std::marker::PhantomData;

#[derive(new, Clone, Component, Debug, From, Reflect)]
pub struct TargetingPositionX(pub f32);

#[derive(new, Clone, Component, Debug, From, Reflect)]
pub struct TargetingPositionY(pub f32);

#[derive(new, Clone, Component, Debug, From, Reflect)]
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
pub struct LinearDirection<D: Send + Sync + 'static, P> {
    #[new(default)]
    _marker_time: PhantomData<D>,
    #[new(default)]
    _marker_position: PhantomData<P>,
    pub value: MovementDirection,
}

impl<D: Send + Sync + 'static, P> LinearDirection<D, P> {
    pub fn from_delta(value: f32) -> Self {
        Self::new(if value > 0.0 {
            MovementDirection::Positive
        } else {
            MovementDirection::Negative
        })
    }
}

#[derive(new, Clone, Component, Debug)]
pub struct LinearTargetPosition<D: Send + Sync + 'static, P> {
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
pub struct LinearTargetReached<D: Send + Sync + 'static, P: Magnitude> {
    #[new(default)]
    _marker_position: PhantomData<P>,
    #[new(default)]
    _marker_time: PhantomData<D>,
}

#[derive(Bundle, Clone, Debug)]
pub struct LinearMovementBundle<
    D: Send + Sync + 'static,
    P: Constructor<f32> + Component + Magnitude,
> {
    pub direction: LinearDirection<D, P>,
    pub position: P,
    pub speed: LinearSpeed<D, P>,
    pub target_position: LinearTargetPosition<D, P>,
    // TODO check if Option<LinearTargetReached> = None will auto-remove
}

impl<D: Send + Sync + 'static, P: Constructor<f32> + Component + Magnitude>
    LinearMovementBundle<D, P>
{
    pub fn new(current_position: f32, target_position: f32, speed: f32) -> Self {
        Self {
            direction: LinearDirection::<D, P>::from_delta(target_position - current_position),
            position: P::new(current_position),
            speed: LinearSpeed::<D, P>::new(speed),
            target_position: LinearTargetPosition::<D, P>::new(target_position),
        }
    }
}

#[derive(Bundle, Clone, Debug)]
pub struct LinearMovementAcceleratedBundle<
    D: Send + Sync + 'static,
    P: Constructor<f32> + Component + Magnitude,
> {
    pub acceleration: LinearAcceleration<D, P>,
    pub direction: LinearDirection<D, P>,
    pub position: P,
    pub speed: LinearSpeed<D, P>,
    pub target_position: LinearTargetPosition<D, P>,
}

impl<D: Send + Sync + 'static, P: Constructor<f32> + Component + Magnitude>
    LinearMovementAcceleratedBundle<D, P>
{
    pub fn new(current_position: f32, target_position: f32, speed: f32, acceleration: f32) -> Self {
        Self {
            direction: LinearDirection::<D, P>::from_delta(target_position - current_position),
            position: P::new(current_position),
            speed: LinearSpeed::<D, P>::new(speed),
            target_position: LinearTargetPosition::<D, P>::new(target_position),
            acceleration: LinearAcceleration::<D, P>::new(acceleration),
        }
    }
}

/// Marker component indicating this entity is a movement child that affects its parent's position.
/// Movement children express movement intent and are aggregated by the parent.
#[derive(Component, Debug, Clone, Copy, Reflect)]
pub struct MovementChild;

/// Bundle for spawning a movement child entity.
/// Movement children own the linear movement components and affect the parent entity's position.
#[derive(Bundle, Clone, Debug)]
pub struct MovementChildBundle<
    D: Send + Sync + 'static,
    P: Constructor<f32> + Component + Magnitude,
> {
    pub child_of: ChildOf,
    pub movement_child: MovementChild,
    pub linear_movement: LinearMovementBundle<D, P>,
}

impl<D: Send + Sync + 'static, P: Constructor<f32> + Component + Magnitude>
    MovementChildBundle<D, P>
{
    pub fn new(parent: Entity, current_position: f32, target_position: f32, speed: f32) -> Self {
        Self {
            child_of: ChildOf(parent),
            movement_child: MovementChild,
            linear_movement: LinearMovementBundle::<D, P>::new(
                current_position,
                target_position,
                speed,
            ),
        }
    }
}

/// Bundle for spawning an accelerated movement child entity.
#[derive(Bundle, Clone, Debug)]
pub struct MovementChildAcceleratedBundle<
    D: Send + Sync + 'static,
    P: Constructor<f32> + Component + Magnitude,
> {
    pub child_of: ChildOf,
    pub movement_child: MovementChild,
    pub linear_movement: LinearMovementAcceleratedBundle<D, P>,
}

impl<D: Send + Sync + 'static, P: Constructor<f32> + Component + Magnitude>
    MovementChildAcceleratedBundle<D, P>
{
    pub fn new(
        parent: Entity,
        current_position: f32,
        target_position: f32,
        speed: f32,
        acceleration: f32,
    ) -> Self {
        Self {
            child_of: ChildOf(parent),
            movement_child: MovementChild,
            linear_movement: LinearMovementAcceleratedBundle::<D, P>::new(
                current_position,
                target_position,
                speed,
                acceleration,
            ),
        }
    }
}

#[derive(Bundle)]
pub struct LinearPositionRemovalBundle<D: Send + Sync + 'static, P: Component + Magnitude> {
    pub position: P,
    pub acceleration: LinearAcceleration<D, P>,
    pub direction: LinearDirection<D, P>,
    pub speed: LinearSpeed<D, P>,
    pub target_position: LinearTargetPosition<D, P>,
    pub target_reached: LinearTargetReached<D, P>,
}
