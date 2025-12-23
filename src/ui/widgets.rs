use bevy_derive::{Deref, DerefMut};

use crate::{position::DefaultLayer, prelude::*};

/// Marks the root entity of a UI tree.
#[derive(Component)]
#[require(PxCanvas, DefaultLayer)]
pub struct PxUiRoot;

/// Sets a minimum size for a UI node.
#[derive(Component, Deref, DerefMut, Default, Reflect)]
#[cfg_attr(feature = "headed", require(Visibility))]
pub struct PxMinSize(pub UVec2);

/// Adds pixel margin around a UI node.
#[derive(Component, Deref, DerefMut, Reflect)]
#[cfg_attr(feature = "headed", require(Visibility))]
pub struct PxMargin(pub u32);

impl Default for PxMargin {
    fn default() -> Self {
        Self(1)
    }
}

/// Per-child layout options for [`PxRow`].
#[derive(Component, Default, Clone)]
pub struct PxRowSlot {
    /// If true, the slot expands to fill available space.
    pub stretch: bool,
}

/// Row/column layout container for UI children.
#[derive(Component, Default, Clone, Reflect)]
#[cfg_attr(feature = "headed", require(Visibility))]
pub struct PxRow {
    /// If true, lay out children vertically; otherwise horizontally.
    pub vertical: bool,
    /// Space between children in pixels.
    pub space_between: u32,
}

/// Row sizing config used by [`PxGrid`].
#[derive(Default, Clone, Reflect)]
pub struct PxGridRow {
    /// If true, the row expands to fill available space.
    pub stretch: bool,
}

/// Row/column definitions for [`PxGrid`].
#[derive(Default, Clone, Reflect)]
pub struct PxGridRows {
    /// Row definitions.
    pub rows: Vec<PxGridRow>,
    /// Space between rows/columns in pixels.
    pub space_between: u32,
}

/// Grid layout container for UI children.
#[derive(Component, Clone)]
#[cfg_attr(feature = "headed", require(Visibility))]
pub struct PxGrid {
    /// Number of columns in the grid.
    pub width: u32,
    /// Row sizing rules.
    pub rows: PxGridRows,
    /// Column sizing rules.
    pub columns: PxGridRows,
}

impl Default for PxGrid {
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
pub struct PxStack;

/// Scroll container that masks and offsets child content.
#[derive(Component, Default, Clone, Copy, Reflect)]
#[require(PxInvertMask, PxRect)]
pub struct PxScroll {
    /// If true, scroll horizontally; otherwise vertically.
    pub horizontal: bool,
    /// Current scroll offset in pixels.
    pub scroll: u32,
    /// Maximum scroll offset in pixels.
    pub max_scroll: u32,
}
