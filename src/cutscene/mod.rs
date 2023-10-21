pub mod bundles;
pub mod cinemachine;
pub mod components;
pub mod data;
pub mod events;
pub mod resources;
pub mod systems;

use crate::core::time::tick_time;

use self::{
    events::CinematicStartupEvent,
    resources::CutsceneTime,
    systems::{interactions::*, layout::*, setup::on_startup},
};
use bevy::prelude::*;

pub struct CutscenePlugin;

impl Plugin for CutscenePlugin {
    fn build(&self, app: &mut App) {
        app.add_state::<CutscenePluginUpdateState>()
            .add_event::<CinematicStartupEvent>()
            .init_resource::<CutsceneTime>()
            .add_systems(PostUpdate, on_startup)
            // .add_systems(OnEnter(CutscenePluginUpdateState::Active), spawn_cutscene)
            .add_systems(
                OnExit(CutscenePluginUpdateState::Inactive),
                mark_cutscene_for_despawn,
            )
            .add_systems(
                Update,
                (
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
