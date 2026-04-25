use bevy_derive::{Deref, DerefMut};

use crate::{filter::DefaultCxFilterLayers, prelude::*};

/// A rectangular region size used by the UI layout system (e.g. scroll viewports).
///
/// The render-pipeline filter-rect drawing was removed; this component is retained
/// only because [`super::ui::layout`] reads and writes it during layout passes.
#[derive(Component, Deref, DerefMut, Clone, Copy, Reflect)]
#[require(CxFilter, DefaultCxFilterLayers, CxPosition, CxAnchor, CxRenderSpace)]
#[cfg_attr(feature = "headed", require(Visibility))]
pub struct CxFilterRect(pub UVec2);

impl Default for CxFilterRect {
    fn default() -> Self {
        Self(UVec2::ONE)
    }
}
