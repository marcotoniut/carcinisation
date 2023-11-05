use crate::{
    cutscene::{
        components::{Cinematic, CutsceneEntity},
        data::CutsceneData,
        events::{CutsceneShutdownEvent, CutsceneStartupEvent},
        resources::CutsceneProgress,
        CutscenePluginUpdateState,
    },
    globals::mark_for_despawn_by_component_query,
};
use bevy::prelude::*;
use seldom_pixel::prelude::PxSubPosition;

pub fn on_startup(
    mut commands: Commands,
    mut event_reader: EventReader<CutsceneStartupEvent>,
    mut cutscene_state_next_state: ResMut<NextState<CutscenePluginUpdateState>>,
) {
    for e in event_reader.iter() {
        cutscene_state_next_state.set(CutscenePluginUpdateState::Active);

        let data = e.data.as_ref();

        commands.insert_resource::<CutsceneData>(data.clone());
        commands.insert_resource::<CutsceneProgress>(CutsceneProgress { index: 0 });

        commands.spawn((
            Cinematic,
            Name::new("Cutscene"),
            PxSubPosition(Vec2::new(50., 30.)),
        ));
    }
}

pub fn on_shutdown(
    mut commands: Commands,
    mut event_reader: EventReader<CutsceneShutdownEvent>,
    cinematic_query: Query<Entity, With<Cinematic>>,
    cutscene_entity_query: Query<Entity, With<CutsceneEntity>>,
) {
    for _ in event_reader.iter() {
        mark_for_despawn_by_component_query(&mut commands, &cutscene_entity_query);
        mark_for_despawn_by_component_query(&mut commands, &cinematic_query);
    }
}
