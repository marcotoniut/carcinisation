//! Type registration for the Bevy reflection system.
//!
//! Enabled by the `reflect` feature. Registers all public `carapace` types
//! that derive [`Reflect`] into the [`AppTypeRegistry`], making them visible to
//! BRP, `bevy-inspector-egui`, scene serialization, and other reflection-based tools.

use crate::prelude::*;

pub(crate) fn register_types(app: &mut App) {
    use crate::{blink::Blink, map::PxTile, text::PxText};

    // Position & camera
    app.register_type::<PxPosition>()
        .register_type::<PxAnchor>()
        .register_type::<PxSubPosition>()
        .register_type::<PxVelocity>()
        .register_type::<PxCamera>()
        .register_type::<PxCanvas>()
        // Sprite
        .register_type::<PxSprite>()
        .register_type::<PxSpriteAsset>()
        // Text
        .register_type::<PxText>()
        // Filter
        .register_type::<PxFilter>()
        .register_type::<PxFilterAsset>()
        .register_type::<PxInvertMask>()
        // Frame
        .register_type::<PxFrameSelector>()
        .register_type::<PxFrameTransition>()
        .register_type::<PxFrameBinding>()
        .register_type::<PxFrameView>()
        .register_type::<PxFrameCount>()
        .register_type::<PxFrameControl>()
        // Animation
        .register_type::<PxAnimationDirection>()
        .register_type::<PxAnimationDuration>()
        .register_type::<PxAnimationFinishBehavior>()
        .register_type::<PxAnimationFinished>()
        // Map
        .register_type::<PxTile>()
        .register_type::<PxTileset>()
        // Other
        .register_type::<Blink>()
        .register_type::<PxCursor>()
        .register_type::<PxCursorPosition>()
        .register_type::<PxRect>()
        // Atlas
        .register_type::<PxSpriteAtlasAsset>()
        .register_type::<AtlasRect>()
        .register_type::<AtlasRegion>()
        .register_type::<AtlasRegionId>()
        // UI
        .register_type::<PxMinSize>()
        .register_type::<PxMargin>()
        .register_type::<PxRow>()
        .register_type::<PxStack>()
        .register_type::<PxScroll>();

    // Feature-gated types
    #[cfg(feature = "line")]
    app.register_type::<PxLine>();
    #[cfg(feature = "particle")]
    app.register_type::<PxParticleLifetime>();
}
