use bevy::prelude::*;

use crate::stage::ui::hud::HudPluginUpdateState;

pub fn on_active(mut hud_state: ResMut<NextState<HudPluginUpdateState>>) {
    hud_state.set(HudPluginUpdateState::Active);
}

pub fn on_inactive(mut hud_state: ResMut<NextState<HudPluginUpdateState>>) {
    hud_state.set(HudPluginUpdateState::Inactive);
}
