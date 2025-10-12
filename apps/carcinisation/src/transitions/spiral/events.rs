use crate::transitions::data::TransitionVenetianData;
use bevy::prelude::*;
use std::sync::Arc;

#[derive(Event)]
pub struct TransitionVenetianStartupEvent {
    pub data: Arc<TransitionVenetianData>,
}

#[derive(Event)]
pub struct TransitionVenetianShutdownEvent;
