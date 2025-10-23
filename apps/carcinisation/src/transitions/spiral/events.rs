use crate::transitions::data::TransitionVenetianData;
use bevy::prelude::*;
use std::sync::Arc;

#[derive(Event, Message)]
pub struct TransitionVenetianStartupEvent {
    pub data: Arc<TransitionVenetianData>,
}

#[derive(Event, Message)]
pub struct TransitionVenetianShutdownEvent;
