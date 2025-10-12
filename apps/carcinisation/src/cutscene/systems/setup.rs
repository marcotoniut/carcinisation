//! Startup/shutdown handling for cutscenes.

use crate::{
    cutscene::{
        components::{Cinematic, CutsceneEntity},
        data::CutsceneData,
        events::{CutsceneShutdownTrigger, CutsceneStartupTrigger},
        resources::CutsceneProgress,
        CutscenePluginUpdateState,
    },
    debug::plugin::{debug_print_shutdown, debug_print_startup},
    globals::mark_for_despawn_by_query,
    letterbox::events::LetterboxMoveTrigger,
};
use bevy::prelude::*;

const DEBUG_MODULE: &str = "Cutscene";

/// @trigger Boots a cutscene, loading data and enabling systems.
pub fn on_cutscene_startup(
    trigger: Trigger<CutsceneStartupTrigger>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<CutscenePluginUpdateState>>,
) {
    #[cfg(debug_assertions)]
    debug_print_startup(DEBUG_MODULE);

    let e = trigger.event();
    next_state.set(CutscenePluginUpdateState::Active);

    commands.insert_resource::<CutsceneData>(e.data.as_ref().clone());
    commands.insert_resource::<CutsceneProgress>(CutsceneProgress { index: 0 });

    commands.spawn((Cinematic, Name::new("Cutscene")));
}

/// @trigger Cleans up cutscene entities and disables the plugin.
pub fn on_cutscene_shutdown(
    _trigger: Trigger<CutsceneShutdownTrigger>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<CutscenePluginUpdateState>>,
    cinematic_query: Query<Entity, With<Cinematic>>,
    cutscene_entity_query: Query<Entity, With<CutsceneEntity>>,
) {
    #[cfg(debug_assertions)]
    debug_print_shutdown(DEBUG_MODULE);

    next_state.set(CutscenePluginUpdateState::Inactive);
    commands.trigger(LetterboxMoveTrigger::hide());

    mark_for_despawn_by_query(&mut commands, &cutscene_entity_query);
    mark_for_despawn_by_query(&mut commands, &cinematic_query);
}
