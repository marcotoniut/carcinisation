//! Module for convenient imports. Use with `use carapace::prelude::*;`.

pub(crate) use bevy_app::prelude::*;
pub(crate) use bevy_asset::prelude::*;
#[cfg(feature = "headed")]
pub(crate) use bevy_camera::prelude::*;
pub(crate) use bevy_color::prelude::*;
pub(crate) use bevy_ecs::prelude::*;
pub(crate) use bevy_image::prelude::*;
pub(crate) use bevy_input::prelude::*;
pub(crate) use bevy_log::prelude::*;
pub(crate) use bevy_math::prelude::*;
pub(crate) use bevy_reflect::prelude::*;
pub(crate) use bevy_render::prelude::*;
#[cfg(feature = "headed")]
pub(crate) use bevy_shader::prelude::*;
pub(crate) use bevy_time::prelude::*;
pub(crate) use bevy_transform::prelude::*;
#[cfg(feature = "particle")]
pub(crate) use bevy_turborand::prelude::*;
pub(crate) use bevy_utils::prelude::*;
#[cfg(feature = "headed")]
pub(crate) use bevy_window::prelude::*;

pub(crate) const OK: Result = Ok(());

#[cfg(feature = "headed")]
pub use crate::debug_draw::{
    draw_circle_collider_2d, draw_rect_collider_2d, draw_world_mask_outline_2d,
};
pub use crate::frame::{
    CxFrameBinding, CxFrameControl, CxFrameCount, CxFrameSelector, CxFrameTransition, CxFrameView,
};
#[cfg(feature = "line")]
pub use crate::line::CxLine;
#[cfg(feature = "particle")]
pub use crate::particle::{CxEmitter, CxEmitterFrequency, CxEmitterSimulation, ParticleLifetime};
#[cfg(feature = "headed")]
pub use crate::picking::CxPick;
#[cfg(feature = "gpu_palette")]
pub use crate::sprite::{CxGpuComposite, CxGpuSprite};
pub use crate::{
    CxHeadlessPlugin, CxPlugin,
    animation::{
        CxAnimation, CxAnimationDirection, CxAnimationDuration, CxAnimationFinishBehavior,
        CxAnimationFinished, CxAnimationPlugin,
    },
    atlas::{AtlasRect, AtlasRegion, AtlasRegionId, CxAtlasSprite, CxSpriteAtlasAsset},
    blink::CxBlink,
    camera::{CxCamera, CxRenderSpace},
    cursor::{CxCursor, CxCursorPosition},
    filter::{CxFilter, CxFilterAsset, CxFilterLayers, CxInvertMask},
    math::{Diagonal, Orthogonal},
    position::{CxAnchor, CxLayer, CxPosition, CxVelocity, WorldPos},
    presentation::CxPresentationTransform,
    rect::CxFilterRect,
    screen::{CxOverlayCamera, CxScreenSize},
    sprite::{
        CxAuthoritativeCompositeMetrics, CxCompositePart, CxCompositePartSource, CxCompositeSprite,
        CxSprite, CxSpriteAsset, PartTransform,
    },
    text::{CxText, CxTypeface},
    tilemap::{CxTile, CxTilemap, CxTiles, CxTileset},
    ui::{
        CxCaret, CxGrid, CxGridRow, CxGridTracks, CxKeyField, CxKeyFieldUpdate, CxMargin,
        CxMinSize, CxRow, CxRowSlot, CxScroll, CxStack, CxTextField, CxTextFieldUpdate, CxUiRoot,
    },
};

pub use carapace_macros::px_layer;
