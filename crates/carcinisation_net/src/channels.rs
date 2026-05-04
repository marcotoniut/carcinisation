use bevy::prelude::*;
use bevy_replicon::prelude::*;

use crate::protocol::{
    AttackFire, ClientInput, DamageEffect, DeathEffect, HitConfirm, MuzzleFlash, PickupEffect,
};

/// Register reliable (ordered) channels for input and attack events.
pub fn register_reliable_channels(app: &mut App) {
    app.add_client_event::<ClientInput>(Channel::Ordered)
        .add_client_event::<AttackFire>(Channel::Ordered);
}

/// Register unreliable (unordered) channels for visual/effect events.
pub fn register_unreliable_channels(app: &mut App) {
    app.add_server_event::<MuzzleFlash>(Channel::Unordered)
        .add_server_event::<HitConfirm>(Channel::Unordered)
        .add_server_event::<DamageEffect>(Channel::Unordered)
        .add_server_event::<DeathEffect>(Channel::Unordered)
        .add_server_event::<PickupEffect>(Channel::Unordered);
}
