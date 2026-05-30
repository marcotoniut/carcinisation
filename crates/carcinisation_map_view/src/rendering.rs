use carapace::image::CxImage;

use crate::classification::{CellKind, MapGrid};

/// Pick the palette index most visually distant from `floor`.
///
/// With a small palette (5 entries), just pick the opposite end:
/// light floor → darkest (1), dark floor → lightest (4).
fn contrasting_inline(floor: u8) -> u8 {
    if floor >= 3 { 1 } else { 4 }
}

/// Render a top-down map view from a [`MapGrid`] into a palette-indexed image.
///
/// - `Void` → transparent (palette index 0)
/// - `ReachableFloor` → solid floor colour
/// - `Wall` → 1 px inner border in a floor-contrasting colour, interior
///   filled with a checkerboard dither of the wall's two dominant colours.
#[must_use]
pub fn render_map_view(grid: &MapGrid, tile_size: u32) -> CxImage {
    let ts = tile_size as usize;
    let w = grid.width as u32 * tile_size;
    let h = grid.height as u32 * tile_size;
    let mut data = vec![0u8; (w * h) as usize];
    let inline_color = contrasting_inline(grid.floor_color);

    for gy in 0..grid.height {
        for gx in 0..grid.width {
            let cell = &grid.cells[gy * grid.width + gx];
            if cell.color == 0 {
                continue;
            }
            let is_wall = matches!(cell.kind, CellKind::Wall(_));
            for dy in 0..ts {
                for dx in 0..ts {
                    let px = gx * ts + dx;
                    let py = (grid.height - 1 - gy) * ts + dy;
                    let color = if is_wall
                        && ts >= 3
                        && (dx == 0 || dx == ts - 1 || dy == 0 || dy == ts - 1)
                    {
                        // 1 px inner border on walls.
                        inline_color
                    } else if is_wall && cell.color != cell.color_alt {
                        // Dithered interior.
                        if (dx + dy) % 2 == 0 {
                            cell.color
                        } else {
                            cell.color_alt
                        }
                    } else {
                        cell.color
                    };
                    data[py * w as usize + px] = color;
                }
            }
        }
    }

    CxImage::new(data, w as usize)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::classification::MapGrid;

    const TEST_TILE_SIZE: u32 = 4;

    fn test_grid() -> MapGrid {
        let width = 3;
        let height = 3;
        let cells = vec![1, 1, 1, 1, 0, 1, 1, 1, 1];
        MapGrid::classify(width, height, &cells, 3, &[(7, 7)])
    }

    #[test]
    fn render_map_view_produces_correct_size() {
        let grid = test_grid();
        let image = render_map_view(&grid, TEST_TILE_SIZE);
        let expected_w = 3 * TEST_TILE_SIZE;
        let expected_h = 3 * TEST_TILE_SIZE;
        assert_eq!(image.width(), expected_w as usize);
        assert_eq!(image.height(), expected_h as usize);
    }

    #[test]
    fn void_cells_are_transparent() {
        // 5×3 grid: two floor pockets separated by a wall column.
        // Player start at (1.5, 1.5) → only left pocket reachable.
        let cells = vec![1, 1, 1, 1, 1, 1, 0, 1, 0, 1, 1, 1, 1, 1, 1];
        let mut grid = MapGrid::classify(5, 3, &cells, 3, &[(7, 7)]);
        grid.classify_voids(&[(1.5, 1.5)]);
        let image = render_map_view(&grid, TEST_TILE_SIZE);
        let w = image.width();

        // Reachable floor cell (1,1) → pixel centre after Y-flip.
        // gy=1, py = (3-1-1)*4 = 4, px = 1*4 = 4.
        let reachable_px = 6 * w + 6; // interior pixel of reachable floor
        assert_ne!(
            image.data()[reachable_px],
            0,
            "reachable floor should be visible"
        );

        // Unreachable floor cell (3,1) → Void, all pixels should be 0.
        // gy=1, py = (3-1-1)*4 = 4, px = 3*4 = 12.
        for dy in 0..TEST_TILE_SIZE as usize {
            for dx in 0..TEST_TILE_SIZE as usize {
                let px = 3 * TEST_TILE_SIZE as usize + dx;
                let py = TEST_TILE_SIZE as usize + dy;
                assert_eq!(
                    image.data()[py * w + px],
                    0,
                    "void cell pixel ({px},{py}) should be transparent"
                );
            }
        }
    }
}
