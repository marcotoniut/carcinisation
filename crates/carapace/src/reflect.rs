//! Type registration for the Bevy reflection system.
//!
//! Enabled by the `reflect` feature. Registers all public `carapace` types
//! that derive [`Reflect`] into the [`AppTypeRegistry`], making them visible to
//! BRP, `bevy-inspector-egui`, scene serialization, and other reflection-based tools.

use crate::prelude::*;

pub(crate) fn register_types(app: &mut App) {
    use crate::{blink::CxBlink, text::CxText, tilemap::CxTile};

    use crate::presentation::CxPresentationTransform;

    // Position & camera
    app.register_type::<CxPosition>()
        .register_type::<CxAnchor>()
        .register_type::<WorldPos>()
        .register_type::<CxVelocity>()
        .register_type::<CxCamera>()
        .register_type::<CxRenderSpace>()
        .register_type::<CxPresentationTransform>()
        // Sprite
        .register_type::<CxSprite>()
        .register_type::<CxSpriteAsset>()
        // Text
        .register_type::<CxText>()
        // Filter
        .register_type::<CxFilter>()
        .register_type::<CxFilterAsset>()
        .register_type::<CxInvertMask>()
        // Frame
        .register_type::<CxFrameSelector>()
        .register_type::<CxFrameTransition>()
        .register_type::<CxFrameBinding>()
        .register_type::<CxFrameView>()
        .register_type::<CxFrameCount>()
        .register_type::<CxFrameControl>()
        // Animation
        .register_type::<CxAnimationDirection>()
        .register_type::<CxAnimationDuration>()
        .register_type::<CxAnimationFinishBehavior>()
        .register_type::<CxAnimationFinished>()
        // Map
        .register_type::<CxTile>()
        .register_type::<CxTileset>()
        // Other
        .register_type::<CxBlink>()
        .register_type::<CxCursor>()
        .register_type::<CxCursorPosition>()
        // Atlas
        .register_type::<CxSpriteAtlasAsset>()
        .register_type::<AtlasRect>()
        .register_type::<AtlasRegion>()
        .register_type::<AtlasRegionId>()
        // UI
        .register_type::<CxMinSize>()
        .register_type::<CxMargin>()
        .register_type::<CxRow>()
        .register_type::<CxStack>()
        .register_type::<CxScroll>();

    // Feature-gated types
    #[cfg(feature = "line")]
    app.register_type::<CxLine>();
    #[cfg(feature = "particle")]
    app.register_type::<ParticleLifetime>();
}
