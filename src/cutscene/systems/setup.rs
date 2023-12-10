use crate::{
    cutscene::{
        components::{Cinematic, CutsceneEntity},
        data::CutsceneData,
        events::{CutsceneShutdownEvent, CutsceneStartupEvent},
        resources::CutsceneProgress,
        CutscenePluginUpdateState,
    },
    globals::mark_for_despawn_by_component_query,
    letterbox::events::LetterboxMoveEvent,
};
use bevy::prelude::*;

pub fn on_startup(
    mut commands: Commands,
    mut event_reader: EventReader<CutsceneStartupEvent>,
    mut next_state: ResMut<NextState<CutscenePluginUpdateState>>,
) {
    for e in event_reader.iter() {
        next_state.set(CutscenePluginUpdateState::Active);

        let data = e.data.as_ref();

        commands.insert_resource::<CutsceneData>(data.clone());
        commands.insert_resource::<CutsceneProgress>(CutsceneProgress { index: 0 });

        commands.spawn((Cinematic, Name::new("Cutscene")));
    }
}

pub fn on_shutdown(
    mut commands: Commands,
    mut event_reader: EventReader<CutsceneShutdownEvent>,
    mut next_state: ResMut<NextState<CutscenePluginUpdateState>>,
    cinematic_query: Query<Entity, With<Cinematic>>,
    cutscene_entity_query: Query<Entity, With<CutsceneEntity>>,
    mut letterbox_move_event_writer: EventWriter<LetterboxMoveEvent>,
) {
    for _ in event_reader.iter() {
        next_state.set(CutscenePluginUpdateState::Inactive);
        letterbox_move_event_writer.send(LetterboxMoveEvent::hide());

        mark_for_despawn_by_component_query(&mut commands, &cutscene_entity_query);
        mark_for_despawn_by_component_query(&mut commands, &cinematic_query);
    }
}
