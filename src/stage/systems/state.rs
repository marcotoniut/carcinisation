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
    attack_state_o: Option<ResMut<NextState<AttackPluginUpdateState>>>,
    destructible_state_o: Option<ResMut<NextState<DestructiblePluginUpdateState>>>,
    enemy_state_o: Option<ResMut<NextState<EnemyPluginUpdateState>>>,
    player_state_o: Option<ResMut<NextState<PlayerPluginUpdateState>>>,
    stage_ui_state_o: Option<ResMut<NextState<StageUiPluginUpdateState>>>,
    // pickup_state_o: Option<ResMut<State<PickupPluginUpdateState>>>,
) {
    if let Some(mut state) = attack_state_o {
        state.set(AttackPluginUpdateState::Active);
    }
    if let Some(mut state) = destructible_state_o {
        state.set(DestructiblePluginUpdateState::Active);
    }
    if let Some(mut state) = enemy_state_o {
        state.set(EnemyPluginUpdateState::Active);
    }
    if let Some(mut state) = player_state_o {
        state.set(PlayerPluginUpdateState::Active);
    }
    if let Some(mut state) = stage_ui_state_o {
        state.set(StageUiPluginUpdateState::Active);
    }
}

pub fn on_inactive(
    attack_state_o: Option<ResMut<NextState<AttackPluginUpdateState>>>,
    destructible_state_o: Option<ResMut<NextState<DestructiblePluginUpdateState>>>,
    enemy_state_o: Option<ResMut<NextState<EnemyPluginUpdateState>>>,
    player_state_o: Option<ResMut<NextState<PlayerPluginUpdateState>>>,
    stage_ui_state_o: Option<ResMut<NextState<StageUiPluginUpdateState>>>,
    // pickup_state_o: Option<ResMut<State<PickupPluginUpdateState>>>,
) {
    if let Some(mut state) = attack_state_o {
        state.set(AttackPluginUpdateState::Inactive);
    }
    if let Some(mut state) = destructible_state_o {
        state.set(DestructiblePluginUpdateState::Inactive);
    }
    if let Some(mut state) = enemy_state_o {
        state.set(EnemyPluginUpdateState::Inactive);
    }
    if let Some(mut state) = player_state_o {
        state.set(PlayerPluginUpdateState::Inactive);
    }
    if let Some(mut state) = stage_ui_state_o {
        state.set(StageUiPluginUpdateState::Inactive);
    }
}
