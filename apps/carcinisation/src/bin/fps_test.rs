//! Dev-only first-person raycaster test.
//!
//! Renders a Wolf3D-style view of a room loaded from RON.
//! Arrow keys to move/turn. Hold B (Shift) + Left/Right to strafe.
//! A (X) to shoot. Enemies chase and attack.
//!
//! Usage:
//!   cargo run --bin `fps_test`

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use carapace::prelude::*;
use carcinisation_fps::data::{EntityKind, MapData};
use carcinisation_fps::enemy::Enemy;
use carcinisation_fps::mosquiton::{Mosquiton, MosquitonConfig};
use carcinisation_fps::player_attack::AttackInput;
use carcinisation_fps::player_attack::PlayerAttackState;
use carcinisation_fps::plugin::{
    CameraRes, CameraShakeState, CharDecals, Config, DeathViewState, EnemySpriteIndex, FpsPlugin,
    MapRes, PlayerDead, PlayerHealth, ProjectileImpacts, Projectiles, QuickTurnDebounce,
    QuickTurnState, ShootRequest, Systems, move_camera, request_quick_turn,
    resolve_quick_turn_pressed,
};
use carcinisation_input::{GBInput, init_gb_input};
use leafwing_input_manager::prelude::*;
use serde::{Deserialize, Serialize};
use std::time::Duration;

const SCREEN_W: u32 = 160;
const SCREEN_H: u32 = 144;
const MAP_PATH: &str = "../../assets/config/fp/test_room.fp_map.ron";
const MOVE_SPEED: f32 = 2.0;
const TURN_SPEED: f32 = 2.0;
const DEATH_RESTART_DELAY_SECS: f32 = 0.75;
const GOD_MODE_DEFAULT: bool = true;

// --- Minimal layer enum for this dev binary ---

#[derive(Deserialize, Reflect, Serialize)]
#[px_layer]
enum Layer {
    Background,
    #[default]
    Main,
}

// --- Input system (binary-specific, reads GBInput → updates FP resources) ---

#[allow(clippy::too_many_arguments)]
#[allow(clippy::needless_pass_by_value)]
fn handle_input(
    action: Res<ActionState<GBInput>>,
    time: Res<Time>,
    mut camera: ResMut<CameraRes>,
    map: Res<MapRes>,
    dead: Res<PlayerDead>,
    mut shoot: ResMut<ShootRequest>,
    mut attack_input: ResMut<AttackInput>,
    mut quick_turn: ResMut<QuickTurnDebounce>,
    mut quick_turn_state: ResMut<QuickTurnState>,
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
    let turning_left = action.pressed(&GBInput::Left) && !b_held;
    let turning_right = action.pressed(&GBInput::Right) && !b_held;
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

    let mut turn_delta = 0.0;
    if turning_left {
        turn_delta += TURN_SPEED * dt;
    }
    if turning_right {
        turn_delta -= TURN_SPEED * dt;
    }
    cam.angle += turn_delta;

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

    let melee_triggered = (select_held && a_just_pressed) || (select_just_pressed && a_held);
    attack_input.cursor_x = SCREEN_W as f32 / 2.0;
    attack_input.aim_turn_velocity = if dt > f32::EPSILON {
        -turn_delta / dt
    } else {
        0.0
    };
    attack_input.strafe_velocity = f32::from(
        i8::from(action.pressed(&GBInput::Right) && b_held)
            - i8::from(action.pressed(&GBInput::Left) && b_held),
    );
    attack_input.melee_triggered = melee_triggered;
    attack_input.cycle_requested = select_just_pressed && !a_held;
    attack_input.moving_forward_back = action.pressed(&GBInput::Up) || (back_held && !b_held);
    attack_input.shoot_just_pressed = a_just_pressed && !select_held;
    attack_input.shoot_held = a_held && !select_held;
    attack_input.shoot_just_released = action.just_released(&GBInput::A);

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
    quick_turn: ResMut<'w, QuickTurnDebounce>,
    quick_turn_state: ResMut<'w, QuickTurnState>,
    restart_gate: ResMut<'w, DeathRestartGate>,
    commands: Commands<'w, 's>,
    enemy_q: Query<'w, 's, Entity, With<Enemy>>,
    mosquiton_q: Query<'w, 's, Entity, With<Mosquiton>>,
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

    // Despawn all existing enemy and mosquiton entities.
    for entity in reset.enemy_q.iter() {
        reset.commands.entity(entity).despawn();
    }
    for entity in reset.mosquiton_q.iter() {
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
            EntityKind::Pillar { .. } => {}
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
    *reset.quick_turn = QuickTurnDebounce::default();
    *reset.quick_turn_state = QuickTurnState::default();
    reset.restart_gate.reset_alive();
}

#[allow(clippy::needless_pass_by_value)]
fn apply_default_god_mode(
    config: Res<Config>,
    mut health: ResMut<PlayerHealth>,
    mut dead: ResMut<PlayerDead>,
) {
    if GOD_MODE_DEFAULT {
        health.0 = config.player_max_health;
        dead.0 = false;
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

    app.add_plugins(CxPlugin::<Layer>::new(
        UVec2::new(SCREEN_W, SCREEN_H),
        "palette/base.png",
    ));

    app.insert_resource(Config {
        map_ron,
        screen_width: SCREEN_W,
        screen_height: SCREEN_H,
        ..Default::default()
    });
    app.init_resource::<DeathRestartGate>();
    app.init_resource::<QuickTurnDebounce>();
    app.add_plugins(FpsPlugin::<Layer>::new());

    app.add_plugins(InputManagerPlugin::<GBInput>::default());
    app.add_systems(
        Startup,
        (init_gb_input, |mut commands: Commands| {
            commands.spawn(Camera2d);
        }),
    );
    app.add_systems(Update, handle_input.before(Systems));
    app.add_systems(Update, apply_default_god_mode.after(Systems));
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
