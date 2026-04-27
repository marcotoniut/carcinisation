//! Startup/shutdown handling for cutscenes.

use crate::cutscene::CutscenePlugin;
#[cfg(debug_assertions)]
use crate::debug::plugin::{debug_print_shutdown, debug_print_startup};
use crate::{
    cutscene::{
        components::{Cinematic, CutsceneEntity},
        data::CutsceneData,
        messages::{CutsceneShutdownEvent, CutsceneStartupEvent},
        resources::CutsceneProgress,
    },
    globals::mark_for_despawn_by_query,
    letterbox::messages::LetterboxMoveEvent,
};
use activable::{activate, deactivate};
use bevy::prelude::*;

const DEBUG_MODULE: &str = "Cutscene";

/// @trigger Boots a cutscene, loading data and enabling systems.
///
/// When `DevFlags::skip_cutscenes` is set, immediately triggers shutdown
/// instead of playing the cutscene.
pub fn on_cutscene_startup(
    trigger: On<CutsceneStartupEvent>,
    mut commands: Commands,
    dev_flags: Res<crate::resources::DevFlags>,
) {
    #[cfg(debug_assertions)]
    debug_print_startup(DEBUG_MODULE);

    let e = trigger.event();

    if dev_flags.skip_cutscenes && e.data.respect_skip_cutscenes {
        info!("CARCINISATION_SKIP_CUTSCENES: auto-skipping cutscene");
        // Activate and immediately shut down so the game progression
        // system sees the plugin cycle and advances to the next step.
        activate::<CutscenePlugin>(&mut commands);
        commands.trigger(CutsceneShutdownEvent);
        return;
    }
    activate::<CutscenePlugin>(&mut commands);

    // Reset the time domain so keyframe evaluations start from zero.
    commands.insert_resource(Time::<super::super::resources::CutsceneTimeDomain>::default());

    commands.insert_resource::<CutsceneData>(e.data.as_ref().clone());
    commands.insert_resource::<CutsceneProgress>(CutsceneProgress { index: 0 });

    commands.spawn((Cinematic, Name::new("Cutscene")));
}

/// @trigger Cleans up cutscene entities and disables the plugin.
///
/// Letterbox entities are NOT despawned here — they are owned by
/// `LetterboxPlugin` and persist across cutscene boundaries.  The
/// `hide()` command moves them off-screen; their lifecycle is managed
/// by the letterbox plugin's own activation/deactivation.
pub fn on_cutscene_shutdown(
    _trigger: On<CutsceneShutdownEvent>,
    mut commands: Commands,
    cinematic_query: Query<Entity, With<Cinematic>>,
    cutscene_entity_query: Query<Entity, With<CutsceneEntity>>,
) {
    #[cfg(debug_assertions)]
    debug_print_shutdown(DEBUG_MODULE);

    deactivate::<CutscenePlugin>(&mut commands);
    commands.trigger(LetterboxMoveEvent::hide());

    commands.remove_resource::<CutsceneData>();
    commands.remove_resource::<CutsceneProgress>();

    mark_for_despawn_by_query(&mut commands, &cutscene_entity_query);
    mark_for_despawn_by_query(&mut commands, &cinematic_query);
}
