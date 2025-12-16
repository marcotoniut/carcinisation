use super::HoveringAttackAnimations;
use crate::{
    data::AnimationData,
    globals::PATH_SPRITES_ATTACKS,
    stage::components::{
        interactive::{Collider, ColliderData},
        placement::Depth,
    },
};
use lazy_static::lazy_static;
use seldom_pixel::prelude::PxAnimationFinishBehavior;
use serde::Deserialize;
use std::collections::HashMap;

const EMBEDDED_CONFIG: &str = include_str!("../../../../../../assets/attacks/blood_shoot.ron");

/// Aggregated tuning values sourced from the embedded RON config.
pub struct BloodShootTuning {
    pub depth_speed: f32,
    pub line_speed: f32,
    pub damage: u32,
    pub randomness: f32,
    pub animations: HoveringAttackAnimations,
}

#[derive(Debug, Deserialize)]
struct BloodShootConfig {
    depth_speed: f32,
    line_speed: f32,
    damage: u32,
    randomness: f32,
    sprite_prefix: String,
    #[serde(default = "default_hovering_fragment")]
    hovering_fragment: String,
    #[serde(default = "default_hit_fragment")]
    hit_fragment: String,
    hit_depth: Depth,
    hovering: ChannelConfig,
    hit: ChannelConfig,
    depths: Vec<DepthConfig>,
}

#[derive(Debug, Deserialize)]
struct ChannelConfig {
    frames: usize,
    speed: u64,
    #[serde(default)]
    finish_behavior: FinishBehavior,
}

#[derive(Debug, Deserialize)]
struct DepthConfig {
    depth: Depth,
    collider_radius: f32,
    #[serde(default)]
    sprite_variant: Option<String>,
    #[serde(default)]
    frames: Option<usize>,
    #[serde(default)]
    speed: Option<u64>,
    #[serde(default)]
    finish_behavior: Option<FinishBehavior>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
enum FinishBehavior {
    Mark,
    #[default]
    Loop,
    Despawn,
}

impl From<FinishBehavior> for PxAnimationFinishBehavior {
    fn from(value: FinishBehavior) -> Self {
        match value {
            FinishBehavior::Mark => PxAnimationFinishBehavior::Mark,
            FinishBehavior::Loop => PxAnimationFinishBehavior::Loop,
            FinishBehavior::Despawn => PxAnimationFinishBehavior::Despawn,
        }
    }
}

fn default_hovering_fragment() -> String {
    "hovering".to_string()
}

fn default_hit_fragment() -> String {
    "hit".to_string()
}

lazy_static! {
    static ref BLOOD_SHOOT_TUNING: BloodShootTuning = {
        let config: BloodShootConfig =
            ron::from_str(EMBEDDED_CONFIG).expect("Embedded blood_shoot.ron should be valid.");
        BloodShootTuning::from_config(config)
    };
}

impl BloodShootTuning {
    pub fn config() -> &'static Self {
        &BLOOD_SHOOT_TUNING
    }

    fn from_config(config: BloodShootConfig) -> Self {
        let hovering = build_hovering_map(&config);
        let hit = build_hit_map(&config);

        Self {
            depth_speed: config.depth_speed,
            line_speed: config.line_speed,
            damage: config.damage,
            randomness: config.randomness,
            animations: HoveringAttackAnimations { hovering, hit },
        }
    }
}

fn build_hovering_map(config: &BloodShootConfig) -> HashMap<Depth, AnimationData> {
    let mut hovering = HashMap::new();

    for depth_config in &config.depths {
        let sprite_fragment = depth_config
            .sprite_variant
            .as_deref()
            .unwrap_or(&config.hovering_fragment);
        let finish_behavior = depth_config
            .finish_behavior
            .unwrap_or(config.hovering.finish_behavior);

        let animation = AnimationData {
            collider_data: ColliderData::from_one(Collider::new_circle(
                depth_config.collider_radius,
            )),
            finish_behavior: finish_behavior.into(),
            frames: depth_config.frames.unwrap_or(config.hovering.frames),
            speed: depth_config.speed.unwrap_or(config.hovering.speed),
            sprite_path: sprite_path(&config.sprite_prefix, sprite_fragment, depth_config.depth),
            ..Default::default()
        };

        hovering.insert(depth_config.depth, animation);
    }

    hovering
}

fn build_hit_map(config: &BloodShootConfig) -> HashMap<Depth, AnimationData> {
    let mut hit = HashMap::new();
    let animation = AnimationData {
        finish_behavior: config.hit.finish_behavior.into(),
        frames: config.hit.frames,
        speed: config.hit.speed,
        sprite_path: sprite_path(
            &config.sprite_prefix,
            &config.hit_fragment,
            config.hit_depth,
        ),
        ..Default::default()
    };

    hit.insert(config.hit_depth, animation);
    hit
}

fn sprite_path(prefix: &str, fragment: &str, depth: Depth) -> String {
    format!(
        "{}{}_{}_{}.px_sprite.png",
        PATH_SPRITES_ATTACKS,
        prefix,
        fragment,
        depth.to_i8()
    )
}
