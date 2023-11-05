use bevy::prelude::*;

#[derive(Clone, Debug)]
pub enum TransitionVenetianDataState {
    Opening,
    Closing,
}

#[derive(Clone, Debug, Resource)]
pub struct TransitionVenetianData {
    pub state: TransitionVenetianDataState,
}
