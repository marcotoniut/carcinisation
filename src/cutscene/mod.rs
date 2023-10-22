pub mod bundles;
pub mod components;
pub mod data;
pub mod events;
pub mod resources;
pub mod systems;

use self::{
    events::{CutsceneShutdownEvent, CutsceneStartupEvent},
    resources::CutsceneTime,
    systems::{
        progress::*,
        setup::{on_shutdown, on_startup},
    },
};
use crate::core::time::tick_time;
use bevy::prelude::*;

pub struct CutscenePlugin;

impl Plugin for CutscenePlugin {
    fn build(&self, app: &mut App) {
        app.add_state::<CutscenePluginUpdateState>()
            .add_event::<CutsceneStartupEvent>()
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
                            check_cutscene_elapsed,
                            process_cutscene_animations_spawn,
                            process_cutscene_despawn,
                            process_cutscene_music_spawn,
                            process_cutscene_music_despawn,
                        ),
                    )
                        .chain(),
                    // render_cutscene,
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
