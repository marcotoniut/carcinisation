//! First-person player attacks and weapon overlays.

use bevy::prelude::{Reflect, ReflectResource, Resource, Vec2};
use carapace::{image::CxImage, palette::TRANSPARENT_INDEX};
use carcinisation_base::fire_death::{DamageKind, FireDeathConfig};
use flate2::read::DeflateDecoder;
use serde::Deserialize;
use std::{
    collections::HashMap,
    io::{Cursor, Read},
    time::Duration,
};

use crate::{
    camera::Camera,
    enemy::{Enemy, Projectile, ProjectileImpact, hitscan, hitscan_projectiles},
    map::Map,
    mosquiton::{Mosquiton, hitscan_mosquitons},
    raycast::{WallSurfaceId, cast_ray},
    render::{CharDecal, WallSurfaceSprite},
};

const PLAYER_BULLET_ATLAS_RON: &str =
    include_str!("../../../assets/sprites/attacks/player_bullet/atlas.px_atlas.ron");
const PLAYER_BULLET_PXI: &[u8] =
    include_bytes!("../../../assets/sprites/attacks/player_bullet/atlas.pxi");
const PLAYER_MELEE_ATLAS_RON: &str =
    include_str!("../../../assets/sprites/attacks/player_melee/atlas.px_atlas.ron");
const PLAYER_MELEE_PXI: &[u8] =
    include_bytes!("../../../assets/sprites/attacks/player_melee/atlas.pxi");
const PLAYER_FLAME_ATLAS_RON: &str =
    include_str!("../../../assets/sprites/attacks/player_flame/atlas.px_atlas.ron");
const PLAYER_FLAME_PXI: &[u8] =
    include_bytes!("../../../assets/sprites/attacks/player_flame/atlas.pxi");
const PLAYER_FLAME_WALL_ATLAS_RON: &str =
    include_str!("../../../assets/sprites/attacks/player_flame_wall/atlas.px_atlas.ron");
const PLAYER_FLAME_WALL_PXI: &[u8] =
    include_bytes!("../../../assets/sprites/attacks/player_flame_wall/atlas.pxi");
const STAGE_WEAPON_ATLAS_RON: &str =
    include_str!("../../../assets/sprites/ui/stage_flamethrower_weapon/atlas.px_atlas.ron");
const STAGE_WEAPON_PXI: &[u8] =
    include_bytes!("../../../assets/sprites/ui/stage_flamethrower_weapon/atlas.pxi");
const STAGE_WEAPON_SHOOTING_ATLAS_RON: &str = include_str!(
    "../../../assets/sprites/ui/stage_flamethrower_weapon_shooting/atlas.px_atlas.ron"
);
const STAGE_WEAPON_SHOOTING_PXI: &[u8] =
    include_bytes!("../../../assets/sprites/ui/stage_flamethrower_weapon_shooting/atlas.pxi");
const STAGE_IDLE_FLAME_ATLAS_RON: &str =
    include_str!("../../../assets/sprites/ui/stage_flamethrower_flame/atlas.px_atlas.ron");
const STAGE_IDLE_FLAME_PXI: &[u8] =
    include_bytes!("../../../assets/sprites/ui/stage_flamethrower_flame/atlas.pxi");
const FLAMETHROWER_CONFIG_RON: &str =
    include_str!("../../../assets/config/attacks/player_flamethrower_fps.ron");

const BULLET_REGION: &str = "bullet_particles";
const MELEE_REGION: &str = "melee_slash";
const FLAME_REGION: &str = "flame";
const FLAME_WALL_HIT_REGION: &str = "flame_wall_hit";
const FLAMETHROWER_IDLE_REGION: &str = "flamethrower_idle";
const FLAMETHROWER_SHOOTING_REGION: &str = "flamethrower_shooting";
const STAGE_IDLE_FLAME_REGION: &str = "flamethrower_flame";
const PISTOL_EFFECT_POS: Vec2 = Vec2::new(80.0, 72.0);
const MELEE_EFFECT_POS: Vec2 = Vec2::new(80.0, 72.0);
const MELEE_RANGE_UNITS: f32 = 1.1;
const FLAME_RANGE_UNITS: f32 = 2.8;
const FLAME_WALL_IMPACT_WIDTH: f32 = 0.30;
const FLAME_WALL_IMPACT_HEIGHT: f32 = 0.30;
const FLAME_CHAR_DECAL_WIDTH: f32 = FLAME_WALL_IMPACT_WIDTH;
const FLAME_CHAR_DECAL_HEIGHT: f32 = FLAME_WALL_IMPACT_HEIGHT;
const FLAME_WALL_BRIDGE_BACKOFF_UNITS: f32 = 0.10;
const FLAME_WALL_BRIDGE_MIN_GAP_UNITS: f32 = 0.12;
const FLAME_WALL_BRIDGE_SCALE: f32 = 0.55;
const FLAME_WALL_BRIDGE_MAX_SCALE: f32 = 0.85;
const MAX_FLAME_CHAR_DECALS: usize = 128;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Reflect)]
pub enum AttackId {
    Pistol,
    Flamethrower,
}

#[derive(Resource, Debug, Reflect)]
#[reflect(Resource)]
pub struct AttackLoadout {
    pub options: Vec<AttackId>,
    pub index: usize,
}

impl Default for AttackLoadout {
    fn default() -> Self {
        Self {
            options: vec![AttackId::Flamethrower, AttackId::Pistol],
            index: 0,
        }
    }
}

impl AttackLoadout {
    #[must_use]
    pub fn current(&self) -> AttackId {
        self.options[self.index]
    }

    pub fn cycle(&mut self) -> AttackId {
        self.index = (self.index + 1) % self.options.len();
        self.current()
    }
}

#[derive(Resource, Clone, Copy, Debug, Reflect)]
#[reflect(Resource)]
pub struct AttackInput {
    pub shoot_just_pressed: bool,
    pub shoot_held: bool,
    pub shoot_just_released: bool,
    pub melee_triggered: bool,
    pub cycle_requested: bool,
    pub moving_forward_back: bool,
    pub cursor_x: f32,
    pub aim_turn_velocity: f32,
    pub strafe_velocity: f32,
}

impl Default for AttackInput {
    fn default() -> Self {
        Self {
            shoot_just_pressed: false,
            shoot_held: false,
            shoot_just_released: false,
            melee_triggered: false,
            cycle_requested: false,
            moving_forward_back: false,
            cursor_x: 80.0,
            aim_turn_velocity: 0.0,
            strafe_velocity: 0.0,
        }
    }
}

impl AttackInput {
    pub fn clear_edges(&mut self) {
        self.shoot_just_pressed = false;
        self.shoot_just_released = false;
        self.melee_triggered = false;
        self.cycle_requested = false;
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename = "FlamethrowerConfig")]
pub struct FlamethrowerConfig {
    pub max_flames: u8,
    pub chain_range: f32,
    pub spacing_curve: f32,
    pub flame_speed: f32,
    pub drain_speed_multiplier: f32,
    pub spawn_interval_ms: u64,
    pub damage_per_tick: u32,
    pub tick_ms: u64,
    pub max_ammo: f32,
    pub ammo_drain_per_ms: f32,
    pub origin_camera_px: (f32, f32),
    #[serde(rename = "hit_radius")]
    pub hit_radius_px: f32,
    pub scale_near: f32,
    pub scale_far: f32,
    pub weapon_base_offset_px: (f32, f32),
    pub weapon_bob_enabled: bool,
    pub weapon_bob_horizontal_px: f32,
    pub weapon_bob_vertical_px: f32,
    pub weapon_bob_speed: f32,
    pub weapon_bob_return_speed: f32,
    pub idle_flame_offset: (f32, f32),
    pub idle_flame_scale: f32,
    pub flame_nozzle_offset: (f32, f32),
    pub flame_start_offset_px: f32,
    pub flame_far_vertical_drop_px: f32,
    pub flame_vertical_curve_power: f32,
    pub turn_bend_strength: f32,
    pub turn_bend_return_speed: f32,
    pub turn_bend_index_power: f32,
    pub strafe_bend_strength: f32,
    pub burning_corpse_duration_secs: f32,
    pub burning_corpse_contact_damage: u32,
    pub burning_corpse_contact_tick_ms: u64,
    pub burning_corpse_contact_radius: f32,
    pub burning_corpse_crossfire_damage: u32,
    pub burning_flame_count: usize,
    pub burning_flame_perimeter_padding_px: f32,
    pub burning_flame_jitter_px: f32,
    pub burning_flame_scale_min: f32,
    pub burning_flame_scale_max: f32,
}

impl FlamethrowerConfig {
    #[must_use]
    pub fn load() -> Self {
        ron::from_str(FLAMETHROWER_CONFIG_RON).expect("embedded player_flamethrower.ron must parse")
    }

    #[must_use]
    pub fn origin(&self) -> Vec2 {
        Vec2::new(self.origin_camera_px.0, self.origin_camera_px.1)
    }

    #[must_use]
    pub fn weapon_base_offset(&self) -> Vec2 {
        Vec2::new(self.weapon_base_offset_px.0, self.weapon_base_offset_px.1)
    }

    #[must_use]
    pub fn idle_flame_offset(&self) -> (f32, f32) {
        self.idle_flame_offset
    }

    #[must_use]
    pub fn flame_nozzle_offset(&self) -> (f32, f32) {
        self.flame_nozzle_offset
    }

    #[must_use]
    pub fn slot_target(&self, slot: u8) -> f32 {
        let t = (f32::from(slot) + 1.0) / f32::from(self.max_flames);
        self.chain_range * t.powf(self.spacing_curve)
    }

    #[must_use]
    pub fn segment_scale(&self, progress: f32) -> f32 {
        let t = (progress / self.chain_range).clamp(0.0, 1.0);
        self.scale_near + (self.scale_far - self.scale_near) * t
    }

    #[must_use]
    pub fn hit_radius_units(&self) -> f32 {
        local_flame_offset(Vec2::splat(self.hit_radius_px), self).x
    }

    #[must_use]
    pub fn fire_death_config(&self) -> FireDeathConfig {
        FireDeathConfig {
            burning_corpse_duration_secs: self.burning_corpse_duration_secs,
            burning_flame_count: self.burning_flame_count,
            burning_flame_perimeter_padding_px: self.burning_flame_perimeter_padding_px,
            burning_flame_jitter_px: self.burning_flame_jitter_px,
            burning_flame_scale_min: self.burning_flame_scale_min,
            burning_flame_scale_max: self.burning_flame_scale_max,
        }
    }

    #[must_use]
    pub fn burning_corpse_contact_tick_secs(&self) -> f32 {
        Duration::from_millis(self.burning_corpse_contact_tick_ms).as_secs_f32()
    }
}

#[derive(Clone, Debug)]
struct AtlasAnimation {
    frames: Vec<CxImage>,
    duration_secs: f32,
}

impl AtlasAnimation {
    fn frame_loop(&self, elapsed_secs: f32) -> &CxImage {
        let len = self.frames.len();
        if len == 1 || self.duration_secs <= f32::EPSILON {
            return &self.frames[0];
        }
        let t = (elapsed_secs / self.duration_secs).fract();
        let index = ((t * len as f32) as usize).min(len - 1);
        &self.frames[index]
    }

    fn frame_clamped(&self, elapsed_secs: f32) -> &CxImage {
        let len = self.frames.len();
        if len == 1 || self.duration_secs <= f32::EPSILON {
            return &self.frames[0];
        }
        let t = (elapsed_secs / self.duration_secs).clamp(0.0, 0.999);
        let index = ((t * len as f32) as usize).min(len - 1);
        &self.frames[index]
    }
}

#[derive(Resource, Clone, Debug)]
pub struct PlayerAttackSprites {
    bullet: AtlasAnimation,
    melee: AtlasAnimation,
    flame: AtlasAnimation,
    flame_wall_hit: AtlasAnimation,
    weapon_idle: AtlasAnimation,
    weapon_shooting: AtlasAnimation,
    idle_flame: AtlasAnimation,
}

impl PlayerAttackSprites {
    #[must_use]
    pub fn load() -> Self {
        Self {
            bullet: load_atlas_animation(PLAYER_BULLET_ATLAS_RON, PLAYER_BULLET_PXI, BULLET_REGION)
                .expect("player bullet atlas must load"),
            melee: load_atlas_animation(PLAYER_MELEE_ATLAS_RON, PLAYER_MELEE_PXI, MELEE_REGION)
                .expect("player melee atlas must load"),
            flame: load_atlas_animation(PLAYER_FLAME_ATLAS_RON, PLAYER_FLAME_PXI, FLAME_REGION)
                .expect("player flame atlas must load"),
            flame_wall_hit: load_atlas_animation(
                PLAYER_FLAME_WALL_ATLAS_RON,
                PLAYER_FLAME_WALL_PXI,
                FLAME_WALL_HIT_REGION,
            )
            .expect("player flame wall hit atlas must load"),
            weapon_idle: load_atlas_animation(
                STAGE_WEAPON_ATLAS_RON,
                STAGE_WEAPON_PXI,
                FLAMETHROWER_IDLE_REGION,
            )
            .expect("stage flamethrower idle weapon atlas must load"),
            weapon_shooting: load_atlas_animation(
                STAGE_WEAPON_SHOOTING_ATLAS_RON,
                STAGE_WEAPON_SHOOTING_PXI,
                FLAMETHROWER_SHOOTING_REGION,
            )
            .expect("stage flamethrower shooting weapon atlas must load"),
            idle_flame: load_atlas_animation(
                STAGE_IDLE_FLAME_ATLAS_RON,
                STAGE_IDLE_FLAME_PXI,
                STAGE_IDLE_FLAME_REGION,
            )
            .expect("stage flamethrower idle flame atlas must load"),
        }
    }

    #[must_use]
    pub fn flame_frame_loop(&self, elapsed_secs: f32) -> &CxImage {
        self.flame.frame_loop(elapsed_secs)
    }
}

#[derive(Resource, Debug)]
pub struct PlayerAttackState {
    one_shots: Vec<OneShotEffect>,
    flamethrower: Option<ActiveFpFlamethrower>,
    weapon_bob_offset: Vec2,
    config: FlamethrowerConfig,
}

impl Default for PlayerAttackState {
    fn default() -> Self {
        Self {
            one_shots: Vec::new(),
            flamethrower: None,
            weapon_bob_offset: Vec2::ZERO,
            config: FlamethrowerConfig::load(),
        }
    }
}

impl PlayerAttackState {
    #[must_use]
    pub fn config(&self) -> &FlamethrowerConfig {
        &self.config
    }
}

#[derive(Clone, Debug)]
struct OneShotEffect {
    kind: OneShotEffectKind,
    elapsed: f32,
    position: Vec2,
}

#[derive(Clone, Copy, Debug)]
enum OneShotEffectKind {
    Bullet,
    Melee,
}

#[derive(Clone, Debug)]
struct ActiveFpFlamethrower {
    spawning: bool,
    ammo: f32,
    next_spawn_at: f32,
    next_slot: u8,
    elapsed: f32,
    desired_direction: Vec2,
    previous_aim_angle: f32,
    segments: Vec<FpFlameSegment>,
    tick_state: HashMap<FpFlameTarget, f32>,
    wall_impact: Option<FlameWallImpact>,
    last_decal_impact: Option<FlameWallImpact>,
}

#[derive(Clone, Debug)]
struct FpFlameSegment {
    progress: f32,
    target: f32,
    bend_px: f32,
    slot: u8,
}

#[derive(Clone, Copy, Debug)]
struct FlameCollisionPoint {
    local: Vec2,
}

#[derive(Clone, Copy, Debug)]
struct FlamePoint {
    local: Vec2,
    screen_offset: Vec2,
    visual_base_y: f32,
    progress: f32,
    slot: u8,
    wall_impact: Option<FlameWallImpact>,
}

#[derive(Clone, Copy, Debug)]
struct FlameWallBridge {
    screen_offset: Vec2,
    progress: f32,
}

#[derive(Clone, Copy, Debug)]
struct FlameWallImpact {
    surface_id: WallSurfaceId,
    u: f32,
    v: f32,
    seed: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum FpFlameTarget {
    Enemy(usize),
    Mosquiton(usize),
}

#[allow(clippy::too_many_arguments)]
pub fn process_player_attacks(
    camera: &Camera,
    map: &Map,
    hitscan_damage: u32,
    dt: f32,
    elapsed_secs: f32,
    input: &mut AttackInput,
    loadout: &mut AttackLoadout,
    state: &mut PlayerAttackState,
    enemies: &mut [Enemy],
    mosquitons: &mut [Mosquiton],
    projectiles: &mut Vec<Projectile>,
    impacts: &mut Vec<ProjectileImpact>,
    char_decals: &mut Vec<CharDecal>,
    screen_height_px: f32,
    legacy_shoot_request: &mut bool,
) {
    if input.cycle_requested {
        loadout.cycle();
    }

    let legacy_shot = *legacy_shoot_request;
    *legacy_shoot_request = false;

    if input.melee_triggered {
        state.one_shots.push(OneShotEffect {
            kind: OneShotEffectKind::Melee,
            elapsed: 0.0,
            position: MELEE_EFFECT_POS,
        });
        apply_hitscan_damage(
            camera,
            map,
            enemies,
            mosquitons,
            projectiles,
            impacts,
            hitscan_damage.saturating_mul(3),
            Some(MELEE_RANGE_UNITS),
        );
    } else {
        match loadout.current() {
            AttackId::Pistol => {
                if input.shoot_just_pressed || legacy_shot {
                    state.one_shots.push(OneShotEffect {
                        kind: OneShotEffectKind::Bullet,
                        elapsed: 0.0,
                        position: PISTOL_EFFECT_POS,
                    });
                    apply_hitscan_damage(
                        camera,
                        map,
                        enemies,
                        mosquitons,
                        projectiles,
                        impacts,
                        hitscan_damage,
                        None,
                    );
                }
            }
            AttackId::Flamethrower => update_flamethrower_attack(
                camera,
                map,
                dt,
                elapsed_secs,
                input,
                state,
                enemies,
                mosquitons,
                projectiles,
                impacts,
                char_decals,
                screen_height_px,
            ),
        }
    }

    if loadout.current() != AttackId::Flamethrower {
        state.flamethrower = None;
    }

    update_weapon_presentation(
        state,
        loadout.current() == AttackId::Flamethrower,
        input.moving_forward_back,
        dt,
        elapsed_secs,
    );
    tick_one_shot_effects(&mut state.one_shots, dt, &state.config);
    input.clear_edges();
}

fn update_weapon_presentation(
    state: &mut PlayerAttackState,
    flamethrower_selected: bool,
    moving_forward_back: bool,
    dt: f32,
    elapsed_secs: f32,
) {
    if !flamethrower_selected {
        state.weapon_bob_offset = Vec2::ZERO;
        return;
    }

    let config = &state.config;
    let firing = state
        .flamethrower
        .as_ref()
        .is_some_and(|active| active.spawning);

    if config.weapon_bob_enabled && moving_forward_back && !firing {
        state.weapon_bob_offset = weapon_bob_offset(config, elapsed_secs);
    } else {
        let t = (config.weapon_bob_return_speed * dt).clamp(0.0, 1.0);
        state.weapon_bob_offset = state.weapon_bob_offset.lerp(Vec2::ZERO, t);
    }
}

fn weapon_bob_offset(config: &FlamethrowerConfig, elapsed_secs: f32) -> Vec2 {
    let phase = elapsed_secs * config.weapon_bob_speed;
    let horizontal = phase.sin();
    Vec2::new(
        horizontal * config.weapon_bob_horizontal_px,
        -horizontal.abs() * config.weapon_bob_vertical_px,
    )
}

#[allow(clippy::too_many_arguments)]
fn update_flamethrower_attack(
    camera: &Camera,
    map: &Map,
    dt: f32,
    elapsed_secs: f32,
    input: &AttackInput,
    state: &mut PlayerAttackState,
    enemies: &mut [Enemy],
    mosquitons: &mut [Mosquiton],
    projectiles: &mut Vec<Projectile>,
    impacts: &mut Vec<ProjectileImpact>,
    char_decals: &mut Vec<CharDecal>,
    screen_height_px: f32,
) {
    let config = &state.config;
    let desired_direction =
        screen_flame_direction(input.cursor_x, config.origin(), config.chain_range);
    let desired_angle = flame_direction_angle(desired_direction);
    if state.flamethrower.is_none() && input.shoot_just_pressed {
        state.flamethrower = Some(ActiveFpFlamethrower {
            spawning: true,
            ammo: config.max_ammo,
            next_spawn_at: elapsed_secs,
            next_slot: 0,
            elapsed: 0.0,
            desired_direction,
            previous_aim_angle: desired_angle,
            segments: Vec::new(),
            tick_state: HashMap::new(),
            wall_impact: None,
            last_decal_impact: None,
        });
    }

    let Some(active) = &mut state.flamethrower else {
        return;
    };

    if active.spawning
        && (input.shoot_just_released
            || (!input.shoot_held && !input.shoot_just_pressed)
            || active.ammo <= 0.0)
    {
        active.spawning = false;
        active.wall_impact = None;
        active.last_decal_impact = None;
    }

    active.elapsed += dt;
    if active.spawning {
        active.ammo -= dt * 1000.0 * config.ammo_drain_per_ms;
    }

    let cursor_turn_velocity = if dt > f32::EPSILON {
        angle_delta(desired_angle, active.previous_aim_angle) / dt
    } else {
        0.0
    };
    let turn_velocity = if input.aim_turn_velocity.abs() > f32::EPSILON {
        input.aim_turn_velocity
    } else {
        cursor_turn_velocity
    };
    let effective_turn = turn_velocity + input.strafe_velocity * config.strafe_bend_strength;
    active.previous_aim_angle = desired_angle;
    active.desired_direction = desired_direction;

    let speed = if active.spawning {
        config.flame_speed
    } else {
        config.flame_speed * config.drain_speed_multiplier
    };

    for segment in &mut active.segments {
        segment.progress += speed * dt;
        if active.spawning {
            segment.progress = segment.progress.min(segment.target);
        }
    }
    update_flame_segment_bends(&mut active.segments, effective_turn, config, dt);

    active
        .segments
        .retain(|segment| segment.progress < config.chain_range);

    if active.spawning
        && elapsed_secs >= active.next_spawn_at
        && active.next_slot < config.max_flames
    {
        let slot = active.next_slot;
        active.segments.push(FpFlameSegment {
            progress: 0.0,
            target: config.slot_target(slot),
            bend_px: 0.0,
            slot,
        });
        active.next_slot += 1;
        active.next_spawn_at = elapsed_secs + config.spawn_interval_ms as f32 / 1000.0;
    }

    let flame_points = active_flame_points(camera, map, active, config, screen_height_px);
    active.wall_impact = flame_points.iter().find_map(|point| point.wall_impact);
    if active.spawning {
        emit_char_decals(
            char_decals,
            active.wall_impact,
            &mut active.last_decal_impact,
        );
    } else {
        active.last_decal_impact = None;
    }

    apply_flamethrower_damage(
        camera,
        map,
        config,
        active,
        &flame_points,
        enemies,
        mosquitons,
        projectiles,
        impacts,
        elapsed_secs,
    );

    if !active.spawning && active.segments.is_empty() {
        state.flamethrower = None;
    }
}

fn update_flame_segment_bends(
    segments: &mut [FpFlameSegment],
    turn_velocity: f32,
    config: &FlamethrowerConfig,
    dt: f32,
) {
    let t = (config.turn_bend_return_speed * dt).clamp(0.0, 1.0);
    for segment in segments {
        let target_bend = flame_target_bend(segment.progress, turn_velocity, config);
        segment.bend_px += (target_bend - segment.bend_px) * t;
    }
}

fn flame_target_bend(progress: f32, turn_velocity: f32, config: &FlamethrowerConfig) -> f32 {
    let progress_t = (progress / config.chain_range).clamp(0.0, 1.0);
    let strength = progress_t.powf(config.turn_bend_index_power.max(0.01));
    -turn_velocity * config.turn_bend_strength * strength
}

fn screen_flame_direction(cursor_x: f32, origin: Vec2, chain_reach: f32) -> Vec2 {
    let direction = Vec2::new(cursor_x - origin.x, chain_reach);
    if direction.length_squared() > 0.0 {
        direction.normalize()
    } else {
        Vec2::Y
    }
}

fn flame_direction_angle(direction: Vec2) -> f32 {
    direction.x.atan2(direction.y)
}

fn angle_delta(to: f32, from: f32) -> f32 {
    let mut delta = to - from;
    while delta > std::f32::consts::PI {
        delta -= std::f32::consts::TAU;
    }
    while delta < -std::f32::consts::PI {
        delta += std::f32::consts::TAU;
    }
    delta
}

fn flame_segment_offset(
    direction: Vec2,
    segment: &FpFlameSegment,
    config: &FlamethrowerConfig,
) -> Vec2 {
    let progress = (segment.progress + config.flame_start_offset_px).max(0.0);

    let chain = direction * progress;
    let bend = Vec2::new(segment.bend_px, 0.0);

    // Screen-space: vertical drop increases toward the far end (negative Y = lower on screen).
    let drop = flame_visual_drop(segment.progress, config);

    chain + bend + drop
}

fn flame_collision_offset(
    direction: Vec2,
    segment: &FpFlameSegment,
    config: &FlamethrowerConfig,
) -> Vec2 {
    let progress = (segment.progress + config.flame_start_offset_px).max(0.0);
    let chain = direction * progress;
    let bend = Vec2::new(segment.bend_px, 0.0);
    chain + bend
}

fn camera_basis(camera: &Camera) -> (Vec2, Vec2) {
    let forward = camera.direction();
    let right = Vec2::new(forward.y, -forward.x);
    (forward, right)
}

fn flame_world_scale_denominator_px(config: &FlamethrowerConfig) -> f32 {
    assert!(
        config.chain_range > 0.0,
        "flamethrower chain range must be positive"
    );
    config.chain_range
}

fn local_flame_offset(screen_offset: Vec2, config: &FlamethrowerConfig) -> Vec2 {
    screen_offset * (FLAME_RANGE_UNITS / flame_world_scale_denominator_px(config))
}

fn screen_flame_offset_from_local(local: Vec2, config: &FlamethrowerConfig) -> Vec2 {
    local * (flame_world_scale_denominator_px(config) / FLAME_RANGE_UNITS)
}

fn flame_visual_drop(progress: f32, config: &FlamethrowerConfig) -> Vec2 {
    let t = (progress / config.chain_range).clamp(0.0, 1.0);
    Vec2::new(
        0.0,
        -config.flame_far_vertical_drop_px * t.powf(config.flame_vertical_curve_power),
    )
}

fn visual_progress_for_collision_offset(
    collision_offset: Vec2,
    config: &FlamethrowerConfig,
) -> f32 {
    (collision_offset.length() - config.flame_start_offset_px).clamp(0.0, config.chain_range)
}

fn camera_local_point(camera: &Camera, world: Vec2) -> Vec2 {
    let (forward, right) = camera_basis(camera);
    let delta = world - camera.position;
    Vec2::new(delta.dot(right), delta.dot(forward))
}

fn camera_world_point(camera: &Camera, local: Vec2) -> Vec2 {
    let (forward, right) = camera_basis(camera);
    camera.position + right * local.x + forward * local.y
}

fn weapon_center_camera(screen_height: f32, config: &FlamethrowerConfig) -> Vec2 {
    let center = flamethrower_weapon_center(screen_height, config, Vec2::ZERO);
    Vec2::new(center.x, screen_height - center.y)
}

fn flame_nozzle_camera_position(screen_height: f32, config: &FlamethrowerConfig) -> Vec2 {
    let (nx, ny) = config.flame_nozzle_offset();
    weapon_center_camera(screen_height, config) + Vec2::new(nx, -ny)
}

fn active_flame_base_offset(
    direction: Vec2,
    screen_height: f32,
    config: &FlamethrowerConfig,
) -> Vec2 {
    flame_nozzle_camera_position(screen_height, config)
        - config.origin()
        - direction * config.flame_start_offset_px
}

fn active_flame_points(
    camera: &Camera,
    map: &Map,
    active: &ActiveFpFlamethrower,
    config: &FlamethrowerConfig,
    screen_height_px: f32,
) -> Vec<FlamePoint> {
    let visual_base_offset =
        active_flame_base_offset(active.desired_direction, screen_height_px, config);
    let mut raw_points = active
        .segments
        .iter()
        .map(|segment| {
            let collision_offset = Vec2::new(visual_base_offset.x, 0.0)
                + flame_collision_offset(active.desired_direction, segment, config);
            let visual_offset = visual_base_offset
                + flame_segment_offset(active.desired_direction, segment, config);
            FlamePoint {
                local: local_flame_offset(collision_offset, config),
                screen_offset: visual_offset,
                visual_base_y: visual_base_offset.y,
                progress: segment.progress,
                slot: segment.slot,
                wall_impact: None,
            }
        })
        .collect::<Vec<_>>();
    raw_points.sort_by(|a, b| {
        a.local
            .y
            .partial_cmp(&b.local.y)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut clamped_points = Vec::with_capacity(raw_points.len());
    for mut point in raw_points {
        let distance = point.local.length();
        if distance <= 0.01 {
            clamped_points.push(point);
            continue;
        }

        let local_dir = point.local / distance;
        let world_dir = camera_world_direction(camera, local_dir);
        let hit = cast_ray(map, camera.position, world_dir);
        if let Some(surface_id) = hit.surface_id
            && hit.distance < distance
        {
            point.local = local_dir * hit.distance;
            let collision_screen_offset = screen_flame_offset_from_local(point.local, config);
            let clamped_progress =
                visual_progress_for_collision_offset(collision_screen_offset, config);
            point.screen_offset = collision_screen_offset
                + Vec2::new(0.0, point.visual_base_y)
                + flame_visual_drop(clamped_progress, config);
            point.wall_impact = Some(FlameWallImpact {
                surface_id,
                u: hit.wall_x,
                v: impact_wall_v(
                    screen_height_px,
                    hit.distance,
                    config.origin() + point.screen_offset,
                ),
                seed: wall_impact_seed(surface_id, hit.wall_x, point.screen_offset.y),
            });
            clamped_points.push(point);
            break;
        }
        clamped_points.push(point);
    }
    clamped_points
}

fn flame_wall_bridge(
    impact: &FlamePoint,
    points: &[FlamePoint],
    config: &FlamethrowerConfig,
) -> Option<FlameWallBridge> {
    impact.wall_impact?;
    let impact_distance = impact.local.length();
    let nearest_visible_distance = points
        .iter()
        .filter(|point| point.wall_impact.is_none())
        .map(|point| point.local.length())
        .filter(|distance| *distance < impact_distance)
        .fold(0.0_f32, f32::max);
    if impact_distance - nearest_visible_distance < FLAME_WALL_BRIDGE_MIN_GAP_UNITS {
        return None;
    }

    let local_dir = impact.local.normalize_or_zero();
    if local_dir.length_squared() <= f32::EPSILON {
        return None;
    }

    let backoff = FLAME_WALL_BRIDGE_BACKOFF_UNITS.min(impact_distance * 0.35);
    let bridge_local = local_dir * (impact_distance - backoff).max(0.0);
    let collision_screen_offset = screen_flame_offset_from_local(bridge_local, config);
    let progress = visual_progress_for_collision_offset(collision_screen_offset, config);
    let screen_offset = collision_screen_offset
        + Vec2::new(0.0, impact.visual_base_y)
        + flame_visual_drop(progress, config);

    Some(FlameWallBridge {
        screen_offset,
        progress,
    })
}

fn flame_collision_points(points: &[FlamePoint]) -> Vec<FlameCollisionPoint> {
    points
        .iter()
        .map(|point| FlameCollisionPoint { local: point.local })
        .collect()
}

fn camera_world_direction(camera: &Camera, local_dir: Vec2) -> Vec2 {
    let (forward, right) = camera_basis(camera);
    (right * local_dir.x + forward * local_dir.y).normalize_or_zero()
}

fn impact_wall_v(screen_height_px: f32, hit_distance: f32, camera_pos: Vec2) -> f32 {
    let line_height = screen_height_px / hit_distance.max(0.001);
    let draw_start = screen_height_px * 0.5 - line_height * 0.5;
    let screen_y = screen_height_px - camera_pos.y;
    ((screen_y - draw_start) / line_height).clamp(0.0, 1.0)
}

fn wall_impact_seed(surface_id: WallSurfaceId, u: f32, v_seed: f32) -> u32 {
    let mut seed = 0x811c_9dc5_u32;
    seed ^= surface_id.cell_x as u32;
    seed = seed.wrapping_mul(0x0100_0193);
    seed ^= surface_id.cell_y as u32;
    seed = seed.wrapping_mul(0x0100_0193);
    seed ^= (u.clamp(0.0, 1.0) * 4096.0).round() as u32;
    seed = seed.wrapping_mul(0x0100_0193);
    seed ^= (v_seed * 4096.0).round() as u32;
    seed = seed.wrapping_mul(0x0100_0193);
    seed ^= surface_id.normal_sign as u32;
    seed
}

fn emit_char_decals(
    decals: &mut Vec<CharDecal>,
    impact: Option<FlameWallImpact>,
    last_impact: &mut Option<FlameWallImpact>,
) {
    let Some(impact) = impact else {
        *last_impact = None;
        return;
    };

    let start = last_impact
        .filter(|previous| previous.surface_id == impact.surface_id)
        .map_or(impact.u, |previous| previous.u);
    let delta = impact.u - start;
    let steps = ((delta.abs() / (FLAME_CHAR_DECAL_WIDTH * 0.35)).ceil() as usize).max(1);
    for step in 0..steps {
        let t = (step + 1) as f32 / steps as f32;
        let u = (start + delta * t).clamp(0.0, 1.0);
        let seed = wall_impact_seed(impact.surface_id, u, impact.v);
        push_char_decal(decals, impact.surface_id, u, impact.v, seed);
        if u < FLAME_CHAR_DECAL_WIDTH * 0.5 {
            push_char_decal(
                decals,
                adjacent_wall_surface(impact.surface_id, -1),
                u + 1.0,
                impact.v,
                seed ^ 0x9e37_79b9,
            );
        }
        if u > 1.0 - FLAME_CHAR_DECAL_WIDTH * 0.5 {
            push_char_decal(
                decals,
                adjacent_wall_surface(impact.surface_id, 1),
                u - 1.0,
                impact.v,
                seed ^ 0x85eb_ca6b,
            );
        }
    }
    if decals.len() > MAX_FLAME_CHAR_DECALS {
        let overflow = decals.len() - MAX_FLAME_CHAR_DECALS;
        decals.drain(0..overflow);
    }
    *last_impact = Some(impact);
}

fn push_char_decal(
    decals: &mut Vec<CharDecal>,
    surface_id: WallSurfaceId,
    u: f32,
    v: f32,
    seed: u32,
) {
    if decals
        .iter()
        .rev()
        .take(12)
        .any(|decal| decal.surface_id == surface_id && (decal.u - u).abs() < 0.025)
    {
        return;
    }
    decals.push(CharDecal {
        surface_id,
        u,
        v,
        width: FLAME_CHAR_DECAL_WIDTH,
        height: FLAME_CHAR_DECAL_HEIGHT,
        intensity: if seed & 1 == 0 { 0.88 } else { 0.58 },
        flip_x: seed & 0b10 != 0,
        flip_y: seed & 0b100 != 0,
        seed,
    });
}

fn adjacent_wall_surface(surface_id: WallSurfaceId, tangent_step: i32) -> WallSurfaceId {
    match surface_id.side {
        crate::raycast::HitSide::Vertical => WallSurfaceId {
            cell_y: surface_id.cell_y + tangent_step,
            ..surface_id
        },
        crate::raycast::HitSide::Horizontal => WallSurfaceId {
            cell_x: surface_id.cell_x + tangent_step,
            ..surface_id
        },
    }
}

pub fn destroy_projectiles_touching_active_flamethrower(
    camera: &Camera,
    map: &Map,
    state: &PlayerAttackState,
    projectiles: &mut Vec<Projectile>,
    impacts: &mut Vec<ProjectileImpact>,
) {
    let Some(active) = &state.flamethrower else {
        return;
    };
    let flame_points = flame_collision_points(&active_flame_points(
        camera,
        map,
        active,
        &state.config,
        144.0,
    ));
    destroy_projectiles_touching_flame(
        camera,
        map,
        &flame_points,
        state.config.hit_radius_units(),
        projectiles,
        impacts,
    );
}

#[allow(clippy::too_many_arguments)]
fn apply_flamethrower_damage(
    camera: &Camera,
    map: &Map,
    config: &FlamethrowerConfig,
    active: &mut ActiveFpFlamethrower,
    flame_points: &[FlamePoint],
    enemies: &mut [Enemy],
    mosquitons: &mut [Mosquiton],
    projectiles: &mut Vec<Projectile>,
    impacts: &mut Vec<ProjectileImpact>,
    elapsed_secs: f32,
) {
    let tick_secs = config.tick_ms as f32 / 1000.0;
    let flame_points = flame_collision_points(flame_points);
    let hit_radius = config.hit_radius_units();

    active.tick_state.retain(|target, _| match *target {
        FpFlameTarget::Enemy(index) => enemies.get(index).is_some_and(Enemy::is_alive),
        FpFlameTarget::Mosquiton(index) => mosquitons.get(index).is_some_and(Mosquiton::is_alive),
    });

    for (index, enemy) in enemies.iter_mut().enumerate() {
        if !enemy.is_alive() {
            continue;
        }
        let target = FpFlameTarget::Enemy(index);
        if can_flame_tick(&active.tick_state, target, elapsed_secs, tick_secs)
            && flame_hits_target(camera, map, &flame_points, enemy.position, hit_radius)
        {
            enemy.take_damage_from(
                config.damage_per_tick,
                DamageKind::Fire,
                config.burning_corpse_duration_secs,
            );
            active.tick_state.insert(target, elapsed_secs);
        }
    }

    for (index, mosquiton) in mosquitons.iter_mut().enumerate() {
        if !mosquiton.is_alive() {
            continue;
        }
        let target = FpFlameTarget::Mosquiton(index);
        if can_flame_tick(&active.tick_state, target, elapsed_secs, tick_secs)
            && flame_hits_target(camera, map, &flame_points, mosquiton.position, hit_radius)
        {
            mosquiton.take_damage_from(
                config.damage_per_tick,
                DamageKind::Fire,
                config.burning_corpse_duration_secs,
            );
            active.tick_state.insert(target, elapsed_secs);
        }
    }

    destroy_projectiles_touching_flame(
        camera,
        map,
        &flame_points,
        hit_radius,
        projectiles,
        impacts,
    );
}

fn destroy_projectiles_touching_flame(
    camera: &Camera,
    map: &Map,
    flame_points: &[FlameCollisionPoint],
    hit_radius: f32,
    projectiles: &mut Vec<Projectile>,
    impacts: &mut Vec<ProjectileImpact>,
) {
    for projectile in projectiles.iter_mut() {
        if projectile.alive
            && flame_hits_target(camera, map, flame_points, projectile.position, hit_radius)
        {
            projectile.alive = false;
            impacts.push(ProjectileImpact::destroy(projectile.position));
        }
    }
    projectiles.retain(|projectile| projectile.alive);
}

fn can_flame_tick(
    ticks: &HashMap<FpFlameTarget, f32>,
    target: FpFlameTarget,
    now: f32,
    interval: f32,
) -> bool {
    ticks
        .get(&target)
        .is_none_or(|last| now - *last >= interval)
}

fn flame_hits_target(
    camera: &Camera,
    map: &Map,
    flame_points: &[FlameCollisionPoint],
    target: Vec2,
    hit_radius: f32,
) -> bool {
    let target_local = camera_local_point(camera, target);
    if target_local.y < -hit_radius {
        return false;
    }
    let to_target = target - camera.position;
    let target_dist = to_target.length();
    if target_dist > 0.01 {
        let wall_hit = cast_ray(map, camera.position, to_target / target_dist);
        if wall_hit.wall_id > 0 && wall_hit.distance < target_dist {
            return false;
        }
    }

    let Some(hit_local) = flame_local_hit_point(flame_points, target_local, hit_radius) else {
        return false;
    };

    let hit_world = camera_world_point(camera, hit_local);
    let to_hit = hit_world - camera.position;
    let dist = to_hit.length();
    if dist <= 0.01 {
        return true;
    }
    cast_ray(map, camera.position, to_hit / dist).distance > dist - hit_radius
}

fn flame_local_hit_point(
    flame_points: &[FlameCollisionPoint],
    target_local: Vec2,
    hit_radius: f32,
) -> Option<Vec2> {
    let mut best = None;
    for point in flame_points {
        retain_closest_hit(&mut best, target_local, point.local, hit_radius);
    }
    for points in flame_points.windows(2) {
        retain_closest_hit(
            &mut best,
            target_local,
            closest_point_on_segment(target_local, points[0].local, points[1].local),
            hit_radius,
        );
    }
    best.map(|(_, point)| point)
}

fn retain_closest_hit(best: &mut Option<(f32, Vec2)>, target: Vec2, candidate: Vec2, radius: f32) {
    let distance = target.distance(candidate);
    if distance > radius || best.is_some_and(|(current, _)| current <= distance) {
        return;
    }
    *best = Some((distance, candidate));
}

fn closest_point_on_segment(point: Vec2, a: Vec2, b: Vec2) -> Vec2 {
    let ab = b - a;
    let len_sq = ab.length_squared();
    if len_sq <= f32::EPSILON {
        return a;
    }
    let t = ((point - a).dot(ab) / len_sq).clamp(0.0, 1.0);
    a + ab * t
}

fn tick_one_shot_effects(effects: &mut Vec<OneShotEffect>, dt: f32, config: &FlamethrowerConfig) {
    let max_duration = config.chain_range / config.flame_speed;
    for effect in effects.iter_mut() {
        effect.elapsed += dt;
    }
    effects.retain(|effect| match effect.kind {
        OneShotEffectKind::Bullet => effect.elapsed <= 0.4,
        OneShotEffectKind::Melee => effect.elapsed <= 0.9_f32.max(max_duration),
    });
}

#[allow(clippy::too_many_arguments)]
fn apply_hitscan_damage(
    camera: &Camera,
    map: &Map,
    enemies: &mut [Enemy],
    mosquitons: &mut [Mosquiton],
    projectiles: &mut Vec<Projectile>,
    impacts: &mut Vec<ProjectileImpact>,
    damage: u32,
    max_range: Option<f32>,
) {
    let enemy_hit = hitscan(camera, enemies, map);
    let mosquiton_hit = hitscan_mosquitons(camera, mosquitons, map);
    let projectile_hit = hitscan_projectiles(camera, projectiles, map);

    let mut hit = enemy_hit
        .enemy_idx
        .map(|enemy_idx| (FpShotHit::Enemy(enemy_idx), enemy_hit.distance));
    if let Some((mosquiton_idx, distance)) = mosquiton_hit
        && hit.is_none_or(|(_, current_distance)| distance < current_distance)
    {
        hit = Some((FpShotHit::Mosquiton(mosquiton_idx), distance));
    }
    if let Some((projectile_idx, distance)) = projectile_hit
        && hit.is_none_or(|(_, current_distance)| distance < current_distance)
    {
        hit = Some((FpShotHit::Projectile(projectile_idx), distance));
    }

    let Some((hit, distance)) = hit else {
        return;
    };
    if max_range.is_some_and(|range| distance > range) {
        return;
    }

    match hit {
        FpShotHit::Enemy(enemy_idx) => enemies[enemy_idx].take_damage(damage),
        FpShotHit::Mosquiton(mosquiton_idx) => mosquitons[mosquiton_idx].take_damage(damage),
        FpShotHit::Projectile(projectile_idx) => {
            if let Some(projectile) = projectiles.get_mut(projectile_idx) {
                projectile.alive = false;
                impacts.push(ProjectileImpact::destroy(projectile.position));
            }
            projectiles.retain(|projectile| projectile.alive);
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum FpShotHit {
    Enemy(usize),
    Mosquiton(usize),
    Projectile(usize),
}

pub fn draw_player_attack_overlays(
    image: &mut CxImage,
    camera: &Camera,
    map: &Map,
    sprites: &PlayerAttackSprites,
    loadout: &AttackLoadout,
    state: &PlayerAttackState,
    elapsed_secs: f32,
) {
    for effect in &state.one_shots {
        let animation = match effect.kind {
            OneShotEffectKind::Bullet => &sprites.bullet,
            OneShotEffectKind::Melee => &sprites.melee,
        };
        draw_image_scaled_center(
            image,
            animation.frame_clamped(effect.elapsed),
            effect.position,
            1.0,
        );
    }

    if loadout.current() == AttackId::Flamethrower {
        let config = &state.config;
        let origin = config.origin();
        let screen_height = image.height() as f32;
        let presentation_offset = state.weapon_bob_offset;
        let weapon_center = flamethrower_weapon_center(screen_height, config, presentation_offset);

        let active_flamethrower = state.flamethrower.as_ref();
        if let Some(active) = active_flamethrower {
            let mut points =
                active_flame_points(camera, map, active, config, image.height() as f32);
            points.sort_by(|a, b| {
                b.progress
                    .partial_cmp(&a.progress)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            for point in &points {
                if point.wall_impact.is_some() {
                    if let Some(bridge) = flame_wall_bridge(point, &points, config) {
                        let camera_pos = origin + bridge.screen_offset;
                        let pos =
                            camera_to_framebuffer(camera_pos, image.height()) + presentation_offset;
                        let scale = (config.segment_scale(bridge.progress)
                            * FLAME_WALL_BRIDGE_SCALE)
                            .min(FLAME_WALL_BRIDGE_MAX_SCALE);
                        draw_image_scaled_center(
                            image,
                            sprites
                                .flame
                                .frame_loop(active.elapsed + f32::from(point.slot) * 0.04),
                            pos,
                            scale,
                        );
                    }
                    continue;
                }
                let camera_pos = origin + point.screen_offset;
                let pos = camera_to_framebuffer(camera_pos, image.height()) + presentation_offset;
                let scale = config.segment_scale(point.progress);
                draw_image_scaled_center(
                    image,
                    sprites
                        .flame
                        .frame_loop(active.elapsed + f32::from(point.slot) * 0.04),
                    pos,
                    scale,
                );
            }
        }

        if active_flamethrower.is_none() {
            let idle_frame = sprites.idle_flame.frame_loop(elapsed_secs);
            let scale = config.idle_flame_scale;
            let half_h = idle_frame.height() as f32 * scale * 0.5;
            let (ox, oy) = config.idle_flame_offset();
            draw_image_scaled_center(
                image,
                idle_frame,
                weapon_center + Vec2::new(ox, oy - half_h),
                scale,
            );
        }

        draw_image_scaled_center(
            image,
            flamethrower_weapon_animation(sprites, state).frame_loop(elapsed_secs),
            weapon_center,
            1.0,
        );
    }
}

fn flamethrower_weapon_animation<'a>(
    sprites: &'a PlayerAttackSprites,
    state: &PlayerAttackState,
) -> &'a AtlasAnimation {
    if state.flamethrower.is_some() {
        &sprites.weapon_shooting
    } else {
        &sprites.weapon_idle
    }
}

fn flamethrower_weapon_center(
    screen_height: f32,
    config: &FlamethrowerConfig,
    presentation_offset: Vec2,
) -> Vec2 {
    Vec2::new(80.0, screen_height - 20.0) + config.weapon_base_offset() + presentation_offset
}

#[must_use]
pub fn wall_impact_sprite<'a>(
    state: &'a PlayerAttackState,
    sprites: &'a PlayerAttackSprites,
) -> Option<WallSurfaceSprite<'a>> {
    let active = state.flamethrower.as_ref()?;
    let impact = active.wall_impact?;
    Some(WallSurfaceSprite {
        surface_id: impact.surface_id,
        u: impact.u,
        v: impact.v,
        width: FLAME_WALL_IMPACT_WIDTH,
        height: FLAME_WALL_IMPACT_HEIGHT,
        texture: sprites.flame_wall_hit.frame_loop(active.elapsed),
        flip_x: impact.seed & 0b10 != 0,
        flip_y: impact.seed & 0b100 != 0,
    })
}

#[must_use]
pub fn flame_wall_mask(sprites: &PlayerAttackSprites) -> &CxImage {
    &sprites.flame_wall_hit.frames[0]
}

fn camera_to_framebuffer(pos: Vec2, screen_height: usize) -> Vec2 {
    Vec2::new(pos.x, screen_height as f32 - pos.y)
}

fn draw_image_scaled_center(dst: &mut CxImage, src: &CxImage, center: Vec2, scale: f32) {
    let scale = scale.max(0.01);
    let src_w = src.width() as i32;
    let src_h = src.height() as i32;
    let dst_w = dst.width() as i32;
    let dst_h = dst.height() as i32;
    let out_w = (src_w as f32 * scale).round().max(1.0) as i32;
    let out_h = (src_h as f32 * scale).round().max(1.0) as i32;
    let start_x = center.x.round() as i32 - out_w / 2;
    let start_y = center.y.round() as i32 - out_h / 2;
    let src_data = src.data();
    let dst_data = dst.data_mut();

    for y in 0..out_h {
        let dst_y = start_y + y;
        if dst_y < 0 || dst_y >= dst_h {
            continue;
        }
        let src_y = ((y as f32 / scale).floor() as i32).clamp(0, src_h - 1);
        for x in 0..out_w {
            let dst_x = start_x + x;
            if dst_x < 0 || dst_x >= dst_w {
                continue;
            }
            let src_x = ((x as f32 / scale).floor() as i32).clamp(0, src_w - 1);
            let pixel = src_data[(src_y * src_w + src_x) as usize];
            if pixel != TRANSPARENT_INDEX {
                dst_data[(dst_y * dst_w + dst_x) as usize] = pixel;
            }
        }
    }
}

#[derive(Debug, Deserialize)]
struct PxAtlasDescriptor {
    regions: Vec<PxAtlasRegion>,
    names: HashMap<String, u32>,
    animations: HashMap<String, PxAtlasAnimation>,
}

#[derive(Debug, Deserialize)]
struct PxAtlasRegion {
    frames: Vec<PxAtlasRect>,
}

#[derive(Clone, Copy, Debug, Deserialize)]
struct PxAtlasRect {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
}

#[derive(Debug, Deserialize)]
struct PxAtlasAnimation {
    duration_ms: u64,
}

fn load_atlas_animation(
    atlas_ron: &str,
    pxi_bytes: &[u8],
    region_name: &str,
) -> Result<AtlasAnimation, String> {
    let descriptor: PxAtlasDescriptor = ron::from_str(atlas_ron).map_err(|err| err.to_string())?;
    let region_index = descriptor
        .names
        .get(region_name)
        .copied()
        .ok_or_else(|| format!("atlas region {region_name:?} missing"))?
        as usize;
    let region = descriptor
        .regions
        .get(region_index)
        .ok_or_else(|| format!("atlas region index {region_index} missing"))?;
    let (atlas_width, _, atlas_pixels) = decode_pxi(pxi_bytes)?;
    let frames = region
        .frames
        .iter()
        .map(|rect| extract_atlas_rect(&atlas_pixels, atlas_width, *rect))
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| format!("atlas region {region_name:?} rect exceeds atlas"))?;
    let duration_secs = descriptor
        .animations
        .get(region_name)
        .map_or(frames.len() as f32 * 0.1, |animation| {
            animation.duration_ms as f32 / 1000.0
        })
        .max(0.001);
    Ok(AtlasAnimation {
        frames,
        duration_secs,
    })
}

fn extract_atlas_rect(atlas_pixels: &[u8], atlas_width: u32, rect: PxAtlasRect) -> Option<CxImage> {
    let mut data = vec![TRANSPARENT_INDEX; (rect.w * rect.h) as usize];
    for local_y in 0..rect.h {
        for local_x in 0..rect.w {
            let src_idx = ((rect.y + local_y) * atlas_width + rect.x + local_x) as usize;
            data[(local_y * rect.w + local_x) as usize] = *atlas_pixels.get(src_idx)?;
        }
    }
    Some(CxImage::new(data, rect.w as usize))
}

fn decode_pxi(bytes: &[u8]) -> Result<(u32, u32, Vec<u8>), String> {
    const HEADER_SIZE: usize = 10;
    if bytes.len() < HEADER_SIZE {
        return Err(format!("PXI file too short: {} bytes", bytes.len()));
    }
    if bytes[0..4] != asset_pipeline::pxi::MAGIC {
        return Err("PXI file has invalid magic".to_owned());
    }
    if bytes[4] != asset_pipeline::pxi::VERSION {
        return Err(format!("PXI version {} is unsupported", bytes[4]));
    }

    let width = u32::from(u16::from_le_bytes([bytes[6], bytes[7]]));
    let height = u32::from(u16::from_le_bytes([bytes[8], bytes[9]]));
    let pixel_count = (width * height) as usize;
    let expected_packed_len = pixel_count.div_ceil(2);
    let payload = &bytes[HEADER_SIZE..];
    let packed = match bytes[5] {
        asset_pipeline::pxi::FORMAT_RAW_4BPP => {
            if payload.len() != expected_packed_len {
                return Err(format!(
                    "PXI raw payload size {} != expected {expected_packed_len}",
                    payload.len(),
                ));
            }
            payload.to_vec()
        }
        asset_pipeline::pxi::FORMAT_DEFLATE_4BPP => {
            let mut inflated = Vec::with_capacity(expected_packed_len);
            let mut decoder = DeflateDecoder::new(Cursor::new(payload));
            decoder
                .read_to_end(&mut inflated)
                .map_err(|err| err.to_string())?;
            if inflated.len() != expected_packed_len {
                return Err(format!(
                    "PXI inflated payload size {} != expected {expected_packed_len}",
                    inflated.len(),
                ));
            }
            inflated
        }
        format => return Err(format!("PXI format {format} is unsupported")),
    };

    let mut indices = Vec::with_capacity(pixel_count);
    for byte in packed {
        indices.push(byte >> 4);
        indices.push(byte & 0x0f);
    }
    indices.truncate(pixel_count);
    Ok((width, height, indices))
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::{IVec2, UVec2};

    fn open_test_map() -> Map {
        Map {
            width: 8,
            height: 8,
            cells: vec![0; 64],
        }
    }

    #[test]
    fn attack_loadout_cycles_flamethrower_pistol() {
        let mut loadout = AttackLoadout::default();
        assert_eq!(loadout.current(), AttackId::Flamethrower);
        assert_eq!(loadout.cycle(), AttackId::Pistol);
        assert_eq!(loadout.cycle(), AttackId::Flamethrower);
    }

    #[test]
    fn player_attack_atlases_load() {
        let sprites = PlayerAttackSprites::load();
        assert_eq!(sprites.bullet.frames.len(), 4);
        assert_eq!(sprites.melee.frames.len(), 9);
        assert_eq!(sprites.flame.frames.len(), 4);
        assert_eq!(sprites.flame_wall_hit.frames.len(), 3);
        assert_eq!(sprites.weapon_idle.frames.len(), 1);
        assert_eq!(sprites.weapon_shooting.frames.len(), 2);
        assert_eq!(sprites.idle_flame.frames.len(), 3);
        assert_eq!(sprites.idle_flame.frames[0].size(), UVec2::new(6, 8));
    }

    #[test]
    fn flamethrower_weapon_animation_follows_active_chain_state() {
        let sprites = PlayerAttackSprites::load();
        let mut state = PlayerAttackState::default();

        assert_eq!(
            flamethrower_weapon_animation(&sprites, &state).frames.len(),
            1
        );

        state.flamethrower = Some(ActiveFpFlamethrower {
            spawning: false,
            ammo: 0.0,
            next_spawn_at: 0.0,
            next_slot: 0,
            elapsed: 0.0,
            desired_direction: Vec2::Y,
            previous_aim_angle: 0.0,
            segments: vec![FpFlameSegment {
                progress: 0.0,
                target: 1.0,
                bend_px: 0.0,
                slot: 0,
            }],
            tick_state: HashMap::new(),
            wall_impact: None,
            last_decal_impact: None,
        });

        assert_eq!(
            flamethrower_weapon_animation(&sprites, &state).frames.len(),
            2
        );
    }

    #[test]
    fn idle_nozzle_flame_renders_behind_weapon() {
        let config = FlamethrowerConfig::load();
        let sprites = PlayerAttackSprites::load();
        let idle_frame = sprites.idle_flame.frame_loop(0.0);
        let weapon_frame = sprites.weapon_idle.frame_loop(0.0);

        let flame_center_y = idle_frame.height() as f32 * config.idle_flame_scale * 0.5;
        let (ox, oy) = config.idle_flame_offset();
        let idle_flame_center = Vec2::new(ox, oy - flame_center_y);

        let weapon_tl_x = 80 - weapon_frame.width() as i32 / 2;
        let weapon_tl_y = 124 - weapon_frame.height() as i32 / 2;

        let flame_tl_x = (80.0 + idle_flame_center.x
            - idle_frame.width() as f32 * config.idle_flame_scale * 0.5)
            .round() as i32;
        let flame_tl_y = (124.0 + idle_flame_center.y - flame_center_y).round() as i32;

        let flame_sample = idle_frame
            .data()
            .iter()
            .position(|&px| px != TRANSPARENT_INDEX)
            .expect("idle flame must have at least one opaque pixel");
        let flame_sample_x = flame_sample % idle_frame.width();
        let flame_sample_y = flame_sample / idle_frame.width();
        let canvas_x = flame_tl_x + flame_sample_x as i32;
        let canvas_y = flame_tl_y + flame_sample_y as i32;

        let mut image = CxImage::empty(UVec2::new(160, 144));
        let camera = Camera::default();
        let map = open_test_map();
        let loadout = AttackLoadout::default();
        let state = PlayerAttackState::default();

        draw_player_attack_overlays(&mut image, &camera, &map, &sprites, &loadout, &state, 0.0);

        let expected = idle_frame.data()[flame_sample];
        assert_eq!(
            image.get_pixel(IVec2::new(canvas_x, canvas_y)),
            Some(expected)
        );

        let weapon_sample = weapon_frame
            .data()
            .iter()
            .position(|&px| px != TRANSPARENT_INDEX)
            .expect("weapon idle must have at least one opaque pixel");
        let weapon_sample_x = weapon_sample % weapon_frame.width();
        let weapon_sample_y = weapon_sample / weapon_frame.width();
        let weapon_canvas_x = weapon_tl_x + weapon_sample_x as i32;
        let weapon_canvas_y = weapon_tl_y + weapon_sample_y as i32;
        let expected_weapon = weapon_frame.data()[weapon_sample];
        assert_eq!(
            image.get_pixel(IVec2::new(weapon_canvas_x, weapon_canvas_y)),
            Some(expected_weapon)
        );
    }

    #[test]
    fn active_flame_visual_origin_uses_authored_nozzle() {
        let config = FlamethrowerConfig::load();
        let active = ActiveFpFlamethrower {
            spawning: true,
            ammo: config.max_ammo,
            next_spawn_at: 0.0,
            next_slot: 1,
            elapsed: 0.0,
            desired_direction: Vec2::Y,
            previous_aim_angle: 0.0,
            segments: vec![FpFlameSegment {
                progress: 0.0,
                target: 1.0,
                bend_px: 0.0,
                slot: 0,
            }],
            tick_state: HashMap::new(),
            wall_impact: None,
            last_decal_impact: None,
        };

        let points = active_flame_points(
            &Camera::default(),
            &open_test_map(),
            &active,
            &config,
            144.0,
        );
        let expected = flame_nozzle_camera_position(144.0, &config) - config.origin();

        assert_eq!(points.len(), 1);
        assert!((points[0].screen_offset.x - expected.x).abs() < 0.01);
        assert!((points[0].screen_offset.y - expected.y).abs() < 0.01);
    }

    #[test]
    fn flame_scaling_matches_config_curve() {
        let config = FlamethrowerConfig::load();
        assert!((config.segment_scale(0.0) - config.scale_near).abs() < f32::EPSILON);
        assert!((config.segment_scale(config.chain_range) - config.scale_far).abs() < f32::EPSILON);
    }

    #[test]
    fn weapon_bob_is_high_at_horizontal_extremes() {
        let config = FlamethrowerConfig::load();
        let center = weapon_bob_offset(&config, 0.0);
        let extreme = weapon_bob_offset(
            &config,
            std::f32::consts::FRAC_PI_2 / config.weapon_bob_speed,
        );

        assert!(center.x.abs() < 0.01);
        assert!(center.y.abs() < 0.01);
        assert!((extreme.x - config.weapon_bob_horizontal_px).abs() < 0.01);
        assert!((extreme.y + config.weapon_bob_vertical_px).abs() < 0.01);
    }

    #[test]
    fn flame_turn_bend_scales_from_straight_spawn_to_bent_far() {
        let config = FlamethrowerConfig::load();
        let mut segments = vec![
            FpFlameSegment {
                progress: 0.0,
                target: 20.0,
                bend_px: 0.0,
                slot: 0,
            },
            FpFlameSegment {
                progress: 40.0,
                target: 40.0,
                bend_px: 0.0,
                slot: 1,
            },
            FpFlameSegment {
                progress: 60.0,
                target: 60.0,
                bend_px: 0.0,
                slot: 2,
            },
        ];

        update_flame_segment_bends(&mut segments, 2.0, &config, 1.0 / 30.0);

        assert_eq!(segments[0].bend_px, 0.0);
        assert!(segments[1].bend_px < 0.0);
        assert!(segments[2].bend_px < segments[1].bend_px);

        let bent = segments[2].bend_px.abs();
        update_flame_segment_bends(&mut segments, 0.0, &config, 1.0 / 30.0);
        assert!(segments[2].bend_px.abs() < bent);
    }

    #[test]
    fn spawned_flame_segment_starts_straight() {
        let config = FlamethrowerConfig::load();
        assert_eq!(flame_target_bend(0.0, 2.0, &config), 0.0);
    }

    #[test]
    fn bend_uses_progress_distance_only() {
        let config = FlamethrowerConfig::load();
        let near = flame_target_bend(0.0, 2.0, &config).abs();
        let mid = flame_target_bend(config.chain_range * 0.5, 2.0, &config).abs();
        let far = flame_target_bend(config.chain_range, 2.0, &config).abs();
        let mut same_progress_segments = vec![
            FpFlameSegment {
                progress: 40.0,
                target: 40.0,
                bend_px: 0.0,
                slot: 1,
            },
            FpFlameSegment {
                progress: 40.0,
                target: 80.0,
                bend_px: 0.0,
                slot: 7,
            },
        ];

        update_flame_segment_bends(&mut same_progress_segments, 2.0, &config, 1.0 / 30.0);
        assert!(mid > near);
        assert!(far > mid);
        assert!(
            (same_progress_segments[0].bend_px - same_progress_segments[1].bend_px).abs()
                < f32::EPSILON
        );
    }

    #[test]
    fn strafe_bend_is_weaker_than_turn_bend() {
        let config = FlamethrowerConfig::load();
        let turn_only = flame_target_bend(config.chain_range, 1.0, &config).abs();
        let strafe_only = flame_target_bend(
            config.chain_range,
            1.0 * config.strafe_bend_strength,
            &config,
        )
        .abs();

        assert!(strafe_only < turn_only);
    }

    #[test]
    fn explicit_aim_turn_velocity_overrides_cursor_delta() {
        let input = AttackInput {
            aim_turn_velocity: 3.0,
            ..Default::default()
        };
        let cursor_velocity = -1.0;
        let turn_velocity = if input.aim_turn_velocity.abs() > f32::EPSILON {
            input.aim_turn_velocity
        } else {
            cursor_velocity
        };

        assert_eq!(turn_velocity, 3.0);
    }

    #[test]
    fn flame_segment_offset_combines_start_offset_and_screen_geometry() {
        let config = FlamethrowerConfig::load();
        let segment = FpFlameSegment {
            progress: 40.0,
            target: 40.0,
            bend_px: -10.0,
            slot: 3,
        };
        let desired = Vec2::Y;
        let offset = flame_segment_offset(desired, &segment, &config);

        // Bend is screen-space X only.
        assert_eq!(offset.x, segment.bend_px);

        // Forward component = start_offset + progress + vertical drop.
        let t = segment.progress / config.chain_range;
        let expected_drop =
            -config.flame_far_vertical_drop_px * t.powf(config.flame_vertical_curve_power);
        let expected_y = segment.progress + config.flame_start_offset_px + expected_drop;
        assert!((offset.y - expected_y).abs() < 0.01);
    }

    #[test]
    fn flame_collision_offset_ignores_visual_vertical_drop() {
        let config = FlamethrowerConfig::load();
        let camera = Camera {
            position: Vec2::ZERO,
            angle: 0.0,
            fov: 1.0,
        };
        let map = open_test_map();
        let segment = FpFlameSegment {
            progress: config.chain_range,
            target: config.chain_range,
            bend_px: 0.0,
            slot: 7,
        };
        let visual = flame_segment_offset(Vec2::Y, &segment, &config);
        let collision = flame_collision_offset(Vec2::Y, &segment, &config);
        let collision_local = local_flame_offset(collision, &config);
        let (forward, right) = camera_basis(&camera);
        let target = camera.position + forward * collision_local.y + right * collision_local.x;
        let points = vec![FlameCollisionPoint {
            local: collision_local,
        }];

        assert!(visual.y < collision.y);
        assert!(local_flame_offset(collision - visual, &config).y > 0.0);
        assert!(flame_hits_target(
            &camera,
            &map,
            &points,
            target,
            config.hit_radius_units()
        ));
    }

    #[test]
    fn flame_max_reach_includes_start_offset() {
        let config = FlamethrowerConfig::load();
        let segment = FpFlameSegment {
            progress: config.chain_range,
            target: config.chain_range,
            bend_px: 0.0,
            slot: 7,
        };
        let collision = flame_collision_offset(Vec2::Y, &segment, &config);
        let local = local_flame_offset(collision, &config);
        let expected_reach = FLAME_RANGE_UNITS
            * (config.chain_range + config.flame_start_offset_px)
            / config.chain_range;

        assert!((collision.y - (config.chain_range + config.flame_start_offset_px)).abs() < 0.01);
        assert!((local.y - expected_reach).abs() < 0.01);
    }

    #[test]
    fn far_wall_hit_uses_full_visual_drop() {
        let config = FlamethrowerConfig::load();
        let max_reach = FLAME_RANGE_UNITS * (config.chain_range + config.flame_start_offset_px)
            / config.chain_range;
        let collision_screen_offset = screen_flame_offset_from_local(Vec2::Y * max_reach, &config);
        let progress = visual_progress_for_collision_offset(collision_screen_offset, &config);
        let visual_base_y = active_flame_base_offset(Vec2::Y, 144.0, &config).y;
        let wall_screen_offset = collision_screen_offset
            + Vec2::new(0.0, visual_base_y)
            + flame_visual_drop(progress, &config);

        assert!(
            (collision_screen_offset.y - (config.chain_range + config.flame_start_offset_px)).abs()
                < 0.01
        );
        assert!((progress - config.chain_range).abs() < 0.01);
        assert!(
            (flame_visual_drop(progress, &config).y + config.flame_far_vertical_drop_px).abs()
                < 0.01
        );
        let expected_wall_y =
            collision_screen_offset.y + visual_base_y - config.flame_far_vertical_drop_px;
        assert!((wall_screen_offset.y - expected_wall_y).abs() < 0.01);
    }

    #[test]
    fn flame_collision_capsules_follow_lateral_curve() {
        let points = vec![
            FlameCollisionPoint {
                local: Vec2::new(0.0, 1.0),
            },
            FlameCollisionPoint {
                local: Vec2::new(0.8, 2.0),
            },
        ];

        assert!(flame_local_hit_point(&points, Vec2::new(0.8, 2.0), 0.2).is_some());
        assert!(flame_local_hit_point(&points, Vec2::new(0.0, 2.0), 0.2).is_none());
    }

    #[test]
    fn flame_points_clamp_to_first_wall() {
        let config = FlamethrowerConfig::load();
        let camera = Camera {
            position: Vec2::new(1.5, 1.5),
            angle: 0.0,
            fov: 1.0,
        };
        let mut map = Map {
            width: 4,
            height: 3,
            cells: vec![0; 12],
        };
        map.cells[map.width + 2] = 1;
        let active = ActiveFpFlamethrower {
            spawning: true,
            ammo: config.max_ammo,
            next_spawn_at: 0.0,
            next_slot: 2,
            elapsed: 0.0,
            desired_direction: Vec2::Y,
            previous_aim_angle: 0.0,
            segments: vec![
                FpFlameSegment {
                    progress: 20.0,
                    target: 20.0,
                    bend_px: 0.0,
                    slot: 0,
                },
                FpFlameSegment {
                    progress: config.chain_range,
                    target: config.chain_range,
                    bend_px: 0.0,
                    slot: 1,
                },
            ],
            tick_state: HashMap::new(),
            wall_impact: None,
            last_decal_impact: None,
        };

        let points = active_flame_points(&camera, &map, &active, &config, 144.0);

        assert_eq!(points.len(), 1);
        assert!(points[0].wall_impact.is_some());
        assert!((points[0].local.y - 0.5).abs() < 0.01);
        let collision_screen_offset = screen_flame_offset_from_local(points[0].local, &config);
        let clamped_progress =
            visual_progress_for_collision_offset(collision_screen_offset, &config);
        let expected_screen_offset = collision_screen_offset
            + Vec2::new(0.0, points[0].visual_base_y)
            + flame_visual_drop(clamped_progress, &config);
        assert!((points[0].screen_offset.y - expected_screen_offset.y).abs() < 0.01);
        let impact = points[0].wall_impact.unwrap();
        assert!(
            impact.v > 0.0 && impact.v < 1.0,
            "wall impact v should be wall-local, got {}",
            impact.v
        );
    }

    #[test]
    fn flame_wall_bridge_uses_dropped_visual_position() {
        let config = FlamethrowerConfig::load();
        let surface_id = WallSurfaceId {
            cell_x: 2,
            cell_y: 2,
            side: crate::raycast::HitSide::Vertical,
            normal_sign: -1,
        };
        let visible = FlamePoint {
            local: Vec2::new(0.0, 1.0),
            screen_offset: Vec2::ZERO,
            visual_base_y: 4.0,
            progress: 20.0,
            slot: 0,
            wall_impact: None,
        };
        let impact = FlamePoint {
            local: Vec2::new(0.0, 2.0),
            screen_offset: Vec2::ZERO,
            visual_base_y: 4.0,
            progress: config.chain_range,
            slot: 7,
            wall_impact: Some(FlameWallImpact {
                surface_id,
                u: 0.5,
                v: 0.5,
                seed: 0,
            }),
        };
        let points = [visible, impact];
        let bridge = flame_wall_bridge(&impact, &points, &config).unwrap();
        let collision_screen_offset = screen_flame_offset_from_local(
            Vec2::new(0.0, 2.0 - FLAME_WALL_BRIDGE_BACKOFF_UNITS),
            &config,
        );
        let progress = visual_progress_for_collision_offset(collision_screen_offset, &config);
        let expected_y = collision_screen_offset.y
            + impact.visual_base_y
            + flame_visual_drop(progress, &config).y;

        assert!((bridge.screen_offset.y - expected_y).abs() < 0.01);
        assert!(bridge.progress > 0.0);
    }

    #[test]
    fn char_decal_emission_spills_across_adjacent_wall_faces() {
        let surface_id = WallSurfaceId {
            cell_x: 2,
            cell_y: 2,
            side: crate::raycast::HitSide::Vertical,
            normal_sign: -1,
        };
        let mut decals = Vec::new();
        let mut last = None;

        emit_char_decals(
            &mut decals,
            Some(FlameWallImpact {
                surface_id,
                u: 0.03,
                v: 0.45,
                seed: 1,
            }),
            &mut last,
        );

        assert!(decals.iter().any(|decal| decal.surface_id == surface_id));
        assert!(decals.iter().any(|decal| {
            decal.surface_id
                == WallSurfaceId {
                    cell_y: surface_id.cell_y - 1,
                    ..surface_id
                }
                && decal.u > 1.0
        }));
    }

    #[test]
    fn flame_wall_blocks_damage_behind_it() {
        let config = FlamethrowerConfig::load();
        let camera = Camera {
            position: Vec2::new(1.5, 1.5),
            angle: 0.0,
            fov: 1.0,
        };
        let mut map = Map {
            width: 4,
            height: 3,
            cells: vec![0; 12],
        };
        map.cells[map.width + 2] = 1;
        let points = vec![FlameCollisionPoint {
            local: Vec2::new(0.0, 0.5),
        }];
        let target = Vec2::new(2.05, 1.5);

        assert!(!flame_hits_target(
            &camera,
            &map,
            &points,
            target,
            config.hit_radius_units()
        ));
    }

    #[test]
    fn flamethrower_destroys_projectiles_on_collision_chain() {
        let config = FlamethrowerConfig::load();
        let camera = Camera {
            position: Vec2::ZERO,
            angle: 0.0,
            fov: 1.0,
        };
        let map = open_test_map();
        let mut active = ActiveFpFlamethrower {
            spawning: true,
            ammo: config.max_ammo,
            next_spawn_at: 0.0,
            next_slot: 1,
            elapsed: 0.0,
            desired_direction: Vec2::Y,
            previous_aim_angle: 0.0,
            segments: vec![FpFlameSegment {
                progress: 40.0,
                target: 40.0,
                bend_px: 0.0,
                slot: 0,
            }],
            tick_state: HashMap::new(),
            wall_impact: None,
            last_decal_impact: None,
        };
        let collision_local = local_flame_offset(
            flame_collision_offset(Vec2::Y, &active.segments[0], &config),
            &config,
        );
        let mut projectiles = vec![Projectile {
            position: Vec2::new(collision_local.y, 0.0),
            source_position: Vec2::new(3.0, 0.0),
            direction: -Vec2::X,
            speed: 1.0,
            radius: 0.3,
            damage: 10,
            lifetime: 1.0,
            alive: true,
        }];
        let mut impacts = Vec::new();
        let mut enemies = Vec::new();
        let mut mosquitons = Vec::new();

        let flame_points = active_flame_points(&camera, &map, &active, &config, 144.0);
        apply_flamethrower_damage(
            &camera,
            &map,
            &config,
            &mut active,
            &flame_points,
            &mut enemies,
            &mut mosquitons,
            &mut projectiles,
            &mut impacts,
            0.0,
        );

        assert!(projectiles.is_empty());
        assert_eq!(impacts.len(), 1);
    }

    #[test]
    fn active_flamethrower_intercepts_projectile_before_it_hits_player() {
        let config = FlamethrowerConfig::load();
        let camera = Camera {
            position: Vec2::ZERO,
            angle: 0.0,
            fov: 1.0,
        };
        let map = open_test_map();
        let segment = FpFlameSegment {
            progress: 40.0,
            target: 40.0,
            bend_px: 0.0,
            slot: 0,
        };
        let collision_local =
            local_flame_offset(flame_collision_offset(Vec2::Y, &segment, &config), &config);
        let mut state = PlayerAttackState {
            one_shots: Vec::new(),
            flamethrower: Some(ActiveFpFlamethrower {
                spawning: true,
                ammo: config.max_ammo,
                next_spawn_at: 0.0,
                next_slot: 1,
                elapsed: 0.0,
                desired_direction: Vec2::Y,
                previous_aim_angle: 0.0,
                segments: vec![segment],
                tick_state: HashMap::new(),
                wall_impact: None,
                last_decal_impact: None,
            }),
            weapon_bob_offset: Vec2::ZERO,
            config,
        };
        let mut projectiles = vec![Projectile {
            position: Vec2::new(collision_local.y, 0.0),
            source_position: Vec2::new(3.0, 0.0),
            direction: -Vec2::X,
            speed: 10.0,
            radius: 0.3,
            damage: 10,
            lifetime: 1.0,
            alive: true,
        }];
        let mut impacts = Vec::new();

        destroy_projectiles_touching_active_flamethrower(
            &camera,
            &map,
            &state,
            &mut projectiles,
            &mut impacts,
        );
        let projectile_result =
            crate::enemy::tick_projectiles(&mut projectiles, camera.position, &map, 1.0);

        assert_eq!(projectile_result.player_damage, 0);
        assert!(projectiles.is_empty());
        assert_eq!(impacts.len(), 1);

        state.flamethrower = None;
        destroy_projectiles_touching_active_flamethrower(
            &camera,
            &map,
            &state,
            &mut projectiles,
            &mut impacts,
        );
    }

    #[test]
    fn flame_start_offset_places_near_slot_twenty_px_from_origin() {
        let config = FlamethrowerConfig::load();
        let near = FpFlameSegment {
            progress: 0.0,
            target: 10.0,
            bend_px: 0.0,
            slot: 0,
        };
        let far = FpFlameSegment {
            progress: config.chain_range,
            target: config.chain_range,
            bend_px: 0.0,
            slot: 7,
        };
        let dir = Vec2::Y;
        let near_offset = flame_segment_offset(dir, &near, &config);
        let far_offset = flame_segment_offset(dir, &far, &config);

        let expected_near_y = config.flame_start_offset_px;
        assert!((near_offset.y - expected_near_y).abs() < 0.01);

        let expected_far_y =
            config.chain_range + config.flame_start_offset_px - config.flame_far_vertical_drop_px;
        assert!((far_offset.y - expected_far_y).abs() < 0.01);
    }

    #[test]
    fn flame_direction_uses_cursor_x_and_fixed_reach() {
        let origin = Vec2::new(80.0, 14.0);
        let left = screen_flame_direction(40.0, origin, 80.0);
        let right = screen_flame_direction(120.0, origin, 80.0);
        assert!(left.x < 0.0);
        assert!(right.x > 0.0);
        assert!((left.y - right.y).abs() < 0.01);
    }
}
