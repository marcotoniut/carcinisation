//! Grid-based map representation for first-person stages.

/// A 2D grid map where each cell is either empty (0) or a wall type (>0).
#[derive(Clone, Debug)]
pub struct FpMap {
    pub width: usize,
    pub height: usize,
    /// Row-major cell data. `cells[y * width + x]`.
    /// 0 = empty, >0 = wall texture ID.
    pub cells: Vec<u8>,
}

impl FpMap {
    /// Look up the cell at grid position `(x, y)`.
    /// Returns 0 (empty) for out-of-bounds coordinates.
    #[must_use]
    pub fn get(&self, x: i32, y: i32) -> u8 {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return 0;
        }
        self.cells[y as usize * self.width + x as usize]
    }
}

/// Hardcoded 8x8 test map for M0.
///
/// ```text
/// 1 1 1 1 1 1 1 1
/// 1 . . . . . . 1
/// 1 . . 2 2 . . 1
/// 1 . . . . . . 1
/// 1 . 2 . . 2 . 1
/// 1 . . . . . . 1
/// 1 . . . . . . 1
/// 1 1 1 1 1 1 1 1
/// ```
#[must_use]
pub fn test_map() -> FpMap {
    #[rustfmt::skip]
    let cells = vec![
        1, 1, 1, 1, 1, 1, 1, 1,
        1, 0, 0, 0, 0, 0, 0, 1,
        1, 0, 0, 2, 2, 0, 0, 1,
        1, 0, 0, 0, 0, 0, 0, 1,
        1, 0, 2, 0, 0, 2, 0, 1,
        1, 0, 0, 0, 0, 0, 0, 1,
        1, 0, 0, 0, 0, 0, 0, 1,
        1, 1, 1, 1, 1, 1, 1, 1,
    ];
    FpMap {
        width: 8,
        height: 8,
        cells,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_boundaries() {
        let map = test_map();
        // Corners are walls.
        assert_eq!(map.get(0, 0), 1);
        assert_eq!(map.get(7, 7), 1);
        // Interior is empty.
        assert_eq!(map.get(1, 1), 0);
        // Interior wall.
        assert_eq!(map.get(3, 2), 2);
        // Out of bounds.
        assert_eq!(map.get(-1, 0), 0);
        assert_eq!(map.get(8, 0), 0);
    }
}
