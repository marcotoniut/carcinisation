pub mod components;
pub mod data;
pub mod events;
pub mod input;
pub mod resources;
pub mod systems;

use self::{
    events::{CutsceneShutdownEvent, CutsceneStartupEvent},
    input::{init_input, CutsceneInput},
    resources::CutsceneTime,
    systems::{
        interactions::check_press_start_input,
        progress::*,
        setup::{on_shutdown, on_startup},
    },
};
use crate::{
    core::time::tick_time,
    plugins::movement::linear::{
        components::{TargetingPositionX, TargetingPositionY},
        LinearMovementPlugin,
    },
};
use bevy::prelude::*;
use bevy_common_assets::ron::RonAssetPlugin;
use data::CutsceneData;
use leafwing_input_manager::plugin::InputManagerPlugin;

pub struct CutscenePlugin;

impl Plugin for CutscenePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<CutscenePluginUpdateState>()
            .add_event::<CutsceneStartupEvent>()
            .add_event::<CutsceneShutdownEvent>()
            .init_resource::<CutsceneTime>()
            // Assets
            .add_plugins(RonAssetPlugin::<CutsceneData>::new(&["cs.ron"]))
            .add_plugins(InputManagerPlugin::<CutsceneInput>::default())
            .add_plugins(LinearMovementPlugin::<CutsceneTime, TargetingPositionX>::default())
            .add_plugins(LinearMovementPlugin::<CutsceneTime, TargetingPositionY>::default())
            .add_systems(Startup, init_input)
            .add_systems(PreUpdate, (on_startup, on_shutdown))
            // .add_systems(OnEnter(CutscenePluginUpdateState::Active), spawn_cutscene)
            .add_systems(
                Update,
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
pub enum CutscenePluginUpdateState {
    #[default]
    Inactive,
    Active,
}
