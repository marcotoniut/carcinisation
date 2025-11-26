//! Startup/shutdown handling for cutscenes.

use crate::cutscene::CutscenePlugin;
use crate::{
    cutscene::{
        components::{Cinematic, CutsceneEntity},
        data::CutsceneData,
        messages::{CutsceneShutdownEvent, CutsceneStartupEvent},
        resources::CutsceneProgress,
    },
    debug::plugin::{debug_print_shutdown, debug_print_startup},
    globals::mark_for_despawn_by_query,
    letterbox::{components::LetterboxEntity, messages::LetterboxMoveEvent},
};
use activable::{activate, deactivate};
use bevy::prelude::*;

const DEBUG_MODULE: &str = "Cutscene";

/// @trigger Boots a cutscene, loading data and enabling systems.
pub fn on_cutscene_startup(trigger: On<CutsceneStartupEvent>, mut commands: Commands) {
    #[cfg(debug_assertions)]
    debug_print_startup(DEBUG_MODULE);

    let e = trigger.event();
    activate::<CutscenePlugin>(&mut commands);

    commands.insert_resource::<CutsceneData>(e.data.as_ref().clone());
    commands.insert_resource::<CutsceneProgress>(CutsceneProgress { index: 0 });

    commands.spawn((Cinematic, Name::new("Cutscene")));
}

/// @trigger Cleans up cutscene entities and disables the plugin.
pub fn on_cutscene_shutdown(
    _trigger: On<CutsceneShutdownEvent>,
    mut commands: Commands,
    cinematic_query: Query<Entity, With<Cinematic>>,
    cutscene_entity_query: Query<Entity, With<CutsceneEntity>>,
    letterbox_query: Query<Entity, With<LetterboxEntity>>,
) {
    #[cfg(debug_assertions)]
    debug_print_shutdown(DEBUG_MODULE);

    deactivate::<CutscenePlugin>(&mut commands);
    commands.trigger(LetterboxMoveEvent::hide());

    mark_for_despawn_by_query(&mut commands, &cutscene_entity_query);
    mark_for_despawn_by_query(&mut commands, &cinematic_query);
    mark_for_despawn_by_query(&mut commands, &letterbox_query);
}
