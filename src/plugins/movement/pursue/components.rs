use std::marker::PhantomData;

use bevy::prelude::*;

use crate::{core::time::DeltaTime, plugins::movement::structs::MovementVec2Position};

// TODO Bundle and on added

#[derive(Component, Debug, Clone)]
pub struct PursueTargetPosition<T: DeltaTime + Send + Sync + 'static, P> {
    _marker_time: PhantomData<T>,
    _marker_position: PhantomData<P>,
    pub value: Vec2,
}

impl<T: DeltaTime + Send + Sync + 'static, P: MovementVec2Position> PursueTargetPosition<T, P> {
    pub fn new(value: Vec2) -> Self {
        Self {
            _marker_time: PhantomData,
            _marker_position: PhantomData,
            value,
        }
    }
}

// TODO split into LinearX, Y, Z
#[derive(Component, Debug, Clone)]
pub struct PursueSpeed<T: DeltaTime + Send + Sync + 'static, P: MovementVec2Position> {
    _marker_position: PhantomData<P>,
    _marker_time: PhantomData<T>,
    pub value: Vec2,
}

impl<T: DeltaTime + Send + Sync + 'static, P: MovementVec2Position> PursueSpeed<T, P> {
    pub fn new(value: Vec2) -> Self {
        Self {
            _marker_position: PhantomData,
            _marker_time: PhantomData,
            value,
        }
    }
}

#[derive(Component, Debug, Clone)]
pub struct PursueTargetXReached<T: DeltaTime + Send + Sync + 'static, P: MovementVec2Position> {
    _marker_position: PhantomData<P>,
    _marker_time: PhantomData<T>,
}

impl<T: DeltaTime + Send + Sync + 'static, P: MovementVec2Position> PursueTargetXReached<T, P> {
    pub fn new() -> Self {
        Self {
            _marker_position: PhantomData,
            _marker_time: PhantomData,
        }
    }
}

#[derive(Component, Debug, Clone)]
pub struct PursueTargetYReached<T: DeltaTime + Send + Sync + 'static, P: MovementVec2Position> {
    _marker_position: PhantomData<P>,
    _marker_time: PhantomData<T>,
}

impl<T: DeltaTime + Send + Sync + 'static, P: MovementVec2Position> PursueTargetYReached<T, P> {
    pub fn new() -> Self {
        Self {
            _marker_position: PhantomData,
            _marker_time: PhantomData,
        }
    }
}

#[derive(Component, Debug, Clone)]
pub struct PursueTargetReached<T: DeltaTime + Send + Sync + 'static, P: MovementVec2Position> {
    _marker_position: PhantomData<P>,
    _marker_time: PhantomData<T>,
}

impl<T: DeltaTime + Send + Sync + 'static, P: MovementVec2Position> PursueTargetReached<T, P> {
    pub fn new() -> Self {
        Self {
            _marker_position: PhantomData,
            _marker_time: PhantomData,
        }
    }
}

// TODO LinearBundle (which should also clear up any previous TargetReached that the component may have)
// The other components shouldn't be used on their own
