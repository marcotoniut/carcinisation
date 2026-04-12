use super::{ENEMY_ATTACK_ATLAS_PATH, EnemyHoveringAttackType};
use crate::{
    pixel::PxAnimationBundle,
    stage::components::interactive::{Collider, ColliderData},
};
use bevy::prelude::*;
use carapace::prelude::{PxAnimationDuration, PxAtlasSprite, PxSpriteAtlasAsset};

/// Creates the atlas-based hovering attack visual bundle.
///
/// All sprites are authored at depth 1. Runtime depth scaling is handled by
/// `AuthoredDepths::single(Depth::One)` + `apply_depth_fallback_scale`.
pub fn make_hovering_attack_atlas_bundle(
    asset_server: &AssetServer,
    attack_type: &EnemyHoveringAttackType,
) -> (PxAtlasSprite, PxAnimationBundle, ColliderData) {
    let atlas_handle: Handle<PxSpriteAtlasAsset> = asset_server.load(ENEMY_ATTACK_ATLAS_PATH);
    let region_id = attack_type.hovering_region_id();

    let animations = attack_type.get_animations();
    // Use the depth-1 hovering animation data for timing/behavior.
    let hovering_anim = animations.hovering_animation_data();

    (
        PxAtlasSprite::new(atlas_handle, region_id),
        PxAnimationBundle::from_parts(
            hovering_anim.direction,
            PxAnimationDuration::millis_per_animation(hovering_anim.speed),
            hovering_anim.finish_behavior,
            hovering_anim.frame_transition,
        ),
        ColliderData::from_one(Collider::new_circle(attack_type.base_collider_radius())),
    )
}

/// Creates the atlas-based hit animation bundle for a separate hit-effect entity.
pub fn make_hit_atlas_bundle(
    asset_server: &AssetServer,
    attack_type: &EnemyHoveringAttackType,
) -> (PxAtlasSprite, PxAnimationBundle) {
    let atlas_handle: Handle<PxSpriteAtlasAsset> = asset_server.load(ENEMY_ATTACK_ATLAS_PATH);
    let region_id = attack_type.hit_region_id();

    let animations = attack_type.get_animations();
    let hit_anim = animations.hit_animation_data();

    (
        PxAtlasSprite::new(atlas_handle, region_id),
        PxAnimationBundle::from_parts(
            hit_anim.direction,
            PxAnimationDuration::millis_per_animation(hit_anim.speed),
            hit_anim.finish_behavior,
            hit_anim.frame_transition,
        ),
    )
}
