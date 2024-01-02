use crate::{core::time::DeltaTime, plugins::movement::structs::MovementVec2Position};
use bevy::prelude::*;
use derive_new::new;
use std::marker::PhantomData;

// TODO Bundle and on added

#[derive(new, Component, Debug, Clone)]
pub struct PursueTargetPosition<T: DeltaTime + Send + Sync + 'static, P> {
    #[new(default)]
    _marker_time: PhantomData<T>,
    #[new(default)]
    _marker_position: PhantomData<P>,
    pub value: Vec2,
}

// TODO split into LinearX, Y, Z
#[derive(new, Component, Debug, Clone)]
pub struct PursueSpeed<T: DeltaTime + Send + Sync + 'static, P: MovementVec2Position> {
    #[new(default)]
    _marker_position: PhantomData<P>,
    #[new(default)]
    _marker_time: PhantomData<T>,
    pub value: Vec2,
}

#[derive(new, Component, Debug, Clone)]
pub struct PursueTargetXReached<T: DeltaTime + Send + Sync + 'static, P: MovementVec2Position> {
    #[new(default)]
    _marker_position: PhantomData<P>,
    #[new(default)]
    _marker_time: PhantomData<T>,
}

#[derive(new, Component, Debug, Clone)]
pub struct PursueTargetYReached<T: DeltaTime + Send + Sync + 'static, P: MovementVec2Position> {
    #[new(default)]
    _marker_position: PhantomData<P>,
    #[new(default)]
    _marker_time: PhantomData<T>,
}

#[derive(new, Component, Debug, Clone)]
pub struct PursueTargetReached<T: DeltaTime + Send + Sync + 'static, P: MovementVec2Position> {
    #[new(default)]
    _marker_position: PhantomData<P>,
    #[new(default)]
    _marker_time: PhantomData<T>,
}

// TODO LinearBundle (which should also clear up any previous TargetReached that the component may have)
// The other components shouldn't be used on their own
