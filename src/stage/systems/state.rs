use bevy::prelude::*;

use crate::stage::{
    attack::AttackPluginUpdateState, destructible::DestructiblePluginUpdateState,
    enemy::EnemyPluginUpdateState, player::PlayerPluginUpdateState, ui::StageUiPluginUpdateState,
};

// pub fn on_state_change(state: State) -> Box<dyn Fn(i32) -> i32> {
//     Box::new(
//         move |mut player_state: ResMut<NextState<PlayerPluginUpdateState>>,
//               mut destructible_state: ResMut<NextState<DestructiblePluginUpdateState>>,
//               mut enemy_state: ResMut<NextState<EnemyPluginUpdateState>>| {
//             player_state.set(PlayerPluginUpdateState::Active);
//             destructible_state.set(DestructiblePluginUpdateState::Active);
//             enemy_state.set(EnemyPluginUpdateState::Active);
//         },
//     )
// }

pub fn on_active(
    mut attack_state: ResMut<NextState<AttackPluginUpdateState>>,
    mut destructible_state: ResMut<NextState<DestructiblePluginUpdateState>>,
    mut enemy_state: ResMut<NextState<EnemyPluginUpdateState>>,
    mut player_state: ResMut<NextState<PlayerPluginUpdateState>>,
    mut stage_ui_state: ResMut<NextState<StageUiPluginUpdateState>>,
    // mut pickup_state: ResMut<State<PickupPluginUpdateState>>,
) {
    attack_state.set(AttackPluginUpdateState::Active);
    destructible_state.set(DestructiblePluginUpdateState::Active);
    enemy_state.set(EnemyPluginUpdateState::Active);
    player_state.set(PlayerPluginUpdateState::Active);
    stage_ui_state.set(StageUiPluginUpdateState::Active);
}

pub fn on_inactive(
    mut attack_state: ResMut<NextState<AttackPluginUpdateState>>,
    mut destructible_state: ResMut<NextState<DestructiblePluginUpdateState>>,
    mut enemy_state: ResMut<NextState<EnemyPluginUpdateState>>,
    mut player_state: ResMut<NextState<PlayerPluginUpdateState>>,
    mut stage_ui_state: ResMut<NextState<StageUiPluginUpdateState>>,
    // mut pickup_state: ResMut<State<PickupPluginUpdateState>>,
) {
    attack_state.set(AttackPluginUpdateState::Inactive);
    destructible_state.set(DestructiblePluginUpdateState::Inactive);
    enemy_state.set(EnemyPluginUpdateState::Inactive);
    player_state.set(PlayerPluginUpdateState::Inactive);
    stage_ui_state.set(StageUiPluginUpdateState::Inactive);
}
