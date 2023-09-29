pub mod bundles;
pub mod components;
pub mod resources;
pub mod systems;

use bevy::prelude::*;

use self::systems::{interactions::*, layout::*};
use crate::AppState;

pub struct CutscenePlugin;

impl Plugin for CutscenePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Cutscene), spawn_cutscene)
            .add_systems(OnExit(AppState::Cutscene), mark_cutscene_for_despawn)
            .add_systems(
                Update,
                (play_cutscene, press_next, press_esc).run_if(in_state(AppState::Cutscene)),
            );
    }
}
