//! Cutscene system: data loading, input handling, progression, and playback state.

pub mod components;
pub mod data;
pub mod events;
pub mod input;
pub mod resources;
mod systems;

use self::{
    events::{CutsceneShutdownTrigger, CutsceneStartupTrigger},
    input::{init_input, CutsceneInput},
    resources::CutsceneTime,
    systems::{
        interactions::check_press_start_input,
        progress::*,
        setup::{on_cutscene_shutdown, on_cutscene_startup},
    },
};
use crate::{
    core::{event::on_trigger_write_event, time::tick_time},
    plugins::movement::linear::{
        components::{TargetingPositionX, TargetingPositionY},
        LinearMovementPlugin,
    },
};
use bevy::prelude::*;
use bevy_common_assets::ron::RonAssetPlugin;
use data::CutsceneData;
use leafwing_input_manager::plugin::InputManagerPlugin;

/// Registers cutscene resources, input mapping, and playback systems.
pub struct CutscenePlugin;

impl Plugin for CutscenePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(RonAssetPlugin::<CutsceneData>::new(&["cs.ron"]))
            .add_plugins(InputManagerPlugin::<CutsceneInput>::default())
            .add_plugins(LinearMovementPlugin::<CutsceneTime, TargetingPositionX>::default())
            .add_plugins(LinearMovementPlugin::<CutsceneTime, TargetingPositionY>::default())
            .init_state::<CutscenePluginUpdateState>()
            .init_resource::<CutsceneTime>()
            .add_event::<CutsceneStartupTrigger>()
            .add_observer(on_cutscene_startup)
            .add_event::<CutsceneShutdownTrigger>()
            .add_observer(on_cutscene_shutdown)
            .add_observer(on_trigger_write_event::<CutsceneShutdownTrigger>)
            // .add_systems(OnEnter(CutscenePluginUpdateState::Active), spawn_cutscene)
            .add_systems(Startup, init_input)
            .add_systems(
                Update,
                // Core playback loop: reads steps, spawns assets, ticks timers.
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
                    // render_cutscene,
                    tick_time::<CutsceneTime>,
                )
                    .run_if(in_state(CutscenePluginUpdateState::Active)),
            )
            .add_systems(
                PostUpdate,
                (check_press_start_input).run_if(in_state(CutscenePluginUpdateState::Active)),
            );
    }
}

#[derive(States, Debug, Clone, Eq, PartialEq, Hash, Default)]
/// Toggle for enabling/disabling cutscene playback systems.
pub enum CutscenePluginUpdateState {
    #[default]
    Inactive,
    Active,
}
