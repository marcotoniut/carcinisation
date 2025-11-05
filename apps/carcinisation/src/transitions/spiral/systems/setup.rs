use crate::{
    globals::mark_for_despawn_by_query,
    transitions::{
        data::TransitionVenetianData,
        spiral::{
            components::TransitionVenetian,
            events::{TransitionVenetianShutdownEvent, TransitionVenetianStartupEvent},
            TransitionVenetianPluginUpdateState,
        },
    },
};
use bevy::prelude::*;

pub fn on_transition_startup(
    trigger: On<TransitionVenetianStartupEvent>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<TransitionVenetianPluginUpdateState>>,
) {
    next_state.set(TransitionVenetianPluginUpdateState::Active);

    let data = trigger.event().data.as_ref().clone();
    commands.insert_resource::<TransitionVenetianData>(data);

    commands.spawn((TransitionVenetian, Name::new("Transition - Venetian")));
}

pub fn on_transition_shutdown(
    _trigger: On<TransitionVenetianShutdownEvent>,
    mut commands: Commands,
    transition_query: Query<Entity, With<TransitionVenetian>>,
) {
    mark_for_despawn_by_query(&mut commands, &transition_query);
}
