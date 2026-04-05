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

pub use crate::frame::{
    PxFrame, PxFrameBinding, PxFrameControl, PxFrameCount, PxFrameSelector, PxFrameTransition,
    PxFrameView,
};
#[cfg(feature = "line")]
pub use crate::line::PxLine;
#[cfg(feature = "particle")]
pub use crate::particle::{PxEmitter, PxEmitterFrequency, PxEmitterSimulation, PxParticleLifetime};
#[cfg(feature = "headed")]
pub use crate::picking::PxPixelPick;
#[cfg(feature = "gpu_palette")]
pub use crate::sprite::{PxGpuComposite, PxGpuSprite};
pub use crate::{
    PxHeadlessPlugin, PxPlugin,
    animation::{
        PxAnimation, PxAnimationDirection, PxAnimationDuration, PxAnimationFinishBehavior,
        PxAnimationFinished, PxAnimationPlugin,
    },
    atlas::{AtlasRect, AtlasRegion, AtlasRegionId, PxAtlasSprite, PxSpriteAtlasAsset},
    camera::{PxCamera, PxCanvas},
    cursor::{PxCursor, PxCursorPosition},
    filter::{PxFilter, PxFilterAsset, PxFilterLayers, PxInvertMask},
    map::{PxMap, PxTile, PxTiles, PxTileset},
    math::{Diagonal, Orthogonal},
    position::{PxAnchor, PxLayer, PxPosition, PxSubPosition, PxVelocity},
    presentation::PxPresentationTransform,
    rect::PxRect,
    screen::ScreenSize,
    sprite::{PxCompositePart, PxCompositePartSource, PxCompositeSprite, PxSprite, PxSpriteAsset},
    text::{PxText, PxTypeface},
    ui::{
        PxCaret, PxGrid, PxGridRow, PxGridRows, PxKeyField, PxKeyFieldUpdate, PxMargin, PxMinSize,
        PxRow, PxRowSlot, PxScroll, PxStack, PxTextField, PxTextFieldUpdate, PxUiRoot,
    },
};

pub use carapace_macros::px_layer;
