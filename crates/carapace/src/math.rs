//! Math helpers

use crate::prelude::*;

pub use next::Next;

/// Extension trait for [`IRect`].
///
/// All rects in carapace use **min-inclusive / max-exclusive** bounds:
/// a pixel at `max` is outside the rect.  Use [`contains_exclusive`](Self::contains_exclusive)
/// for hit-testing consistent with this convention.
pub trait RectExt {
    /// Point is inside the rect (max-exclusive).
    fn contains_exclusive(self, point: IVec2) -> bool;

    /// Rect from bottom-left `pos` and unsigned `size`.  Max is exclusive.
    fn from_pos_size(pos: IVec2, size: UVec2) -> Self;

    /// Creates a rectangle from a position, size, and anchor
    fn pos_size_anchor(pos: IVec2, size: UVec2, anchor: CxAnchor) -> Self;

    /// Subtracts an [`IVec2`] from the rectangle's points
    #[must_use]
    fn sub_ivec2(self, other: IVec2) -> Self;
}

impl RectExt for IRect {
    fn contains_exclusive(self, point: IVec2) -> bool {
        point.cmpge(self.min).all() && point.cmplt(self.max).all()
    }

    fn from_pos_size(pos: IVec2, size: UVec2) -> Self {
        Self {
            min: pos,
            max: pos + size.as_ivec2(),
        }
    }

    fn pos_size_anchor(pos: IVec2, size: UVec2, anchor: CxAnchor) -> Self {
        let min = pos - anchor.pos(size).as_ivec2();

        Self {
            min,
            max: min + size.as_ivec2(),
        }
    }

    fn sub_ivec2(self, other: IVec2) -> Self {
        Self {
            min: self.min - other,
            max: self.max - other,
        }
    }
}

/// An orthogonal direction
#[derive(Debug)]
pub enum Orthogonal {
    /// Right
    Right,
    /// Up
    Up,
    /// Left
    Left,
    /// Down
    Down,
}

/// A diagonal direction
#[derive(Copy, Clone)]
pub enum Diagonal {
    /// Up-right
    UpRight,
    /// Up-left
    UpLeft,
    /// Down-left
    DownLeft,
    /// Down-right
    DownRight,
}

impl Diagonal {
    /// 1 for each positive axis and 0 for each negative axis
    #[must_use]
    pub fn as_uvec2(self) -> UVec2 {
        use Diagonal::{DownLeft, DownRight, UpLeft, UpRight};

        match self {
            UpRight => UVec2::ONE,
            UpLeft => UVec2::new(0, 1),
            DownLeft => UVec2::ZERO,
            DownRight => UVec2::new(1, 0),
        }
    }
}
