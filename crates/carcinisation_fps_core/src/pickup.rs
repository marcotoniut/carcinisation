use bevy::prelude::Resource;
use bevy_math::Vec2;

/// Types of pickups that can appear in the world.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PickupKind {
    Health,
    Ammo,
    Weapon,
}

/// Rules governing pickup behavior (heal/ammo amounts, respawn time, collection radius).
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize, Resource)]
#[serde(rename = "PickupRules")]
pub struct PickupRules {
    /// Amount of health restored by a health pickup.
    pub heal_amount: f32,
    /// Amount of ammo restored by an ammo pickup.
    pub ammo_amount: f32,
    /// Time in seconds before a pickup respawns after being collected.
    pub respawn_time: f32,
    /// Radius within which a player can collect the pickup.
    pub radius: f32,
}

impl PickupRules {
    #[must_use]
    pub fn load() -> Self {
        carcinisation_core::ron_config!("assets/config/fp/pickup.ron")
    }
}

impl Default for PickupRules {
    fn default() -> Self {
        Self {
            heal_amount: 50.0,
            ammo_amount: 50.0,
            respawn_time: 30.0,
            radius: 0.5,
        }
    }
}

/// Apply a health pickup, clamping to max health.
#[must_use]
pub fn apply_health_pickup(current: f32, max: f32, amount: f32) -> f32 {
    let new = current + amount;
    if new > max { max } else { new }
}

/// Apply an ammo pickup, clamping to max ammo.
#[must_use]
pub fn apply_ammo_pickup(current: f32, max: f32, amount: f32) -> f32 {
    let new = current + amount;
    if new > max { max } else { new }
}

/// Update a respawn timer by subtracting dt.
/// Returns None when the timer has expired (i.e., the pickup should become available).
#[must_use]
pub fn update_respawn_timer(timer: Option<f32>, dt: f32) -> Option<f32> {
    let mut timer = timer?;
    timer -= dt;
    if timer <= 0.0 { None } else { Some(timer) }
}

/// Check if two positions are within pickup collection radius.
#[must_use]
pub fn is_within_radius(a: Vec2, b: Vec2, radius: f32) -> bool {
    a.distance_squared(b) <= radius * radius
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pickup_rules_load() {
        let _ = PickupRules::load();
    }
}
