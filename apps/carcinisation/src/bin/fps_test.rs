//! Dev-only first-person raycaster test.
//!
//! Renders a Wolf3D-style view of a room loaded from RON.
//! Arrow keys to move/turn. Hold B (Shift) + Left/Right to strafe.
//! A (X) to shoot. Enemies chase and attack.
//!
//! Usage:
//!   cargo run --bin fps_test

use bevy::prelude::*;
use carapace::prelude::*;
use carcinisation_fps::plugin::{
    FpCameraRes, FpConfig, FpMapRes, FpPlayerDead, FpPlugin, FpQuickTurnDebounce, FpQuickTurnState,
    FpShootRequest, FpSystems, move_camera, request_quick_turn, resolve_quick_turn_pressed,
};
use carcinisation_input::{GBInput, init_gb_input};
use leafwing_input_manager::prelude::*;
use serde::{Deserialize, Serialize};

const SCREEN_W: u32 = 160;
const SCREEN_H: u32 = 144;
const MAP_PATH: &str = "../../assets/config/fp/test_room.fp_map.ron";
const MOVE_SPEED: f32 = 2.0;
const TURN_SPEED: f32 = 2.0;

// --- Minimal layer enum for this dev binary ---

#[derive(Deserialize, Reflect, Serialize)]
#[px_layer]
enum FpLayer {
    Background,
    #[default]
    Main,
}

// --- Input system (binary-specific, reads GBInput → updates FP resources) ---

#[allow(clippy::too_many_arguments)]
fn handle_input(
    action: Res<ActionState<GBInput>>,
    time: Res<Time>,
    mut camera: ResMut<FpCameraRes>,
    map: Res<FpMapRes>,
    dead: Res<FpPlayerDead>,
    mut shoot: ResMut<FpShootRequest>,
    mut quick_turn: ResMut<FpQuickTurnDebounce>,
    mut quick_turn_state: ResMut<FpQuickTurnState>,
) {
    if dead.0 {
        return;
    }

    let dt = time.delta_secs();
    let cam = &mut camera.0;
    let b_held = action.pressed(&GBInput::B);
    let back_held = action.pressed(&GBInput::Down);
    let quick_turn_pressed = resolve_quick_turn_pressed(
        back_held,
        b_held,
        action.just_pressed(&GBInput::Down),
        action.just_pressed(&GBInput::B),
        time.elapsed_secs(),
        &mut quick_turn,
    );

    if quick_turn_pressed {
        request_quick_turn(&mut quick_turn_state);
    }

    if !b_held {
        if action.pressed(&GBInput::Left) {
            cam.angle += TURN_SPEED * dt;
        }
        if action.pressed(&GBInput::Right) {
            cam.angle -= TURN_SPEED * dt;
        }
    }

    let dir = cam.direction();
    let right = Vec2::new(dir.y, -dir.x);
    let mut move_delta = Vec2::ZERO;

    if action.pressed(&GBInput::Up) {
        move_delta += dir;
    }
    if back_held && !b_held {
        move_delta -= dir;
    }
    if b_held {
        if action.pressed(&GBInput::Left) {
            move_delta -= right;
        }
        if action.pressed(&GBInput::Right) {
            move_delta += right;
        }
    }

    if move_delta != Vec2::ZERO {
        move_delta = move_delta.normalize() * MOVE_SPEED * dt;
        move_camera(cam, move_delta, &map.0);
    }

    if action.just_pressed(&GBInput::A) {
        shoot.0 = true;
    }
}

fn main() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = std::path::Path::new(manifest_dir).join(MAP_PATH);
    let map_ron = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));

    let mut app = App::new();

    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "FP Test — Arrows: move/turn, Shift+L/R: strafe, X: shoot".into(),
                    resolution: UVec2::new(SCREEN_W * 4, SCREEN_H * 4).into(),
                    ..default()
                }),
                ..default()
            })
            .set(AssetPlugin {
                file_path: "../../assets".into(),
                ..default()
            }),
    );

    app.add_plugins(CxPlugin::<FpLayer>::new(
        UVec2::new(SCREEN_W, SCREEN_H),
        "palette/base.png",
    ));

    app.insert_resource(FpConfig {
        map_ron,
        screen_width: SCREEN_W,
        screen_height: SCREEN_H,
        ..Default::default()
    });
    app.init_resource::<FpQuickTurnDebounce>();
    app.add_plugins(FpPlugin::<FpLayer>::new());

    app.add_plugins(InputManagerPlugin::<GBInput>::default());
    app.add_systems(
        Startup,
        (init_gb_input, |mut commands: Commands| {
            commands.spawn(Camera2d);
        }),
    );
    app.add_systems(Update, handle_input.before(FpSystems));

    app.run();
}
