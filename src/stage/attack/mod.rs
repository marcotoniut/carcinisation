pub mod components;
pub mod data;
pub mod spawns;
pub mod systems;

use bevy::prelude::*;

use self::{
    systems::player::*,
    systems::{hovering::*, *},
};

pub struct AttackPlugin;

impl Plugin for AttackPlugin {
    fn build(&self, app: &mut App) {
        app.add_state::<AttackPluginUpdateState>().add_systems(
            Update,
            (
                (check_got_hit, check_health_at_0).chain(),
                despawn_dead_attacks,
                read_enemy_attack_depth_changed,
                miss_on_reached,
                hovering_damage_on_reached,
            )
                .run_if(in_state(AttackPluginUpdateState::Active)),
        );
    }
}

#[derive(States, Debug, Clone, Eq, PartialEq, Hash, Default)]
pub enum AttackPluginUpdateState {
    #[default]
    Inactive,
    Active,
}
