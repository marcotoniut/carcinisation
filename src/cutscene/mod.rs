pub mod bundles;
pub mod cinemachine;
pub mod components;
pub mod data;
pub mod events;
pub mod resources;
pub mod systems;

use crate::core::time::tick_time;

use self::{
    events::{CinematicStartupEvent, CutsceneShutdownEvent},
    resources::CutsceneTime,
    systems::{
        interactions::*,
        layout::*,
        progress::{check_cutscene_elapsed_step, read_step_trigger},
        setup::{initialise_cutscene_animation_spawn_step, on_shutdown, on_startup},
    },
};
use bevy::prelude::*;

pub struct CutscenePlugin;

impl Plugin for CutscenePlugin {
    fn build(&self, app: &mut App) {
        app.add_state::<CutscenePluginUpdateState>()
            .add_event::<CinematicStartupEvent>()
            .add_event::<CutsceneShutdownEvent>()
            .init_resource::<CutsceneTime>()
            .add_systems(PostUpdate, (on_startup, on_shutdown))
            // .add_systems(OnEnter(CutscenePluginUpdateState::Active), spawn_cutscene)
            .add_systems(
                Update,
                (
                    (
                        read_step_trigger,
                        (
                            check_cutscene_elapsed_step,
                            initialise_cutscene_animation_spawn_step,
                        ),
                    )
                        .chain(),
                    play_cutscene,
                    // render_cutscene,
                    press_next,
                    press_esc,
                    tick_time::<CutsceneTime>,
                )
                    .run_if(in_state(CutscenePluginUpdateState::Active)),
            );
    }
}

#[derive(States, Debug, Clone, Eq, PartialEq, Hash, Default)]
pub enum CutscenePluginUpdateState {
    #[default]
    Inactive,
    Active,
}
