use std::collections::VecDeque;

/// Per-cell classification for map-view rendering.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CellKind {
    Void,
    Wall(u8),
    ReachableFloor,
}

/// Static grid classification for a Wolf3D-style map.
///
/// Classified once at map load. Grid cells hold palette indices for direct
/// rendering (no per-frame texture lookups).
#[derive(Clone, Debug)]
pub struct MapGrid {
    pub width: usize,
    pub height: usize,
    pub cells: Vec<CellClassification>,
    /// Floor palette index, used by the renderer to pick a contrasting inline.
    pub floor_color: u8,
}

#[derive(Clone, Copy, Debug)]
pub struct CellClassification {
    pub kind: CellKind,
    /// Primary palette index (0 = transparent for void).
    pub color: u8,
    /// Secondary palette index for dithered wall rendering.
    pub color_alt: u8,
}

impl MapGrid {
    /// Build classification from a raw `Map` and wall colour pairs.
    ///
    /// `wall_color_pairs` maps wall texture index → `(primary, secondary)`
    /// palette indices (1-based indexing, same as `Map.cells`).
    /// `floor_color` is the palette index for interior floor tiles.
    pub fn classify(
        width: usize,
        height: usize,
        cells: &[u8],
        floor_color: u8,
        wall_color_pairs: &[(u8, u8)],
    ) -> Self {
        let mut out = Vec::with_capacity(width * height);
        for &cell in cells {
            if cell == 0 {
                out.push(CellClassification {
                    kind: CellKind::ReachableFloor,
                    color: floor_color,
                    color_alt: floor_color,
                });
            } else {
                let idx = (cell - 1) as usize;
                let (primary, secondary) = wall_color_pairs.get(idx).copied().unwrap_or((1, 1));
                out.push(CellClassification {
                    kind: CellKind::Wall(cell),
                    color: primary,
                    color_alt: secondary,
                });
            }
        }
        MapGrid {
            width,
            height,
            cells: out,
            floor_color,
        }
    }

    /// Run flood-fill from `starts` to mark reachable floor tiles.
    ///
    /// Unreachable floor tiles become `Void`. Call after `classify`.
    pub fn classify_voids(&mut self, starts: &[(f32, f32)]) {
        let mut visited = vec![false; self.width * self.height];
        for &(sx, sy) in starts {
            let cx = sx as usize;
            let cy = sy as usize;
            if cx < self.width && cy < self.height {
                self.flood_fill(cx, cy, &mut visited);
            }
        }
        for (i, cell) in self.cells.iter_mut().enumerate() {
            if cell.kind == CellKind::ReachableFloor && !visited[i] {
                cell.kind = CellKind::Void;
                cell.color = 0;
                cell.color_alt = 0;
            }
        }
    }

    fn flood_fill(&self, x: usize, y: usize, visited: &mut [bool]) {
        let mut queue = VecDeque::new();
        queue.push_back((x, y));
        while let Some((cx, cy)) = queue.pop_front() {
            let idx = cy * self.width + cx;
            if visited[idx] {
                continue;
            }
            if !matches!(self.cells[idx].kind, CellKind::ReachableFloor) {
                continue;
            }
            visited[idx] = true;
            if cx > 0 {
                queue.push_back((cx - 1, cy));
            }
            if cx + 1 < self.width {
                queue.push_back((cx + 1, cy));
            }
            if cy > 0 {
                queue.push_back((cx, cy - 1));
            }
            if cy + 1 < self.height {
                queue.push_back((cx, cy + 1));
            }
        }
    }

    /// Repopulate from scratch given a `carcinisation_fps_core::map::Map`.
    pub fn from_fps_map(
        map: &carcinisation_fps_core::map::Map,
        floor_color: u8,
        wall_color_pairs: &[(u8, u8)],
        player_starts: &[(f32, f32)],
    ) -> Self {
        let mut grid = Self::classify(
            map.width,
            map.height,
            &map.cells,
            floor_color,
            wall_color_pairs,
        );
        grid.classify_voids(player_starts);
        grid
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_test_map(
        width: usize,
        height: usize,
        cells: Vec<u8>,
    ) -> carcinisation_fps_core::map::Map {
        carcinisation_fps_core::map::Map {
            width,
            height,
            cells,
        }
    }

    #[test]
    fn enclosed_room_classifies_correctly() {
        // 5x5 map: border walls (1), interior floor (0).
        // Player starts at center (2.5, 2.5).
        let cells = vec![
            1, 1, 1, 1, 1, 1, 0, 0, 0, 1, 1, 0, 0, 0, 1, 1, 0, 0, 0, 1, 1, 1, 1, 1, 1,
        ];
        let map = empty_test_map(5, 5, cells);
        let grid = MapGrid::from_fps_map(&map, 3, &[(2, 2)], &[(2.5, 2.5)]);
        assert_eq!(grid.cells.len(), 25);
        // Border walls.
        for x in 0..5 {
            assert_eq!(grid.cells[x].kind, CellKind::Wall(1), "top wall");
            assert_eq!(grid.cells[4 * 5 + x].kind, CellKind::Wall(1), "bottom wall");
        }
        for y in 0..5 {
            assert_eq!(grid.cells[y * 5].kind, CellKind::Wall(1), "left wall");
            assert_eq!(grid.cells[y * 5 + 4].kind, CellKind::Wall(1), "right wall");
        }
        // Interior floor.
        assert_eq!(grid.cells[6].kind, CellKind::ReachableFloor);
        assert_eq!(grid.cells[7].kind, CellKind::ReachableFloor);
        assert_eq!(grid.cells[8].kind, CellKind::ReachableFloor);
        assert_eq!(grid.cells[11].kind, CellKind::ReachableFloor);
        assert_eq!(grid.cells[12].kind, CellKind::ReachableFloor);
    }

    #[test]
    fn unreachable_internal_void_is_void() {
        // 7x7 map: outer ring reachable, inner 3x3 sealed by walls (void).
        //   WWWWWWW
        //   W00000W   0 = floor
        //   W0WWW0W   W = wall (1)
        //   W0W0W0W
        //   W0WWW0W
        //   W00000W
        //   WWWWWWW
        // Player start at (1.5, 1.5).
        let cells = vec![
            1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 1, 1, 0, 1, 1, 1, 0, 1, 1, 0, 1, 0, 1, 0, 1, 1,
            0, 1, 1, 1, 0, 1, 1, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1,
        ];
        let map = empty_test_map(7, 7, cells);
        let grid = MapGrid::from_fps_map(&map, 3, &[(2, 2)], &[(1.5, 1.5)]);

        // Center cell (3,3) = index 3*7+3 = 24 — unreachable → Void.
        assert_eq!(grid.cells[24].kind, CellKind::Void);
        // Its color must be 0 (transparent).
        assert_eq!(grid.cells[24].color, 0);

        // Surrounding void ring inside the inner walls.
        assert_eq!(grid.cells[3 * 7 + 2].kind, CellKind::Wall(1)); // left inner wall
        assert_eq!(grid.cells[3 * 7 + 4].kind, CellKind::Wall(1)); // right inner wall
        assert_eq!(grid.cells[2 * 7 + 3].kind, CellKind::Wall(1)); // top inner wall
        assert_eq!(grid.cells[4 * 7 + 3].kind, CellKind::Wall(1)); // bottom inner wall

        // Reachable outer corridor is ReachableFloor.
        assert_eq!(grid.cells[7 + 1].kind, CellKind::ReachableFloor);
        assert_eq!(grid.cells[5 * 7 + 5].kind, CellKind::ReachableFloor);
    }

    #[test]
    fn single_cell_void_inside_room() {
        // 3x3 with a single wall pillar at center.
        // Player starts at (0.5, 0.5) — unreachable to center pillar.
        let cells = vec![0, 0, 0, 0, 1, 0, 0, 0, 0];
        let map = empty_test_map(3, 3, cells);
        let grid = MapGrid::from_fps_map(&map, 3, &[(2, 2)], &[(0.5, 0.5)]);

        // Center is wall (1), not void.
        assert_eq!(grid.cells[4].kind, CellKind::Wall(1));

        // All floor cells reachable since no walls block.
        for i in 0..9 {
            if i != 4 {
                assert_eq!(grid.cells[i].kind, CellKind::ReachableFloor, "cell {i}");
            }
        }
    }

    #[test]
    fn wall_color_classification() {
        let cells = vec![1, 0, 0, 2];
        let map = empty_test_map(2, 2, cells);
        // Use classify directly (no flood-fill) to verify wall color mapping.
        let grid = MapGrid::classify(2, 2, &map.cells, 3, &[(7, 7), (12, 12)]);

        assert_eq!(grid.cells[0].kind, CellKind::Wall(1));
        assert_eq!(grid.cells[0].color, 7);

        assert_eq!(grid.cells[3].kind, CellKind::Wall(2));
        assert_eq!(grid.cells[3].color, 12);

        // Floor cells have floor color (classify sets all 0-cells to floor).
        assert_eq!(grid.cells[1].color, 3);
        assert_eq!(grid.cells[2].color, 3);
    }
}
