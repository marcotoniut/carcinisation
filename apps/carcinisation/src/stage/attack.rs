//! Handles enemy attack entities, their lifecycle, and collision outcomes.

pub mod components;
pub mod data;
pub mod spawns;
mod systems;

#[cfg(debug_assertions)]
use self::systems::sync_enemy_attack_debug_positions;
use self::{
    data::{
        blood_shot::BloodShotConfig, boulder_throw::BoulderThrowConfig,
        spider_shot::SpiderShotConfig,
    },
    spawns::blood_shot::arm_pending_blood_shot_motion,
    systems::player::check_got_hit,
    systems::{
        check_health_at_0, despawn_dead_attacks, hovering::hovering_damage_on_reached,
        miss_on_reached, on_enemy_attack_depth_changed, update_attached_attack_positions,
    },
};
use super::enemy::composed::update_composed_enemy_visuals;
use activable::{Activable, ActivableAppExt};
use bevy::prelude::*;

/// Schedules attack-related systems and gating state.
#[derive(Activable)]
pub struct AttackPlugin;

impl Plugin for AttackPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(BloodShotConfig::load());
        app.insert_resource(BoulderThrowConfig::load());
        app.insert_resource(SpiderShotConfig::load());
        app.add_active_systems::<AttackPlugin, _>(
            // Only advance attack behaviour when the plugin is explicitly active.
            (
                (check_got_hit, check_health_at_0).chain(),
                #[cfg(debug_assertions)]
                sync_enemy_attack_debug_positions,
                (
                    update_attached_attack_positions,
                    arm_pending_blood_shot_motion,
                )
                    .chain()
                    .after(update_composed_enemy_visuals),
                despawn_dead_attacks,
                on_enemy_attack_depth_changed,
                miss_on_reached,
                hovering_damage_on_reached,
            ),
        );
    }
}
