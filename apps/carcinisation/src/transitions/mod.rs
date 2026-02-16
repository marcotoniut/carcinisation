//! Full-screen transition effects (venetian wipes, fades).

pub mod data;
pub mod spiral;

use std::sync::Arc;

use bevy::prelude::Commands;

use self::{
    data::{TransitionRequest, TransitionVenetianData, TransitionVenetianDataState},
    spiral::messages::TransitionVenetianStartupEvent,
};

/// Triggers the configured transition effect.
pub fn trigger_transition(commands: &mut Commands, request: &TransitionRequest) {
    match request {
        TransitionRequest::Venetian => commands.trigger(TransitionVenetianStartupEvent {
            data: Arc::new(TransitionVenetianData::new(
                TransitionVenetianDataState::Closing,
            )),
        }),
    }
}
