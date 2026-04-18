use super::EnemyHoveringAttackType;
use crate::{
    pixel::PxAnimationBundle,
    stage::components::interactive::{Collider, ColliderData},
};
use bevy::prelude::*;
use carapace::prelude::{PxAtlasSprite, PxFrameTransition, PxSpriteAtlasAsset};

/// Region name constants matching aseprite tag names.
pub const REGION_HOVER: &str = "hover";
pub const REGION_DESTROY: &str = "destroy";
pub const REGION_HIT: &str = "hit";

/// Creates the atlas-based hovering attack visual bundle.
///
/// Animation parameters are read from the atlas metadata when available,
/// falling back to sensible defaults if the atlas is not yet loaded.
#[must_use]
pub fn make_hovering_attack_atlas_bundle(
    asset_server: &AssetServer,
    atlas_assets: &Assets<PxSpriteAtlasAsset>,
    attack_type: &EnemyHoveringAttackType,
) -> (PxAtlasSprite, PxAnimationBundle, ColliderData) {
    let atlas_handle: Handle<PxSpriteAtlasAsset> = asset_server.load(attack_type.atlas_path());
    let region_id = resolve_region_id(atlas_assets, &atlas_handle, REGION_HOVER);
    let anim = make_animation_bundle(atlas_assets, &atlas_handle, REGION_HOVER);

    (
        PxAtlasSprite::new(atlas_handle, region_id),
        anim,
        ColliderData::from_one(Collider::new_circle(attack_type.base_collider_radius())),
    )
}

/// Creates the atlas-based destroy animation bundle. Returns `None` if the
/// atlas has no "destroy" region.
#[must_use]
pub fn make_destroy_atlas_bundle(
    asset_server: &AssetServer,
    atlas_assets: &Assets<PxSpriteAtlasAsset>,
    attack_type: &EnemyHoveringAttackType,
) -> Option<(PxAtlasSprite, PxAnimationBundle)> {
    let atlas_handle: Handle<PxSpriteAtlasAsset> = asset_server.load(attack_type.atlas_path());
    let atlas = atlas_assets.get(&atlas_handle)?;
    let region_id = atlas.region_id(REGION_DESTROY)?;
    let anim = atlas
        .animation(REGION_DESTROY)
        .map(|a| {
            PxAnimationBundle::from_parts(
                a.px_direction(),
                a.px_duration(),
                a.px_finish_behavior(),
                PxFrameTransition::None,
            )
        })
        .unwrap_or_default();

    Some((PxAtlasSprite::new(atlas_handle, region_id), anim))
}

/// Creates the atlas-based hit animation bundle for a separate hit-effect entity.
#[must_use]
pub fn make_hit_atlas_bundle(
    asset_server: &AssetServer,
    atlas_assets: &Assets<PxSpriteAtlasAsset>,
    attack_type: &EnemyHoveringAttackType,
) -> (PxAtlasSprite, PxAnimationBundle) {
    let atlas_handle: Handle<PxSpriteAtlasAsset> = asset_server.load(attack_type.atlas_path());
    let region_id = resolve_region_id(atlas_assets, &atlas_handle, REGION_HIT);
    let anim = make_animation_bundle(atlas_assets, &atlas_handle, REGION_HIT);

    (PxAtlasSprite::new(atlas_handle, region_id), anim)
}

fn resolve_region_id(
    atlas_assets: &Assets<PxSpriteAtlasAsset>,
    handle: &Handle<PxSpriteAtlasAsset>,
    name: &str,
) -> carapace::prelude::AtlasRegionId {
    atlas_assets
        .get(handle)
        .and_then(|a| a.region_id(name))
        .unwrap_or_default()
}

fn make_animation_bundle(
    atlas_assets: &Assets<PxSpriteAtlasAsset>,
    handle: &Handle<PxSpriteAtlasAsset>,
    name: &str,
) -> PxAnimationBundle {
    atlas_assets
        .get(handle)
        .and_then(|a| a.animation(name))
        .map_or_else(
            || {
                // HACK: Atlas not loaded yet — safe fallback. Loop prevents the
                // default Despawn finish behavior from killing the entity on the
                // first frame. The correct fix is to decouple spawn from atlas
                // readiness: either pre-load attack atlases during stage setup, or
                // use a reactive system that patches animation parameters once the
                // atlas becomes available (via AssetEvent::LoadedWithDependencies).
                PxAnimationBundle::from_parts(
                    carapace::prelude::PxAnimationDirection::Foreward,
                    carapace::prelude::PxAnimationDuration::millis_per_animation(1000),
                    carapace::prelude::PxAnimationFinishBehavior::Loop,
                    PxFrameTransition::None,
                )
            },
            |a| {
                PxAnimationBundle::from_parts(
                    a.px_direction(),
                    a.px_duration(),
                    a.px_finish_behavior(),
                    PxFrameTransition::None,
                )
            },
        )
}
