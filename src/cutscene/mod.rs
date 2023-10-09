pub mod bundles;
pub mod components;
pub mod resources;
pub mod systems;

use bevy::prelude::*;

use self::systems::{interactions::*, layout::*};

pub struct CutscenePlugin;

impl Plugin for CutscenePlugin {
    fn build(&self, app: &mut App) {
        app.add_state::<CutscenePluginUpdateState>()
            .add_systems(OnEnter(CutscenePluginUpdateState::Active), spawn_cutscene)
            .add_systems(
                OnExit(CutscenePluginUpdateState::Inactive),
                mark_cutscene_for_despawn,
            )
            .add_systems(
                Update,
                (play_cutscene, press_next, press_esc)
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
