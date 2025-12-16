//! Contains the pure "compiler" logic to transform raw config data into
//! game-ready runtime data. This is used by the asset loaders and the build tool.

use crate::{
    config::{AttackConfig, FinishBehavior},
    runtime::{AttackTuning, HoveringAttackAnimations},
};
use carcinisation_core::{
    data::AnimationData,
    globals::PATH_SPRITES_ATTACKS,
    stage::components::{
        interactive::{Collider, ColliderData},
        placement::Depth,
    },
};
use seldom_pixel::prelude::PxAnimationFinishBehavior;
use std::collections::HashMap;

impl From<FinishBehavior> for PxAnimationFinishBehavior {
    fn from(value: FinishBehavior) -> Self {
        match value {
            FinishBehavior::Mark => PxAnimationFinishBehavior::Mark,
            FinishBehavior::Loop => PxAnimationFinishBehavior::Loop,
            FinishBehavior::Despawn => PxAnimationFinishBehavior::Despawn,
        }
    }
}

/// Compiles a raw `AttackConfig` into a game-ready `AttackTuning`.
///
/// This function is pure and can be used anywhere (asset processing, tests, build tools).
pub fn compile(config: &AttackConfig) -> AttackTuning {
    let hovering = build_hovering_map(config);
    let hit = build_hit_map(config);

    AttackTuning {
        depth_speed: config.depth_speed,
        line_speed: config.line_speed,
        damage: config.damage,
        randomness: config.randomness,
        animations: HoveringAttackAnimations { hovering, hit },
    }
}

fn build_hovering_map(config: &AttackConfig) -> HashMap<Depth, AnimationData> {
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

fn build_hit_map(config: &AttackConfig) -> HashMap<Depth, AnimationData> {
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
