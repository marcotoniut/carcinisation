use bevy::prelude::*;
use bevy_replicon::prelude::*;

use crate::protocol::{
    ClientIntent, DamageEffect, DeathEffect, EnemyAttackVisual, FlameActive, FlameCharMark,
    HitConfirm, InputAck, MonitorAck, MuzzleFlash, PickupEffect, PlayerIdAssigned,
};

/// Register reliable (ordered) channels for input and identity.
pub fn register_reliable_channels(app: &mut App) {
    app.add_client_event::<ClientIntent>(Channel::Ordered)
        .add_server_event::<PlayerIdAssigned>(Channel::Ordered)
        .add_server_event::<MonitorAck>(Channel::Ordered)
        .add_server_event::<InputAck>(Channel::Ordered);
}

/// Register unreliable (unordered) channels for visual/effect events.
///
/// Authoritative state is replicated via components (`NetEnemyState`, `PlayerNetState`).
/// These events are cosmetic feedback; dropped packets are tolerated.
pub fn register_unreliable_channels(app: &mut App) {
    app.add_server_event::<MuzzleFlash>(Channel::Unordered)
        .add_server_event::<HitConfirm>(Channel::Unordered)
        .add_server_event::<DamageEffect>(Channel::Unordered)
        .add_server_event::<DeathEffect>(Channel::Unordered)
        .add_server_event::<FlameActive>(Channel::Unordered)
        .add_server_event::<EnemyAttackVisual>(Channel::Unordered)
        .add_server_event::<FlameCharMark>(Channel::Unordered)
        .add_server_event::<PickupEffect>(Channel::Unordered);
}
