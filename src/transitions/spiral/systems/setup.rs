use crate::{
    globals::mark_for_despawn_by_component_query,
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

pub fn on_startup(
    mut commands: Commands,
    mut event_reader: EventReader<TransitionVenetianStartupEvent>,
    mut next_state: ResMut<NextState<TransitionVenetianPluginUpdateState>>,
) {
    for e in event_reader.read() {
        next_state.set(TransitionVenetianPluginUpdateState::Active);

        let data = e.data.as_ref();

        commands.insert_resource::<TransitionVenetianData>(data.clone());
        // commands.insert_resource::<TransitionVenetianProgress>(TransitionVenetianProgress { index: 0 });

        commands.spawn((TransitionVenetian, Name::new("Transition - Venetian")));
    }
}

pub fn on_shutdown(
    mut commands: Commands,
    mut event_reader: EventReader<TransitionVenetianShutdownEvent>,
    transition_query: Query<Entity, With<TransitionVenetian>>,
) {
    for _ in event_reader.read() {
        mark_for_despawn_by_component_query(&mut commands, &transition_query);
    }
}
