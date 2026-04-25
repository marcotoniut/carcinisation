use bevy_derive::{Deref, DerefMut};

use crate::{position::DefaultLayer, prelude::*};

/// Marks the root entity of a UI tree.
#[derive(Component)]
#[require(CxRenderSpace, DefaultLayer)]
pub struct CxUiRoot;

/// Sets a minimum size for a UI node.
#[derive(Component, Deref, DerefMut, Default, Reflect)]
#[cfg_attr(feature = "headed", require(Visibility))]
pub struct CxMinSize(pub UVec2);

/// Adds pixel margin around a UI node.
#[derive(Component, Deref, DerefMut, Reflect)]
#[cfg_attr(feature = "headed", require(Visibility))]
pub struct CxMargin(pub u32);

impl Default for CxMargin {
    fn default() -> Self {
        Self(1)
    }
}

/// Per-child layout options for [`CxRow`].
#[derive(Component, Default, Clone)]
pub struct CxRowSlot {
    /// If true, the slot expands to fill available space.
    pub stretch: bool,
}

/// Row/column layout container for UI children.
#[derive(Component, Default, Clone, Reflect)]
#[cfg_attr(feature = "headed", require(Visibility))]
pub struct CxRow {
    /// If true, lay out children vertically; otherwise horizontally.
    pub vertical: bool,
    /// Space between children in pixels.
    pub space_between: u32,
}

/// Row sizing config used by [`CxGrid`].
#[derive(Default, Clone, Reflect)]
pub struct CxGridRow {
    /// If true, the row expands to fill available space.
    pub stretch: bool,
}

/// Track definitions for [`CxGrid`] — used for both rows and columns.
///
/// Each track entry ([`CxGridRow`]) specifies a sizing strategy for one
/// row or column.
#[derive(Default, Clone, Reflect)]
pub struct CxGridTracks {
    /// Track definitions (one per row or column).
    pub rows: Vec<CxGridRow>,
    /// Space between tracks in pixels.
    pub space_between: u32,
}

/// Grid layout container for UI children.
#[derive(Component, Clone)]
#[cfg_attr(feature = "headed", require(Visibility))]
pub struct CxGrid {
    /// Number of columns in the grid.
    pub width: u32,
    /// Row sizing rules.
    pub rows: CxGridTracks,
    /// Column sizing rules.
    pub columns: CxGridTracks,
}

impl Default for CxGrid {
    fn default() -> Self {
        Self {
            width: 2,
            rows: default(),
            columns: default(),
        }
    }
}

/// Stack layout container; children overlap in insertion order.
#[derive(Component, Clone, Reflect)]
#[cfg_attr(feature = "headed", require(Visibility))]
pub struct CxStack;

/// Scroll container that masks and offsets child content.
#[derive(Component, Default, Clone, Copy, Reflect)]
#[require(CxInvertMask, crate::rect::CxFilterRect)]
pub struct CxScroll {
    /// If true, scroll horizontally; otherwise vertically.
    pub horizontal: bool,
    /// Current scroll offset in pixels.
    pub scroll: u32,
    /// Maximum scroll offset in pixels.
    pub max_scroll: u32,
}
