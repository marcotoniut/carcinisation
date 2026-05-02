//! Full-screen transition effects (venetian wipes, fades).

pub mod data;
pub mod spiral;

use std::sync::Arc;

use bevy::prelude::Commands;

use self::spiral::messages::TransitionVenetianStartupEvent;
use carcinisation_cutscene::data::TransitionRequest;

/// Triggers the configured transition effect.
pub fn trigger_transition(commands: &mut Commands, request: &TransitionRequest) {
    match request {
        TransitionRequest::Venetian => commands.trigger(TransitionVenetianStartupEvent {
            data: Arc::new(data::TransitionVenetianData::new(
                data::TransitionVenetianDataState::Closing,
            )),
        }),
    }
}
