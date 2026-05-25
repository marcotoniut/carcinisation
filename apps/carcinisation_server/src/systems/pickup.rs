use crate::systems::NetHealth;
use bevy::prelude::*;
use carcinisation_fps_core::pickup::{PickupRules, apply_health_pickup};
use carcinisation_net::PlayerNetState;
use carcinisation_net::components::{NetPickup, NetPlayer};
use carcinisation_net::protocol::{NetPickupKind, PickupEffect};

/// Marker for the pickup system set.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct PickupSet;

/// Buffers pickup events between `pickup_system` (core logic) and
/// `flush_pickup_events` (replicon broadcast, production only).
///
/// Separating these lets unit tests exercise core logic without wiring
/// up the full replicon plugin stack (states, transport, etc.).
#[derive(Resource, Default)]
pub struct PickupEventBuffer(pub Vec<PickupEffect>);

/// Server-authoritative pickup system.
/// Runs in `FixedUpdate` after movement and combat.
/// - Updates respawn timers for unavailable pickups
/// - Checks for player-pickup overlaps and applies effects
pub fn pickup_system(
    fixed_time: Res<Time<Fixed>>,
    pickup_rules: Res<PickupRules>,
    mut pickup_query: Query<(Entity, &mut NetPickup)>,
    mut player_query: Query<(Entity, &NetPlayer, &mut NetHealth)>,
    mut buffer: ResMut<PickupEventBuffer>,
) {
    let dt = fixed_time.delta_secs();

    // Update respawn timers for unavailable pickups
    for (_, mut pickup) in &mut pickup_query {
        if pickup.available {
            continue;
        }
        if let Some(mut timer) = pickup.respawn_remaining.take() {
            timer -= dt;
            if timer <= 0.0 {
                pickup.available = true;
                pickup.respawn_remaining = None;
            } else {
                pickup.respawn_remaining = Some(timer);
            }
        }
    }

    // Check for player-pickup overlaps
    for (_player_entity, player, mut health) in &mut player_query {
        if !matches!(player.state, PlayerNetState::Alive) || health.current <= 0.0 {
            continue;
        }
        let player_pos = player.position;
        for (_pickup_entity, mut pickup) in &mut pickup_query {
            if !pickup.available {
                continue;
            }
            let pickup_pos = pickup.position;
            let distance_squared = player_pos.distance_squared(pickup_pos);
            if distance_squared <= pickup_rules.radius * pickup_rules.radius {
                // Apply the pickup effect based on kind
                let consumed = match pickup.kind {
                    NetPickupKind::Health => {
                        let new_health = apply_health_pickup(
                            health.current,
                            health.max,
                            pickup_rules.heal_amount,
                        );
                        if (new_health - health.current).abs() < f32::EPSILON {
                            false
                        } else {
                            health.current = new_health;
                            pickup.available = false;
                            if pickup.respawnable {
                                pickup.respawn_remaining = Some(pickup_rules.respawn_time);
                            }
                            buffer.0.push(PickupEffect {
                                player_id: player.player_id,
                                pickup_id: pickup.object_id,
                                kind: pickup.kind,
                                position: pickup.position,
                            });
                            true
                        }
                    }
                    NetPickupKind::Ammo => {
                        warn_once!("Ammo pickup parsed but not yet implemented");
                        false
                    }
                    NetPickupKind::Weapon => {
                        warn_once!("Weapon pickup parsed but not yet implemented");
                        false
                    }
                };
                // Once a pickup is collected by a player, it becomes unavailable and cannot be collected by another player in the same tick.
                if consumed {
                    break;
                }
            }
        }
    }
}

/// Flushes buffered pickup events to all clients via replicon.
/// Only wired in production (not in isolated unit tests).
pub fn flush_pickup_events(mut buffer: ResMut<PickupEventBuffer>, mut commands: Commands) {
    use bevy_replicon::prelude::*;

    let events = std::mem::take(&mut buffer.0);
    for effect in events {
        commands.server_trigger(ToClients {
            mode: SendMode::Broadcast,
            message: effect,
        });
    }
}
