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

    // -----------------------------------------------------------------------
    // apply_health_pickup
    // -----------------------------------------------------------------------

    #[test]
    fn health_pickup_below_max() {
        assert_eq!(apply_health_pickup(30.0, 100.0, 50.0), 80.0);
    }

    #[test]
    fn health_pickup_clamps_to_max() {
        assert_eq!(apply_health_pickup(80.0, 100.0, 50.0), 100.0);
    }

    #[test]
    fn health_pickup_at_max_stays_at_max() {
        assert_eq!(apply_health_pickup(100.0, 100.0, 50.0), 100.0);
    }

    #[test]
    fn health_pickup_zero_amount() {
        assert_eq!(apply_health_pickup(50.0, 100.0, 0.0), 50.0);
    }

    // -----------------------------------------------------------------------
    // apply_ammo_pickup
    // -----------------------------------------------------------------------

    #[test]
    fn ammo_pickup_below_max() {
        assert_eq!(apply_ammo_pickup(10.0, 50.0, 25.0), 35.0);
    }

    #[test]
    fn ammo_pickup_clamps_to_max() {
        assert_eq!(apply_ammo_pickup(40.0, 50.0, 25.0), 50.0);
    }

    // -----------------------------------------------------------------------
    // update_respawn_timer
    // -----------------------------------------------------------------------

    #[test]
    fn respawn_timer_decrements() {
        assert_eq!(update_respawn_timer(Some(5.0), 1.0), Some(4.0));
    }

    #[test]
    fn respawn_timer_expires_at_zero() {
        assert_eq!(update_respawn_timer(Some(1.0), 1.0), None);
    }

    #[test]
    fn respawn_timer_expires_past_zero() {
        assert_eq!(update_respawn_timer(Some(0.5), 1.0), None);
    }

    #[test]
    fn respawn_timer_none_stays_none() {
        assert_eq!(update_respawn_timer(None, 1.0), None);
    }

    // -----------------------------------------------------------------------
    // is_within_radius
    // -----------------------------------------------------------------------

    #[test]
    fn within_radius_inside() {
        assert!(is_within_radius(Vec2::ZERO, Vec2::new(0.3, 0.0), 0.5));
    }

    #[test]
    fn within_radius_exact_boundary() {
        assert!(is_within_radius(Vec2::ZERO, Vec2::new(0.5, 0.0), 0.5));
    }

    #[test]
    fn within_radius_outside() {
        assert!(!is_within_radius(Vec2::ZERO, Vec2::new(0.6, 0.0), 0.5));
    }

    #[test]
    fn within_radius_same_point() {
        assert!(is_within_radius(Vec2::ZERO, Vec2::ZERO, 0.5));
    }
}
