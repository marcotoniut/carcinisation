use bevy::prelude::*;
pub use carcinisation_collision::{Collider, ColliderData, ColliderShape};
use std::time::Duration;

#[derive(Component)]
pub struct Object;

#[derive(Clone, Component, Debug, Default)]
pub struct Flickerer;

// Should hittable specify whether you can hit with Melee, ranged or both?
#[derive(Clone, Component, Debug, Default)]
pub struct Hittable;

// TODO? critical kill
#[derive(Clone, Component, Debug, Default)]
pub struct Dead;

#[derive(Clone, Component, Debug, Reflect)]
#[reflect(Component)]
pub struct BurningCorpse {
    pub started: Duration,
    pub duration: Duration,
    pub seed: u32,
}

#[derive(Clone, Component, Debug, Reflect)]
pub struct Health(pub u32);

#[derive(Clone, Component, Debug, Reflect)]
pub struct HealthOverride(pub u32);
