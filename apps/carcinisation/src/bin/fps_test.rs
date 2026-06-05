//! Dev-only first-person raycaster test.
//!
//! Renders a Wolf3D-style view of a room loaded from RON.
//! Arrow keys to move/turn. Legacy: hold B (Shift) + Left/Right to strafe.
//! `AimCommitment`: hold B to aim, A (X) shoots only while aiming.
//! Outside `AimMode`, A alone is reserved/no-op; Select+Down/Left/Right quick/snap turns.
//! Select tap/release switches weapon in `AimCommitment`.
//!
//! Usage:
//!   cargo run --bin `fps_test`
#![allow(clippy::cast_precision_loss, clippy::needless_pass_by_value)]

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
#[cfg(feature = "brp")]
use bevy_brp_extras::BrpExtrasPlugin;
use carapace::prelude::*;
use carcinisation_fps::data::{EntityKind, MapData};
use carcinisation_fps::enemy::Enemy;
use carcinisation_fps::mosquiton::{Mosquiton, MosquitonConfig};
use carcinisation_fps::player_attack::AttackInput;
use carcinisation_fps::player_attack::PlayerAttackState;
use carcinisation_fps::plugin::{
    CameraRes, CameraShakeState, CharDecals, Config, DeathViewState, EnemySpriteIndex, FpsPlugin,
    MapRes, PlayerDead, PlayerHealth, PlayerSpeedModifier, ProjectileImpacts, Projectiles,
    QuickTurnState, SelectActionOutcome, SelectActionTurnInput, SelectActionTurnState,
    ShootRequest, Systems, TurnChordInput, TurnChordState, request_snap_turn,
    resolve_select_action_turn, resolve_turn_chord, select_actions_allowed_outside_aim_mode,
};
use carcinisation_fps::spidey::{Spidey, SpideyConfig};
use carcinisation_input::{GBInput, init_gb_input};
use carcinisation_map_view::MapViewPlugin;
use carcinisation_map_view::MapViewToggle;
use clap::Parser;
use leafwing_input_manager::prelude::*;
use serde::{Deserialize, Serialize};
use std::time::Duration;

const SCREEN_W: u32 = 160;
const SCREEN_H: u32 = 144;
const MAP_PATH: &str = "../../assets/config/fp/test_room.fp_map.ron";
const SKY_PATH: &str = "../../assets/config/sky/park.sky.ron";
const DEATH_RESTART_DELAY_SECS: f32 = 0.75;
const GOD_MODE_ENV: &str = "CARCINISATION_GOD_MODE";

/// Debug god mode resource — mirrors `DebugGodMode` from the main app debug plugin.
#[derive(Resource)]
struct GodMode {
    enabled: bool,
}

fn load_initial_god_mode() -> bool {
    std::env::var(GOD_MODE_ENV).map_or(true, |value| {
        match value.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => true,
            "0" | "false" | "no" | "off" => false,
            _ => {
                warn!("{GOD_MODE_ENV} unrecognised value '{value}'; defaulting to true");
                true
            }
        }
    })
}

// --- Minimal layer enum for this dev binary ---

#[derive(Deserialize, Reflect, Serialize)]
#[px_layer]
enum Layer {
    Background,
    #[default]
    Main,
    MapView,
    MapViewOverlay,
}

// --- Input system (binary-specific, reads GBInput → updates FP resources) ---

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
#[allow(clippy::needless_pass_by_value)]
fn handle_input(
    action: Res<ActionState<GBInput>>,
    time: Res<Time>,
    mut camera: ResMut<CameraRes>,
    map: Res<MapRes>,
    config: Res<Config>,
    movement_config: Res<carcinisation_fps_core::FpsMovementConfig>,
    combat_config: Res<carcinisation_fps_core::FpsCombatConfig>,
    dead: Res<PlayerDead>,
    mut shoot: ResMut<ShootRequest>,
    mut attack_input: ResMut<AttackInput>,
    mut turn_chord: ResMut<TurnChordState>,
    mut select_action_turn: ResMut<SelectActionTurnState>,
    mut quick_turn_state: ResMut<QuickTurnState>,
    mut speed_modifier: ResMut<PlayerSpeedModifier>,
) {
    if dead.0 {
        return;
    }

    let dt = time.delta_secs();
    let cam = &mut camera.0;
    let b_held = action.pressed(&GBInput::B);
    let back_held = action.pressed(&GBInput::Down);
    let a_held = action.pressed(&GBInput::A);
    let a_just_pressed = action.just_pressed(&GBInput::A);
    let select_held = action.pressed(&GBInput::Select);
    let select_just_pressed = action.just_pressed(&GBInput::Select);
    let select_just_released = action.just_released(&GBInput::Select);
    let left_held = action.pressed(&GBInput::Left);
    let right_held = action.pressed(&GBInput::Right);
    let turning_left = left_held && !b_held;
    let turning_right = right_held && !b_held;
    let up_held = action.pressed(&GBInput::Up);

    let aim_commitment = matches!(
        combat_config.combat_control_mode,
        carcinisation_fps_core::CombatControlMode::AimCommitment
    );

    let chord_input = TurnChordInput {
        b_pressed: b_held,
        b_just_pressed: action.just_pressed(&GBInput::B),
        b_just_released: action.just_released(&GBInput::B),
        down_pressed: back_held,
        down_just_pressed: action.just_pressed(&GBInput::Down),
        down_just_released: action.just_released(&GBInput::Down),
        left_pressed: action.pressed(&GBInput::Left),
        left_just_pressed: action.just_pressed(&GBInput::Left),
        left_just_released: action.just_released(&GBInput::Left),
        right_pressed: action.pressed(&GBInput::Right),
        right_just_pressed: action.just_pressed(&GBInput::Right),
        right_just_released: action.just_released(&GBInput::Right),
        now_secs: time.elapsed_secs(),
    };

    // Legacy keeps B+direction snap turns. AimCommitment uses B as immediate
    // aim; quick/snap turn moves to Select+direction outside AimMode.
    if let Some(kind) = resolve_turn_chord(&chord_input, &mut turn_chord, aim_commitment) {
        let blocked = up_held
            || (matches!(
                kind,
                carcinisation_fps::plugin::TurnKind::SideTurnLeft
                    | carcinisation_fps::plugin::TurnKind::SideTurnRight
            ) && back_held);
        if !blocked {
            request_snap_turn(&mut quick_turn_state, kind, &config);
        }
    }

    let in_aim_mode = aim_commitment && turn_chord.is_aim_mode();

    let select_action = if aim_commitment {
        resolve_select_action_turn(
            &SelectActionTurnInput {
                select_pressed: select_held,
                select_just_pressed,
                select_just_released,
                down_pressed: back_held,
                down_just_pressed: action.just_pressed(&GBInput::Down),
                left_pressed: left_held,
                left_just_pressed: action.just_pressed(&GBInput::Left),
                right_pressed: right_held,
                right_just_pressed: action.just_pressed(&GBInput::Right),
                now_secs: time.elapsed_secs(),
            },
            &mut select_action_turn,
            select_actions_allowed_outside_aim_mode(in_aim_mode),
        )
    } else {
        None
    };

    let action_turn = match select_action {
        Some(SelectActionOutcome::SnapTurn(kind)) => Some(kind),
        _ => None,
    };

    if let Some(kind) = action_turn {
        request_snap_turn(&mut quick_turn_state, kind, &config);
    }

    // -- Movement / Turn / Aim (branched on AimCommitment) --
    let turn_delta;
    let movement;

    if in_aim_mode {
        // AimCommitment: feet locked, body turns freely, Up/Down = visual pitch.
        movement = Vec2::ZERO;

        // Turn uses configurable aim_turn_speed for steadier aiming.
        let turn_animating = quick_turn_state.is_active();
        let mut td = 0.0;
        if left_held && !turn_animating {
            td += combat_config.aim_turn_speed * dt;
        }
        if right_held && !turn_animating {
            td -= combat_config.aim_turn_speed * dt;
        }
        turn_delta = td;

        // Vertical pitch (visual-only).
        quick_turn_state.update_aim_pitch(
            action.pressed(&GBInput::Up),
            back_held,
            combat_config.aim_pitch_speed,
            dt,
        );
    } else {
        // LEGACY(strafe) or not-yet-aiming: normal movement/turn.
        quick_turn_state.reset_aim_pitch_offset();

        // Suppress manual turning while a snap turn animation is active.
        let turn_animating = quick_turn_state.is_active();
        let mut td = 0.0;
        if turning_left && !turn_animating {
            td += config.turn_speed * dt;
        }
        if turning_right && !turn_animating {
            td -= config.turn_speed * dt;
        }
        turn_delta = td;

        // Build local-space movement intent (matches MP client → server path).
        let mut mv = Vec2::ZERO;
        if action_turn.is_none() {
            if action.pressed(&GBInput::Up) {
                mv.y += 1.0;
            }
            if back_held {
                mv.y -= 1.0;
            }
        }
        // LEGACY(strafe): B + Left/Right = strafe. Only in Legacy mode.
        if b_held && !aim_commitment {
            if action.pressed(&GBInput::Left) {
                mv.x -= 1.0;
            }
            if action.pressed(&GBInput::Right) {
                mv.x += 1.0;
            }
        }
        if mv.length_squared() > 1.0 {
            mv = mv.normalize();
        }
        movement = mv;
    }

    cam.angle += turn_delta;

    let pos_before = cam.position;
    if movement != Vec2::ZERO {
        carcinisation_fps_core::movement::apply_movement_with_modifier(
            &mut cam.position,
            cam.angle,
            movement,
            config.move_speed,
            speed_modifier.0.as_ref(),
            dt,
            &map.0,
            movement_config.collision_margin,
        );
    }
    // Tick speed modifier with actual movement distance so moving drains it faster.
    if let Some(ref mut modifier) = speed_modifier.0 {
        let move_dist = cam.position.distance(pos_before);
        if !modifier.tick(dt, move_dist) {
            speed_modifier.0 = None;
        }
    }

    // AimCommitment: cannot fire without aiming. Legacy: fire anytime.
    let fire_allowed = if aim_commitment { in_aim_mode } else { true };

    let melee_triggered =
        !aim_commitment && ((select_held && a_just_pressed) || (select_just_pressed && a_held));
    attack_input.cursor_x = SCREEN_W as f32 / 2.0;
    attack_input.aim_turn_velocity = if dt > f32::EPSILON {
        -turn_delta / dt
    } else {
        0.0
    };
    attack_input.strafe_velocity = if in_aim_mode {
        0.0
    } else {
        f32::from(
            i8::from(action.pressed(&GBInput::Right) && b_held)
                - i8::from(action.pressed(&GBInput::Left) && b_held),
        )
    };
    attack_input.melee_triggered = melee_triggered;
    attack_input.aim_held = in_aim_mode;
    attack_input.cycle_requested = if aim_commitment {
        matches!(select_action, Some(SelectActionOutcome::WeaponSwitch))
    } else {
        select_just_pressed && !a_held
    };
    attack_input.moving_forward_back = if in_aim_mode {
        false
    } else {
        action.pressed(&GBInput::Up) || back_held
    };
    attack_input.shoot_just_pressed = fire_allowed && a_just_pressed && !select_held;
    attack_input.shoot_held = fire_allowed && a_held && !select_held;
    attack_input.shoot_just_released = fire_allowed && action.just_released(&GBInput::A);

    if attack_input.shoot_just_pressed {
        shoot.0 = true;
    }
}

fn restart_pressed(action: &ActionState<GBInput>) -> bool {
    [
        GBInput::A,
        GBInput::B,
        GBInput::Up,
        GBInput::Down,
        GBInput::Left,
        GBInput::Right,
        GBInput::Start,
        GBInput::Select,
    ]
    .iter()
    .any(|button| action.just_pressed(button))
}

#[derive(Resource)]
struct DeathRestartGate {
    timer: Timer,
    was_dead: bool,
}

impl Default for DeathRestartGate {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(DEATH_RESTART_DELAY_SECS, TimerMode::Once),
            was_dead: false,
        }
    }
}

impl DeathRestartGate {
    fn tick(&mut self, dead: bool, delta: Duration) {
        if dead && !self.was_dead {
            self.timer.reset();
        }
        if dead {
            self.timer.tick(delta);
        }
        self.was_dead = dead;
    }

    fn accepts_restart(&self) -> bool {
        self.was_dead && self.timer.is_finished()
    }

    fn reset_alive(&mut self) {
        self.was_dead = false;
        self.timer.reset();
    }
}

#[derive(SystemParam)]
struct ResetParams<'w, 's> {
    config: Res<'w, Config>,
    camera: ResMut<'w, CameraRes>,
    health: ResMut<'w, PlayerHealth>,
    dead: ResMut<'w, PlayerDead>,
    shoot: ResMut<'w, ShootRequest>,
    attack_input: ResMut<'w, AttackInput>,
    attack_state: ResMut<'w, PlayerAttackState>,
    projectiles: ResMut<'w, Projectiles>,
    impacts: ResMut<'w, ProjectileImpacts>,
    char_decals: ResMut<'w, CharDecals>,
    death_view: ResMut<'w, DeathViewState>,
    camera_shake: ResMut<'w, CameraShakeState>,
    turn_chord: ResMut<'w, TurnChordState>,
    quick_turn_state: ResMut<'w, QuickTurnState>,
    restart_gate: ResMut<'w, DeathRestartGate>,
    commands: Commands<'w, 's>,
    enemy_q: Query<'w, 's, Entity, With<Enemy>>,
    mosquiton_q: Query<'w, 's, Entity, With<Mosquiton>>,
    spidey_q: Query<'w, 's, Entity, With<Spidey>>,
}

#[allow(clippy::needless_pass_by_value)]
fn reset_on_dead_input(action: Res<ActionState<GBInput>>, time: Res<Time>, mut reset: ResetParams) {
    let dead = reset.dead.0;
    reset.restart_gate.tick(dead, time.delta());
    if dead && reset.restart_gate.accepts_restart() && restart_pressed(&action) {
        reset_stage(&mut reset);
    }
}

fn reset_stage(reset: &mut ResetParams<'_, '_>) {
    let map_data = MapData::from_ron(&reset.config.map_ron)
        .unwrap_or_else(|e| panic!("failed to reset FP map: {e}"));

    // Despawn all existing enemy, mosquiton, and spidey entities.
    for entity in reset.enemy_q.iter() {
        reset.commands.entity(entity).despawn();
    }
    for entity in reset.mosquiton_q.iter() {
        reset.commands.entity(entity).despawn();
    }
    for entity in reset.spidey_q.iter() {
        reset.commands.entity(entity).despawn();
    }

    // Clear transient resources.
    reset.projectiles.0.clear();
    reset.impacts.0.clear();
    reset.char_decals.0.clear();

    // Spawn new enemies and mosquitons from map data.
    for spawn in &map_data.entities {
        let pos = Vec2::new(spawn.x, spawn.y);
        match &spawn.kind {
            EntityKind::Enemy { health, speed, .. }
            | EntityKind::SpriteEnemy { health, speed, .. } => {
                let enemy = Enemy::new(pos, *health, *speed);
                reset.commands.spawn((enemy, EnemySpriteIndex(0)));
            }
            EntityKind::Mosquiton { health, speed } => {
                let config = MosquitonConfig {
                    health: *health,
                    move_speed: *speed,
                    ..Default::default()
                };
                let mosquiton = Mosquiton::new(pos, config);
                reset.commands.spawn(mosquiton);
            }
            EntityKind::Pillar { .. } | EntityKind::Pickup { .. } => {}
            EntityKind::Spidey { health, speed } => {
                let combat = carcinisation_fps_core::FpsCombatConfig::load();
                let config = SpideyConfig {
                    health: *health,
                    ..SpideyConfig::from_combat_config(&combat)
                }
                .with_authored_speed(*speed);
                let spidey = Spidey::new(pos, config);
                reset.commands.spawn(spidey);
            }
        }
    }

    // Reset remaining resources.
    reset.camera.0 = map_data.to_camera();
    reset.health.0 = reset.config.player_max_health;
    reset.dead.0 = false;
    reset.shoot.0 = false;
    *reset.attack_input = AttackInput::default();
    *reset.attack_state = PlayerAttackState::default();
    *reset.death_view = DeathViewState::default();
    *reset.camera_shake = CameraShakeState::default();
    *reset.turn_chord = TurnChordState::default();
    *reset.quick_turn_state = QuickTurnState::default();
    reset.restart_gate.reset_alive();
}

#[allow(clippy::needless_pass_by_value)]
fn apply_god_mode(
    config: Res<Config>,
    god_mode: Res<GodMode>,
    mut health: ResMut<PlayerHealth>,
    mut dead: ResMut<PlayerDead>,
) {
    if god_mode.enabled {
        health.0 = config.player_max_health;
        dead.0 = false;
    }
}

fn toggle_god_mode(keys: Res<ButtonInput<KeyCode>>, mut god_mode: ResMut<GodMode>) {
    let modifier_held = keys.any_pressed([
        KeyCode::ShiftLeft,
        KeyCode::ShiftRight,
        KeyCode::SuperLeft,
        KeyCode::SuperRight,
        KeyCode::ControlLeft,
        KeyCode::ControlRight,
    ]);
    if modifier_held && keys.just_pressed(KeyCode::KeyG) {
        god_mode.enabled = !god_mode.enabled;
        info!(
            "God mode {}",
            if god_mode.enabled {
                "enabled"
            } else {
                "disabled"
            }
        );
    }
}

#[derive(Parser, Debug)]
struct Args {
    /// Start with automap enabled.
    #[arg(long)]
    map_view: bool,
}

fn main() {
    let args = Args::parse();
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let map_path = std::path::Path::new(manifest_dir).join(MAP_PATH);
    let map_ron = std::fs::read_to_string(&map_path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", map_path.display()));
    let sky_path = std::path::Path::new(manifest_dir)
        .join(SKY_PATH)
        .to_string_lossy()
        .to_string();

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

    app.add_plugins(CxPlugin::<Layer>::new(
        UVec2::new(SCREEN_W, SCREEN_H),
        "palette/base.png",
    ));

    app.insert_resource(Config {
        map_ron,
        map_path: map_path.to_string_lossy().to_string(),
        sky_path,
        screen_width: SCREEN_W,
        screen_height: SCREEN_H,
        ..Default::default()
    });
    app.init_resource::<DeathRestartGate>();
    app.insert_resource(GodMode {
        enabled: load_initial_god_mode(),
    });
    if args.map_view {
        app.insert_resource(MapViewToggle::new(true));
    }

    app.add_plugins(FpsPlugin::<Layer>::new());
    app.add_plugins(MapViewPlugin::new(Layer::MapView, Layer::MapViewOverlay));

    #[cfg(feature = "brp")]
    if !app.is_plugin_added::<BrpExtrasPlugin>() {
        app.add_plugins(BrpExtrasPlugin);
    }

    app.add_plugins(InputManagerPlugin::<GBInput>::default());
    app.add_systems(
        Startup,
        (init_gb_input, |mut commands: Commands| {
            commands.spawn(Camera2d);
        }),
    );
    app.add_systems(Update, handle_input.before(Systems));
    app.add_systems(Update, toggle_god_mode);
    app.add_systems(Update, apply_god_mode.after(Systems));
    app.add_systems(Update, reset_on_dead_input.after(Systems));

    app.run();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn death_restart_gate_requires_delay_after_death() {
        let mut gate = DeathRestartGate::default();

        gate.tick(
            true,
            Duration::from_secs_f32(DEATH_RESTART_DELAY_SECS * 0.5),
        );
        assert!(!gate.accepts_restart());

        gate.tick(true, Duration::from_secs_f32(DEATH_RESTART_DELAY_SECS));
        assert!(gate.accepts_restart());

        gate.reset_alive();
        assert!(!gate.accepts_restart());
    }
}
