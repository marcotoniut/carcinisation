//! Cutscene system: data loading, input handling, progression, and playback state.

pub mod components;
pub mod data;
pub mod input;
pub mod messages;
pub mod resources;
mod systems;

use self::{
    input::{CutsceneInput, init_input},
    messages::{CutsceneShutdownEvent, CutsceneStartupEvent},
    resources::CutsceneTimeDomain,
    systems::{
        interactions::check_press_start_input,
        progress::*,
        setup::{on_cutscene_shutdown, on_cutscene_startup},
    },
};
use crate::core::{event::on_trigger_write_event, time::tick_time};
use activable::{Activable, ActivableAppExt};
use bevy::prelude::*;
use bevy_common_assets::ron::RonAssetPlugin;
use cween::linear::{
    LinearTweenPlugin,
    components::{TargetingValueX, TargetingValueY},
};
use data::CutsceneData;
use leafwing_input_manager::plugin::InputManagerPlugin;

/// Registers cutscene resources, input mapping, and playback systems.
#[derive(Activable)]
pub struct CutscenePlugin;

impl Plugin for CutscenePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(RonAssetPlugin::<CutsceneData>::new(&["cs.ron"]))
            .add_plugins(InputManagerPlugin::<CutsceneInput>::default())
            .add_plugins(LinearTweenPlugin::<CutsceneTimeDomain, TargetingValueX>::default())
            .add_plugins(LinearTweenPlugin::<CutsceneTimeDomain, TargetingValueY>::default())
            .init_resource::<Time<CutsceneTimeDomain>>()
            .add_message::<CutsceneStartupEvent>()
            .add_observer(on_cutscene_startup)
            .add_message::<CutsceneShutdownEvent>()
            .add_observer(on_cutscene_shutdown)
            .add_observer(on_trigger_write_event::<CutsceneShutdownEvent>)
            .add_systems(Startup, init_input)
            .add_active_systems_in::<CutscenePlugin, _>(
                FixedUpdate,
                (
                    (
                        read_step_trigger,
                        (
                            check_cutscene_elapsed,
                            process_cutscene_animations_spawn,
                            process_cutscene_images_spawn,
                            process_cutscene_music_spawn,
                            process_cutscene_music_despawn,
                        ),
                    )
                        .chain(),
                    tick_time::<Fixed, CutsceneTimeDomain>,
                ),
            )
            .add_active_systems_in::<CutscenePlugin, _>(PostUpdate, check_press_start_input);
    }
}
