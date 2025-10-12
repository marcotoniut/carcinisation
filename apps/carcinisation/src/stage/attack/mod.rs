//! Handles enemy attack entities, their lifecycle, and collision outcomes.

pub mod components;
pub mod data;
pub mod spawns;
mod systems;

use self::{
    systems::player::*,
    systems::{hovering::*, *},
};
use bevy::prelude::*;

/// Schedules attack-related systems and gating state.
pub struct AttackPlugin;

impl Plugin for AttackPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<AttackPluginUpdateState>().add_systems(
            Update,
            // Only advance attack behaviour when the plugin is explicitly active.
            (
                (check_got_hit, check_health_at_0).chain(),
                despawn_dead_attacks,
                on_enemy_attack_depth_changed,
                miss_on_reached,
                hovering_damage_on_reached,
            )
                .run_if(in_state(AttackPluginUpdateState::Active)),
        );
    }
}

#[derive(States, Debug, Clone, Eq, PartialEq, Hash, Default)]
/// Controls when attack systems tick (mirrors the stage lifecycle).
pub enum AttackPluginUpdateState {
    #[default]
    Inactive,
    Active,
}
