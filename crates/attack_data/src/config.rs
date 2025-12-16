//! Data structures that map directly to the RON configuration file format.
//! These are for validation and deserialization, not for direct use in gameplay systems.

use bevy::prelude::*;
use carcinisation_core::stage::components::placement::Depth;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct AttackConfig {
    pub attack_id: String,
    pub depth_speed: f32,
    pub line_speed: f32,
    pub damage: u32,
    pub randomness: f32,
    pub sprite_prefix: String,
    #[serde(default = "default_hovering_fragment")]
    pub hovering_fragment: String,
    #[serde(default = "default_hit_fragment")]
    pub hit_fragment: String,
    pub hit_depth: Depth,
    pub hovering: ChannelConfig,
    pub hit: ChannelConfig,
    pub depths: Vec<DepthConfig>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChannelConfig {
    pub frames: usize,
    pub speed: u64,
    #[serde(default)]
    pub finish_behavior: FinishBehavior,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DepthConfig {
    pub depth: Depth,
    pub collider_radius: f32,
    #[serde(default)]
    pub sprite_variant: Option<String>,
    #[serde(default)]
    pub frames: Option<usize>,
    #[serde(default)]
    pub speed: Option<u64>,
    #[serde(default)]
    pub finish_behavior: Option<FinishBehavior>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinishBehavior {
    Mark,
    #[default]
    Loop,
    Despawn,
}

fn default_hovering_fragment() -> String {
    "hovering".to_string()
}

fn default_hit_fragment() -> String {
    "hit".to_string()
}
