use super::EnemyHoveringAttackType;
use crate::stage::components::interactive::{Collider, ColliderData};
use bevy::prelude::*;
use carapace::prelude::{CxAnimationBundle, CxAtlasSprite, CxFrameTransition, CxSpriteAtlasAsset};

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
    atlas_assets: &Assets<CxSpriteAtlasAsset>,
    attack_type: &EnemyHoveringAttackType,
) -> (CxAtlasSprite, CxAnimationBundle, ColliderData) {
    let atlas_handle: Handle<CxSpriteAtlasAsset> = asset_server.load(attack_type.atlas_path());
    let region_id = resolve_region_id(atlas_assets, &atlas_handle, REGION_HOVER);
    let anim = make_animation_bundle(atlas_assets, &atlas_handle, REGION_HOVER);

    (
        CxAtlasSprite::new(atlas_handle, region_id),
        anim,
        ColliderData::from_one(Collider::new_circle(attack_type.base_collider_radius())),
    )
}

/// Creates the atlas-based destroy animation bundle. Returns `None` if the
/// atlas has no "destroy" region.
#[must_use]
pub fn make_destroy_atlas_bundle(
    asset_server: &AssetServer,
    atlas_assets: &Assets<CxSpriteAtlasAsset>,
    attack_type: &EnemyHoveringAttackType,
) -> Option<(CxAtlasSprite, CxAnimationBundle)> {
    let atlas_handle: Handle<CxSpriteAtlasAsset> = asset_server.load(attack_type.atlas_path());
    let atlas = atlas_assets.get(&atlas_handle)?;
    let region_id = atlas.region_id(REGION_DESTROY)?;
    let anim = atlas
        .animation(REGION_DESTROY)
        .map(|a| {
            CxAnimationBundle::from_parts(
                a.px_direction(),
                a.px_duration(),
                a.px_finish_behavior(),
                CxFrameTransition::None,
            )
        })
        .unwrap_or_default();

    Some((CxAtlasSprite::new(atlas_handle, region_id), anim))
}

/// Creates the atlas-based hit animation bundle for a separate hit-effect entity.
#[must_use]
pub fn make_hit_atlas_bundle(
    asset_server: &AssetServer,
    atlas_assets: &Assets<CxSpriteAtlasAsset>,
    attack_type: &EnemyHoveringAttackType,
) -> (CxAtlasSprite, CxAnimationBundle) {
    let atlas_handle: Handle<CxSpriteAtlasAsset> = asset_server.load(attack_type.atlas_path());
    let region_id = resolve_region_id(atlas_assets, &atlas_handle, REGION_HIT);
    let anim = make_animation_bundle(atlas_assets, &atlas_handle, REGION_HIT);

    (CxAtlasSprite::new(atlas_handle, region_id), anim)
}

fn resolve_region_id(
    atlas_assets: &Assets<CxSpriteAtlasAsset>,
    handle: &Handle<CxSpriteAtlasAsset>,
    name: &str,
) -> carapace::prelude::AtlasRegionId {
    atlas_assets
        .get(handle)
        .and_then(|a| a.region_id(name))
        .unwrap_or_default()
}

fn make_animation_bundle(
    atlas_assets: &Assets<CxSpriteAtlasAsset>,
    handle: &Handle<CxSpriteAtlasAsset>,
    name: &str,
) -> CxAnimationBundle {
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
                CxAnimationBundle::from_parts(
                    carapace::prelude::CxAnimationDirection::Forward,
                    carapace::prelude::CxAnimationDuration::millis_per_animation(1000),
                    carapace::prelude::CxAnimationFinishBehavior::Loop,
                    CxFrameTransition::None,
                )
            },
            |a| {
                CxAnimationBundle::from_parts(
                    a.px_direction(),
                    a.px_duration(),
                    a.px_finish_behavior(),
                    CxFrameTransition::None,
                )
            },
        )
}
