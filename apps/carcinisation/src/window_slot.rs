//! Deterministic window placement for multi-client dev workflows.
//!
//! Maps a numeric slot index within a grid layout to a screen position so
//! that `dev-fps-duo` and similar recipes can tile client windows automatically.
//!
//! # Grid layout
//!
//! The `--window-grid COLSxROWS` option defines the tiling grid. Slot indices
//! are assigned row-major (left-to-right, top-to-bottom):
//!
//! ```text
//! 2x1 (duo):          2x2 (quattuor):      3x2 (six):
//! ┌─────┬─────┐       ┌─────┬─────┐        ┌────┬────┬────┐
//! │  0  │  1  │       │  0  │  1  │        │  0 │  1 │  2 │
//! └─────┴─────┘       ├─────┼─────┤        ├────┼────┼────┤
//!                     │  2  │  3  │        │  3 │  4 │  5 │
//!                     └─────┴─────┘        └────┴────┴────┘
//! ```
//!
//! # Ergonomic presets
//!
//! When `--window-grid` is omitted, the grid is inferred from the slot index:
//! - slot 0–1 → 2x1 (side-by-side halves)
//! - slot 2–3 → 2x2 (four quadrants)
//! - slot 4–5 → 3x2
//! - slot 6–8 → 3x3
//!
//! # Monitor detection
//!
//! At startup, the [`apply_window_slot`] system queries the primary monitor
//! for its physical dimensions. Fallback chain:
//!
//! 1. Primary monitor via Bevy's `Monitor` + `PrimaryMonitor` entities
//! 2. `CARCINISATION_DEV_SCREEN` env var (format: `WIDTHxHEIGHT`)
//! 3. Hardcoded 2560x1440
//!
//! # Usage
//!
//! ```text
//! # Explicit grid:
//! cargo run --bin multiplayer_client -- --window-slot 0 --window-grid 2x1
//! cargo run --bin multiplayer_client -- --window-slot 5 --window-grid 3x2
//!
//! # Auto-inferred grid:
//! cargo run --bin multiplayer_client -- --window-slot 0
//! cargo run --bin multiplayer_client -- --window-slot 1
//! ```
//!
//! Via justfile: `just dev-fps-duo` assigns slots 0+1 with grid 2x1.

use bevy::math::IVec2;
use bevy::prelude::*;
use bevy::window::{Monitor, PrimaryMonitor, PrimaryWindow, WindowPosition};

/// Env var for overriding screen dimensions (format: `WIDTHxHEIGHT`).
const DEV_SCREEN_ENV: &str = "CARCINISATION_DEV_SCREEN";

/// Fallback screen dimensions when neither monitor nor env var is available.
const FALLBACK_SCREEN_W: i32 = 2560;
const FALLBACK_SCREEN_H: i32 = 1440;

// ---------------------------------------------------------------------------
// Grid definition
// ---------------------------------------------------------------------------

/// Grid dimensions: columns x rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowGrid {
    pub cols: u32,
    pub rows: u32,
}

impl WindowGrid {
    /// Create a new grid.
    ///
    /// # Panics
    ///
    /// Panics if `cols` or `rows` is zero.
    #[must_use]
    pub fn new(cols: u32, rows: u32) -> Self {
        assert!(cols > 0 && rows > 0, "grid dimensions must be positive");
        Self { cols, rows }
    }

    /// Total number of slots in this grid.
    #[must_use]
    pub const fn total_slots(&self) -> u32 {
        self.cols * self.rows
    }

    /// Parse from `COLSxROWS` string (e.g. `"2x2"`, `"3x2"`).
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        let (c, r) = s.split_once('x').or_else(|| s.split_once('X'))?;
        let cols = c.trim().parse::<u32>().ok()?;
        let rows = r.trim().parse::<u32>().ok()?;
        if cols > 0 && rows > 0 {
            Some(Self { cols, rows })
        } else {
            None
        }
    }
}

/// Infer the smallest grid that fits a given slot index.
///
/// Used when no explicit `--window-grid` is provided.
#[must_use]
pub fn infer_grid(slot: u32) -> WindowGrid {
    match slot {
        0..=1 => WindowGrid::new(2, 1),
        2..=3 => WindowGrid::new(2, 2),
        4..=5 => WindowGrid::new(3, 2),
        6..=8 => WindowGrid::new(3, 3),
        _ => {
            // For very high slots, compute a square-ish grid.
            let side = (slot as f32 + 1.0).sqrt().ceil() as u32;
            WindowGrid::new(side, side)
        }
    }
}

// ---------------------------------------------------------------------------
// Geometry
// ---------------------------------------------------------------------------

/// A window slot definition: position and size in physical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlotGeometry {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// Compute window geometry for a slot within a grid on a given screen.
///
/// Returns `None` if `slot >= grid.total_slots()`.
#[must_use]
pub const fn grid_slot_geometry(
    slot: u32,
    grid: WindowGrid,
    screen_w: i32,
    screen_h: i32,
) -> Option<SlotGeometry> {
    if slot >= grid.total_slots() {
        return None;
    }
    let col = slot % grid.cols;
    let row = slot / grid.cols;
    let cell_w = screen_w / grid.cols as i32;
    let cell_h = screen_h / grid.rows as i32;
    Some(SlotGeometry {
        x: col as i32 * cell_w,
        y: row as i32 * cell_h,
        width: cell_w as u32,
        height: cell_h as u32,
    })
}

// ---------------------------------------------------------------------------
// Screen size resolution
// ---------------------------------------------------------------------------

/// Parse screen dimensions from the `CARCINISATION_DEV_SCREEN` env var.
///
/// Expected format: `WIDTHxHEIGHT` (e.g. `1920x1080`).
#[must_use]
pub fn screen_size_from_env() -> Option<(i32, i32)> {
    parse_dimensions(&std::env::var(DEV_SCREEN_ENV).ok()?)
}

/// Parse a `WIDTHxHEIGHT` string into (width, height).
#[must_use]
pub fn parse_dimensions(s: &str) -> Option<(i32, i32)> {
    let (w_str, h_str) = s.split_once('x').or_else(|| s.split_once('X'))?;
    let w = w_str.trim().parse::<i32>().ok()?;
    let h = h_str.trim().parse::<i32>().ok()?;
    if w > 0 && h > 0 { Some((w, h)) } else { None }
}

/// Resolve the effective screen dimensions.
///
/// Priority: monitor dimensions > env var > hardcoded fallback.
#[must_use]
pub fn resolve_screen_size(monitor: Option<(i32, i32)>) -> (i32, i32) {
    monitor
        .or_else(screen_size_from_env)
        .unwrap_or((FALLBACK_SCREEN_W, FALLBACK_SCREEN_H))
}

// ---------------------------------------------------------------------------
// Bevy integration
// ---------------------------------------------------------------------------

/// Resource holding the requested window slot index.
#[derive(Resource, Debug, Clone, Copy)]
pub struct WindowSlot(pub u32);

/// Resource holding an explicit grid override. When absent, the grid is
/// inferred from the slot index via [`infer_grid`].
#[derive(Resource, Debug, Clone, Copy)]
pub struct WindowGridOverride(pub WindowGrid);

/// Plugin that applies window slot positioning once monitor info is available.
///
/// Runs in `Update` and retries each frame until either the primary monitor
/// is detected or a fallback is used. Marks itself as applied after the
/// first successful placement — no further work after that.
pub struct WindowSlotPlugin;

impl Plugin for WindowSlotPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, apply_window_slot);
    }
}

/// Tracks window slot placement state.
#[derive(Default)]
pub struct WindowSlotState {
    applied: bool,
    /// Frames waited for monitor info before falling back.
    attempts: u32,
}

/// Max frames to wait for monitor entities before using fallback dimensions.
const MAX_MONITOR_WAIT_FRAMES: u32 = 10;

/// Retrying Update system: waits for primary monitor info, applies slot
/// placement once, then marks itself done.
///
/// No-op if [`WindowSlot`] resource is not present.
#[allow(clippy::needless_pass_by_value)]
pub fn apply_window_slot(
    slot_res: Option<Res<WindowSlot>>,
    grid_res: Option<Res<WindowGridOverride>>,
    monitor_q: Query<&Monitor, With<PrimaryMonitor>>,
    mut window_q: Query<&mut Window, With<PrimaryWindow>>,
    mut state: Local<WindowSlotState>,
) {
    if state.applied {
        return;
    }

    let Some(slot_res) = slot_res else {
        state.applied = true;
        return;
    };
    let slot = slot_res.0;
    let grid = grid_res.map_or_else(|| infer_grid(slot), |r| r.0);

    let monitor_dims = monitor_q
        .iter()
        .next()
        .map(|m| (m.physical_width as i32, m.physical_height as i32));

    // Wait for monitor info unless we have an env fallback or have waited long enough.
    if monitor_dims.is_none()
        && screen_size_from_env().is_none()
        && state.attempts < MAX_MONITOR_WAIT_FRAMES
    {
        state.attempts += 1;
        return;
    }

    state.applied = true;
    let (screen_w, screen_h) = resolve_screen_size(monitor_dims);

    let Some(geom) = grid_slot_geometry(slot, grid, screen_w, screen_h) else {
        warn!(
            "Window slot {slot} out of range for {}x{} grid ({} slots)",
            grid.cols,
            grid.rows,
            grid.total_slots()
        );
        return;
    };

    let Ok(mut window) = window_q.single_mut() else {
        return;
    };

    let monitor_origin = monitor_q
        .iter()
        .next()
        .map_or(IVec2::ZERO, |m| m.physical_position);
    window.position = WindowPosition::At(monitor_origin + IVec2::new(geom.x, geom.y));
    window
        .resolution
        .set_physical_resolution(geom.width, geom.height);

    info!(
        "Window slot {slot} ({}x{} grid): {}x{} at ({}, {})",
        grid.cols,
        grid.rows,
        geom.width,
        geom.height,
        monitor_origin.x + geom.x,
        monitor_origin.y + geom.y,
    );
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const SW: i32 = 1920;
    const SH: i32 = 1080;

    // --- Grid parsing ---

    #[test]
    fn parse_grid_valid() {
        assert_eq!(WindowGrid::parse("2x1"), Some(WindowGrid::new(2, 1)));
        assert_eq!(WindowGrid::parse("3x2"), Some(WindowGrid::new(3, 2)));
        assert_eq!(WindowGrid::parse("3X3"), Some(WindowGrid::new(3, 3)));
        assert_eq!(WindowGrid::parse(" 2 x 2 "), Some(WindowGrid::new(2, 2)));
    }

    #[test]
    fn parse_grid_invalid() {
        assert_eq!(WindowGrid::parse("abc"), None);
        assert_eq!(WindowGrid::parse("0x2"), None);
        assert_eq!(WindowGrid::parse("2x0"), None);
        assert_eq!(WindowGrid::parse(""), None);
        assert_eq!(WindowGrid::parse("2"), None);
    }

    #[test]
    fn grid_total_slots() {
        assert_eq!(WindowGrid::new(2, 1).total_slots(), 2);
        assert_eq!(WindowGrid::new(2, 2).total_slots(), 4);
        assert_eq!(WindowGrid::new(3, 2).total_slots(), 6);
        assert_eq!(WindowGrid::new(3, 3).total_slots(), 9);
    }

    // --- Grid inference ---

    #[test]
    fn infer_grid_for_duo() {
        assert_eq!(infer_grid(0), WindowGrid::new(2, 1));
        assert_eq!(infer_grid(1), WindowGrid::new(2, 1));
    }

    #[test]
    fn infer_grid_for_quattuor() {
        assert_eq!(infer_grid(2), WindowGrid::new(2, 2));
        assert_eq!(infer_grid(3), WindowGrid::new(2, 2));
    }

    #[test]
    fn infer_grid_for_six() {
        assert_eq!(infer_grid(4), WindowGrid::new(3, 2));
        assert_eq!(infer_grid(5), WindowGrid::new(3, 2));
    }

    #[test]
    fn infer_grid_for_nine() {
        assert_eq!(infer_grid(6), WindowGrid::new(3, 3));
        assert_eq!(infer_grid(8), WindowGrid::new(3, 3));
    }

    // --- 2x1 geometry (duo) ---

    #[test]
    fn grid_2x1_slot_0_left_half() {
        let g = grid_slot_geometry(0, WindowGrid::new(2, 1), SW, SH).unwrap();
        assert_eq!(
            g,
            SlotGeometry {
                x: 0,
                y: 0,
                width: 960,
                height: 1080
            }
        );
    }

    #[test]
    fn grid_2x1_slot_1_right_half() {
        let g = grid_slot_geometry(1, WindowGrid::new(2, 1), SW, SH).unwrap();
        assert_eq!(
            g,
            SlotGeometry {
                x: 960,
                y: 0,
                width: 960,
                height: 1080
            }
        );
    }

    #[test]
    fn grid_2x1_tiles_full_width() {
        let grid = WindowGrid::new(2, 1);
        let g0 = grid_slot_geometry(0, grid, SW, SH).unwrap();
        let g1 = grid_slot_geometry(1, grid, SW, SH).unwrap();
        assert_eq!(g0.width + g1.width, SW as u32);
    }

    #[test]
    fn grid_2x1_out_of_bounds() {
        assert!(grid_slot_geometry(2, WindowGrid::new(2, 1), SW, SH).is_none());
    }

    // --- 2x2 geometry (quattuor) ---

    #[test]
    fn grid_2x2_four_quadrants() {
        let grid = WindowGrid::new(2, 2);
        let slots: Vec<_> = (0..4)
            .map(|s| grid_slot_geometry(s, grid, SW, SH).unwrap())
            .collect();
        // Top-left.
        assert_eq!(
            slots[0],
            SlotGeometry {
                x: 0,
                y: 0,
                width: 960,
                height: 540
            }
        );
        // Top-right.
        assert_eq!(
            slots[1],
            SlotGeometry {
                x: 960,
                y: 0,
                width: 960,
                height: 540
            }
        );
        // Bottom-left.
        assert_eq!(
            slots[2],
            SlotGeometry {
                x: 0,
                y: 540,
                width: 960,
                height: 540
            }
        );
        // Bottom-right.
        assert_eq!(
            slots[3],
            SlotGeometry {
                x: 960,
                y: 540,
                width: 960,
                height: 540
            }
        );
    }

    #[test]
    fn grid_2x2_tiles_full_screen() {
        let grid = WindowGrid::new(2, 2);
        let slots: Vec<_> = (0..4)
            .map(|s| grid_slot_geometry(s, grid, SW, SH).unwrap())
            .collect();
        assert_eq!(slots[0].width + slots[1].width, SW as u32);
        assert_eq!(slots[0].height + slots[2].height, SH as u32);
    }

    // --- 3x2 geometry (six) ---

    #[test]
    fn grid_3x2_six_cells() {
        let grid = WindowGrid::new(3, 2);
        let slots: Vec<_> = (0..6)
            .map(|s| grid_slot_geometry(s, grid, SW, SH).unwrap())
            .collect();
        // Row-major order: 0,1,2 on top row, 3,4,5 on bottom.
        assert_eq!(slots[0].x, 0);
        assert_eq!(slots[1].x, 640);
        assert_eq!(slots[2].x, 1280);
        assert_eq!(slots[3].y, 540);
        assert_eq!(slots[4].y, 540);
        assert_eq!(slots[5].y, 540);
        // All same width/height.
        for s in &slots {
            assert_eq!(s.width, 640);
            assert_eq!(s.height, 540);
        }
    }

    #[test]
    fn grid_3x2_out_of_bounds() {
        assert!(grid_slot_geometry(6, WindowGrid::new(3, 2), SW, SH).is_none());
    }

    // --- 3x3 geometry ---

    #[test]
    fn grid_3x3_nine_cells() {
        let grid = WindowGrid::new(3, 3);
        for slot in 0..9 {
            assert!(grid_slot_geometry(slot, grid, SW, SH).is_some());
        }
        assert!(grid_slot_geometry(9, grid, SW, SH).is_none());

        let g4 = grid_slot_geometry(4, grid, SW, SH).unwrap();
        // Centre cell: col=1, row=1.
        assert_eq!(g4.x, 640);
        assert_eq!(g4.y, 360);
    }

    // --- Monitor offset ---

    #[test]
    fn geometry_is_relative_to_origin() {
        let g = grid_slot_geometry(1, WindowGrid::new(2, 1), SW, SH).unwrap();
        // Position is relative to (0,0) — monitor offset is applied by the system, not geometry.
        assert_eq!(g.x, 960);
        assert_eq!(g.y, 0);
    }

    // --- Screen size resolution ---

    #[test]
    fn resolve_screen_size_prefers_monitor() {
        assert_eq!(resolve_screen_size(Some((3840, 2160))), (3840, 2160));
    }

    #[test]
    fn resolve_screen_size_fallback_is_positive() {
        let (w, h) = resolve_screen_size(None);
        assert!(w > 0 && h > 0);
    }

    // --- Dimension parsing ---

    #[test]
    fn parse_dimensions_valid() {
        assert_eq!(parse_dimensions("1920x1080"), Some((1920, 1080)));
        assert_eq!(parse_dimensions("2560X1440"), Some((2560, 1440)));
        assert_eq!(parse_dimensions(" 3840 x 2160 "), Some((3840, 2160)));
    }

    #[test]
    fn parse_dimensions_invalid() {
        assert_eq!(parse_dimensions("abc"), None);
        assert_eq!(parse_dimensions("0x0"), None);
        assert_eq!(parse_dimensions("-1x100"), None);
        assert_eq!(parse_dimensions(""), None);
    }
}
