pub mod components;
pub mod systems;

use bevy::prelude::*;

use self::{
    systems::player::*,
    systems::{blood_shot::*, *},
};
use super::{GameState, StageState};
use crate::AppState;

pub struct AttackPlugin;

impl Plugin for AttackPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                (check_got_hit, check_health_at_0).chain(),
                despawn_dead_attacks,
                read_enemy_attack_depth_changed,
                miss_on_reached,
                blood_attack_damage_on_reached,
            )
                .run_if(in_state(StageState::Running))
                .run_if(in_state(GameState::Running))
                .run_if(in_state(AppState::Game)),
        );
    }
}
