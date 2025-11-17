use crate::structs::MovementVec2Position;
use bevy::prelude::*;
use derive_new::new;
use std::marker::PhantomData;

// TODO Bundle and on added

#[derive(new, Component, Debug, Clone)]
pub struct PursueTargetPosition<D: Send + Sync + 'static, P> {
    #[new(default)]
    _marker_time: PhantomData<D>,
    #[new(default)]
    _marker_position: PhantomData<P>,
    pub value: Vec2,
}

// TODO split into LinearX, Y, Z
#[derive(new, Component, Debug, Clone)]
pub struct PursueSpeed<D: Send + Sync + 'static, P: MovementVec2Position> {
    #[new(default)]
    _marker_position: PhantomData<P>,
    #[new(default)]
    _marker_time: PhantomData<D>,
    pub value: Vec2,
}

#[derive(new, Component, Debug, Clone)]
pub struct PursueTargetXReached<D: Send + Sync + 'static, P: MovementVec2Position> {
    #[new(default)]
    _marker_position: PhantomData<P>,
    #[new(default)]
    _marker_time: PhantomData<D>,
}

#[derive(new, Component, Debug, Clone)]
pub struct PursueTargetYReached<D: Send + Sync + 'static, P: MovementVec2Position> {
    #[new(default)]
    _marker_position: PhantomData<P>,
    #[new(default)]
    _marker_time: PhantomData<D>,
}

#[derive(new, Component, Debug, Clone)]
pub struct PursueTargetReached<D: Send + Sync + 'static, P: MovementVec2Position> {
    #[new(default)]
    _marker_position: PhantomData<P>,
    #[new(default)]
    _marker_time: PhantomData<D>,
}

// TODO LinearBundle (which should also clear up any previous TargetReached that the component may have)
// The other components shouldn't be used on their own
