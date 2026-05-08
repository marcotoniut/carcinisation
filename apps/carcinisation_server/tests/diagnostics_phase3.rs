//! Phase 3 diagnostic tests: map equivalence, boundary sweep, collision radius, input combos.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss
)]

use bevy::math::Vec2;
use carcinisation_fps_core::collision::try_move;
use carcinisation_fps_core::map::Map;
use carcinisation_fps_core::movement::COLLISION_MARGIN;

/// Load the RON map that both server and client default to.
fn load_real_map() -> Map {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../assets/config/fp/test_room.fp_map.ron"
    );
    let ron = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("cannot read {path}: {e}"));
    Map::from_ron(&ron).unwrap_or_else(|e| panic!("cannot parse {path}: {e}"))
}

// ---------------------------------------------------------------------------
// 1. Map equivalence
// ---------------------------------------------------------------------------

/// Server and client must use the same map. Both default to `test_room.fp_map.ron`.
/// This test loads the RON and verifies it matches expected dimensions.
#[test]
fn real_map_loads_and_has_expected_dimensions() {
    let map = load_real_map();

    eprintln!(
        "\n=== Real Map ===\n  {}x{}, {} cells, {} walls",
        map.width,
        map.height,
        map.cells.len(),
        map.cells.iter().filter(|&&c| c > 0).count(),
    );

    assert_eq!(map.width, 12, "expected 12x12 map");
    assert_eq!(map.height, 12);
    assert_eq!(map.cells.len(), 144);
    // Border walls: 4*12 - 4 corners = 44. Plus interior walls.
    let walls = map.cells.iter().filter(|&&c| c > 0).count();
    assert!(
        walls >= 44,
        "expected at least 44 border walls, got {walls}"
    );
}

/// Spawn positions must be inside open cells of the real map.
#[test]
fn spawn_positions_valid_in_real_map() {
    let map = load_real_map();
    let spawns = [
        Vec2::new(1.5, 1.5),
        Vec2::new(6.5, 1.5),
        Vec2::new(1.5, 6.5),
        Vec2::new(6.5, 6.5),
    ];

    for (i, pos) in spawns.iter().enumerate() {
        let cx = pos.x.floor() as i32;
        let cy = pos.y.floor() as i32;
        let cell = map.get(cx, cy);
        assert_eq!(
            cell, 0,
            "Spawn {i} at ({:.1},{:.1}) is in wall cell ({cx},{cy})={cell}",
            pos.x, pos.y
        );
    }
}

// ---------------------------------------------------------------------------
// 2. Boundary sweep on the real map
// ---------------------------------------------------------------------------

#[test]
fn boundary_sweep_real_map() {
    let map = load_real_map();
    let spawn = Vec2::new(1.5, 1.5);
    let step = 0.05_f32;

    let directions: [(&str, Vec2); 4] = [
        ("east", Vec2::new(step, 0.0)),
        ("west", Vec2::new(-step, 0.0)),
        ("north", Vec2::new(0.0, -step)),
        ("south", Vec2::new(0.0, step)),
    ];

    eprintln!(
        "\n=== Boundary Sweep (real map {}x{}, spawn ({:.1},{:.1}), margin={COLLISION_MARGIN}) ===",
        map.width, map.height, spawn.x, spawn.y
    );

    for (name, delta) in &directions {
        let mut pos = spawn;
        let mut steps = 0;
        loop {
            let prev = pos;
            try_move(&mut pos, *delta, COLLISION_MARGIN, &map);
            steps += 1;
            if pos == prev || steps > 500 {
                break;
            }
        }

        let wall_x = pos.x.floor() as i32
            + if delta.x > 0.0 {
                1
            } else if delta.x < 0.0 {
                -1
            } else {
                0
            };
        let wall_y = pos.y.floor() as i32
            + if delta.y > 0.0 {
                1
            } else if delta.y < 0.0 {
                -1
            } else {
                0
            };
        let cell_val = map.get(wall_x, wall_y);

        eprintln!(
            "  {:<6} {:>4} steps | stopped ({:.2},{:.2}) | wall cell ({},{})={cell_val}",
            name, steps, pos.x, pos.y, wall_x, wall_y
        );

        assert!(
            cell_val > 0,
            "{name}: stopped at ({:.2},{:.2}) but cell ({wall_x},{wall_y})=0 (empty)",
            pos.x,
            pos.y
        );
    }
}

// ---------------------------------------------------------------------------
// 3. Collision radius diagnostic
// ---------------------------------------------------------------------------

#[test]
fn collision_radius_diagnostic() {
    let map = load_real_map();
    let spawn = Vec2::new(1.5, 1.5);
    let step = Vec2::new(0.05, 0.0);

    let mut pos_default = spawn;
    for _ in 0..500 {
        let prev = pos_default;
        try_move(&mut pos_default, step, COLLISION_MARGIN, &map);
        if pos_default == prev {
            break;
        }
    }

    let mut pos_tiny = spawn;
    for _ in 0..500 {
        let prev = pos_tiny;
        try_move(&mut pos_tiny, step, 0.01, &map);
        if pos_tiny == prev {
            break;
        }
    }

    let spawn_y = spawn.y.floor() as i32;
    let mut wall_x = spawn.x.floor() as i32 + 1;
    while wall_x < map.width as i32 && map.get(wall_x, spawn_y) == 0 {
        wall_x += 1;
    }

    let gap_default = wall_x as f32 - pos_default.x;
    let gap_tiny = wall_x as f32 - pos_tiny.x;

    eprintln!("\n=== Collision Radius Diagnostic (east from spawn) ===");
    eprintln!("  Wall at x={wall_x} (cell={})", map.get(wall_x, spawn_y));
    eprintln!(
        "  margin={COLLISION_MARGIN}: stop x={:.3}, gap={gap_default:.3}",
        pos_default.x
    );
    eprintln!(
        "  margin=0.01:    stop x={:.3}, gap={gap_tiny:.3}",
        pos_tiny.x
    );

    assert!(
        gap_default < COLLISION_MARGIN + 0.15,
        "stopped too far: gap={gap_default:.3}"
    );
    assert!(gap_default > 0.0, "overlapped wall: gap={gap_default:.3}");
}

// ---------------------------------------------------------------------------
// 4. Input combination table
// Legacy `input_combination_table` test removed — it validated `buttons_to_intent`
// which was replaced by `ClientIntent` semantic protocol. Intent semantics are now
// tested in `sp_mp_parity.rs` and the `PlayerIntentBuffer` unit tests.
