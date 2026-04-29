use std::time::Duration;

use bevy::prelude::*;

#[derive(Component, Debug)]
pub struct InvertFilter;

#[derive(Clone, Component, Debug, Reflect)]
pub struct InflictsDamage(pub u32);

#[derive(Clone, Component, Debug, Reflect)]
pub struct DamageFlicker {
    pub phase_start: Duration,
    pub count: u8,
}
