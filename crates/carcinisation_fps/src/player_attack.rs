//! First-person player attacks and weapon overlays.

use bevy::prelude::{Reflect, ReflectResource, Resource, Vec2};
use carapace::{image::CxImage, palette::TRANSPARENT_INDEX};
use flate2::read::DeflateDecoder;
use serde::Deserialize;
use std::{
    collections::HashMap,
    io::{Cursor, Read},
    sync::Arc,
};

use crate::{
    billboard::Billboard,
    camera::Camera,
    enemy::{Enemy, Projectile, ProjectileImpact, hitscan, hitscan_projectiles},
    map::Map,
    mosquiton::{Mosquiton, hitscan_mosquitons},
    raycast::{WallSurfaceId, cast_ray},
    render::{CharDecal, WallSurfaceSprite},
    spidey::{Spidey, hitscan_spideys},
};

/// Load an atlas animation from workspace-relative RON + PXI paths.
///
/// In production, uses compile-time embedded data. In dev mode (`hot_reload`),
/// reads from filesystem first with embedded fallback — enabling live sprite
/// hot reload.
macro_rules! load_sprite_atlas {
    ($ron_path:literal, $pxi_path:literal, $region:expr) => {{
        const EMBEDDED_RON: &str =
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../", $ron_path));
        const EMBEDDED_PXI: &[u8] =
            include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../", $pxi_path));

        #[cfg(feature = "hot_reload")]
        {
            // Try filesystem data first. If the file is missing OR the data is
            // invalid (wrong dimensions, corrupt PXI), fall back to embedded.
            let fs_result: Option<Result<AtlasAnimation, String>> = (|| {
                let ron_text = std::fs::read_to_string($ron_path).ok()?;
                let pxi_data = std::fs::read($pxi_path).ok()?;
                Some(load_atlas_animation(&ron_text, &pxi_data, $region))
            })();

            match fs_result {
                Some(Ok(anim)) => Ok(anim),
                Some(Err(e)) => {
                    bevy::log::warn!(
                        "{} / {}: filesystem data invalid ({}), using embedded",
                        $ron_path,
                        $pxi_path,
                        e,
                    );
                    load_atlas_animation(EMBEDDED_RON, EMBEDDED_PXI, $region)
                }
                None => load_atlas_animation(EMBEDDED_RON, EMBEDDED_PXI, $region),
            }
        }

        #[cfg(not(feature = "hot_reload"))]
        {
            load_atlas_animation(EMBEDDED_RON, EMBEDDED_PXI, $region)
        }
    }};
}

const BULLET_REGION: &str = "bullet_particles";
const MELEE_REGION: &str = "melee_slash";
const FLAME_REGION: &str = "flame";
const FLAME_WALL_HIT_REGION: &str = "flame_wall_hit";
const FLAMETHROWER_IDLE_REGION: &str = "flamethrower_idle";
const FLAMETHROWER_SHOOTING_REGION: &str = "flamethrower_shooting";
const STAGE_IDLE_FLAME_REGION: &str = "flamethrower_flame";
const GUN_IDLE_REGION: &str = "idle";
const GUN_SHOOTING_REGION: &str = "shooting";
const GUN_MUZZLE_FLASH_REGION: &str = "shooting";
const PISTOL_EFFECT_POS: Vec2 = Vec2::new(80.0, 72.0);
const MELEE_EFFECT_POS: Vec2 = Vec2::new(80.0, 72.0);
const MELEE_RANGE_UNITS: f32 = 1.1;
const FLAME_WALL_IMPACT_WIDTH: f32 = 0.30;
const FLAME_WALL_IMPACT_HEIGHT: f32 = 0.30;
const FLAME_CHAR_DECAL_WIDTH: f32 = FLAME_WALL_IMPACT_WIDTH;
const FLAME_CHAR_DECAL_HEIGHT: f32 = FLAME_WALL_IMPACT_HEIGHT;
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
    pub const fn clear_edges(&mut self) {
        self.shoot_just_pressed = false;
        self.shoot_just_released = false;
        self.melee_triggered = false;
        self.cycle_requested = false;
    }
}

/// First-person flamethrower visual config.
///
/// Loaded from `assets/config/attacks/player_flamethrower_1p.ron`.
/// Controls how the local player's own flamethrower looks on screen.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename = "PlayerFlamethrower1pConfig")]
pub struct PlayerFlamethrower1pConfig {
    /// Forward offset from the camera position to the flame emission point
    /// along the facing direction (world units). Pushes the stream origin
    /// ahead of the camera so flames appear to emerge from the weapon nozzle.
    pub nozzle_forward: f32,
    /// Base screen-space offset of the weapon sprite from its default position
    /// (pixels, x right / y down).
    pub weapon_base_offset_px: (f32, f32),
    /// Vertical pixel distance the weapon drops when idle (lowered stance).
    /// Lerps toward 0 when firing (raised stance).
    pub weapon_raise_px: f32,
    /// Speed of the weapon raise/lower tween (higher = snappier).
    pub weapon_raise_speed: f32,
    /// Whether the weapon bobs while the player walks.
    pub weapon_bob_enabled: bool,
    /// Horizontal amplitude of the walk bob (pixels).
    pub weapon_bob_horizontal_px: f32,
    /// Vertical amplitude of the walk bob (pixels).
    pub weapon_bob_vertical_px: f32,
    /// Oscillation speed of the walk bob (radians per second).
    pub weapon_bob_speed: f32,
    /// Speed at which the bob returns to centre when the player stops (higher = faster).
    pub weapon_bob_return_speed: f32,
    /// Screen-space offset of the small idle nozzle flame relative to the
    /// weapon sprite centre (pixels, x right / y down). Shown when not firing.
    pub idle_flame_offset: (f32, f32),
    /// Scale multiplier for the idle nozzle flame sprite.
    pub idle_flame_scale: f32,
    /// World-space billboard height for the nearest flame sample (at the nozzle).
    /// Interpolates toward `billboard_scale_far` as samples age toward max range.
    pub billboard_scale_near: f32,
    /// World-space billboard height for the farthest flame sample (at max range).
    pub billboard_scale_far: f32,
    /// Multiplier for the transient nozzle/head billboard that visually bridges
    /// the viewmodel nozzle to the persistent world-space stream.
    pub nozzle_head_scale: f32,
    /// Lateral offset from the camera centre to the nozzle (world units).
    /// Negative = left of centre. Aligns the stream origin with the on-screen
    /// weapon nozzle position.
    pub nozzle_lateral: f32,
    /// Vertical billboard offset at the nozzle (world units, negative = below
    /// eye level). Fades linearly to zero at max range so distant flames
    /// converge toward the aim point.
    pub nozzle_height: f32,
}

/// Third-person (remote player) flame visual config.
///
/// Loaded from `assets/config/attacks/player_flamethrower_3p.ron`.
/// Controls how another player's flamethrower looks from the observer's perspective.
#[derive(Clone, Debug, Deserialize, Resource)]
#[serde(rename = "PlayerFlamethrower3pConfig")]
pub struct PlayerFlamethrower3pConfig {
    /// Forward offset from the remote player's origin to the flame emission
    /// point along their facing direction (world units). Positions the stream
    /// start in front of the player sprite so it appears to come from the weapon.
    pub nozzle_forward: f32,
    /// World-space billboard height for the nearest flame sample (at the nozzle).
    /// Interpolates toward `flame_scale_far` as samples age toward max range.
    pub flame_scale_near: f32,
    /// World-space billboard height for the farthest flame sample (at max range).
    pub flame_scale_far: f32,
    /// Vertical billboard offset applied to every flame sample (world units).
    /// Negative values lower the stream below eye level. Unlike the 1P
    /// `nozzle_height`, this does not fade with distance.
    pub nozzle_height: f32,
    /// Lateral offset from the player's centre to the nozzle (world units).
    /// Positive = right of the facing direction. Shifts the emission point
    /// sideways so the stream originates from the weapon hand.
    pub nozzle_lateral: f32,
    /// Per-sample lateral jitter amplitude (world units), scaled by age.
    /// Adds subtle randomness so the stream doesn't look perfectly straight.
    pub jitter_amp: f32,
    /// Multiplier applied to sample age for sprite animation phase offset.
    /// Higher values make consecutive samples animate more out of phase.
    pub phase_step: f32,
    /// Distance to pull the wall-impact billboard back from the wall surface
    /// (world units). Prevents z-fighting with the wall.
    pub wall_offset: f32,
    /// World-space billboard height of the wall-impact splash effect.
    pub impact_scale: f32,
}

impl Default for PlayerFlamethrower3pConfig {
    fn default() -> Self {
        Self {
            nozzle_forward: 0.15,
            flame_scale_near: 0.22,
            flame_scale_far: 0.22,
            nozzle_height: -0.22,
            nozzle_lateral: -0.06,
            jitter_amp: 0.015,
            phase_step: 0.15,
            wall_offset: 0.08,
            impact_scale: 0.5,
        }
    }
}

/// Visual tuning for ground fire flame billboards.
/// Loaded from `assets/config/attacks/ground_fire.ron`.
#[derive(Clone, Debug, Deserialize, Resource)]
#[serde(rename = "GroundFireVisualConfig")]
pub struct GroundFireVisualConfig {
    /// Number of flame sprites per ground fire.
    pub flame_count: usize,
    /// Spread radius for flame placement (world units).
    pub visual_radius: f32,
    /// Base world-space height of each flame billboard.
    pub flame_world_height: f32,
    /// Bottom pixels to crop from the flame sprite (hides base).
    pub crop_bottom_px: usize,
}

impl Default for GroundFireVisualConfig {
    fn default() -> Self {
        Self {
            flame_count: 6,
            visual_radius: 0.35,
            flame_world_height: 0.39,
            crop_bottom_px: 4,
        }
    }
}

impl GroundFireVisualConfig {
    #[must_use]
    pub fn load() -> Self {
        carcinisation_core::ron_config!("assets/config/attacks/ground_fire.ron")
    }
}

impl PlayerFlamethrower1pConfig {
    #[must_use]
    pub fn load() -> Self {
        carcinisation_core::ron_config!("assets/config/attacks/player_flamethrower_1p.ron")
    }

    #[must_use]
    pub const fn weapon_base_offset(&self) -> Vec2 {
        Vec2::new(self.weapon_base_offset_px.0, self.weapon_base_offset_px.1)
    }

    #[must_use]
    pub const fn idle_flame_offset(&self) -> (f32, f32) {
        self.idle_flame_offset
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename = "GunConfig")]
pub struct GunConfig {
    pub weapon_base_offset_px: (f32, f32),
    pub weapon_raise_px: f32,
    pub weapon_raise_speed: f32,
    pub weapon_bob_enabled: bool,
    pub weapon_bob_horizontal_px: f32,
    pub weapon_bob_vertical_px: f32,
    pub weapon_bob_speed: f32,
    pub weapon_bob_return_speed: f32,
    pub muzzle_flash_offset: (f32, f32),
    pub muzzle_flash_scale: f32,
}

impl GunConfig {
    #[must_use]
    pub fn load() -> Self {
        carcinisation_core::ron_config!("assets/config/attacks/player_gun_fps.ron")
    }

    #[must_use]
    pub const fn weapon_base_offset(&self) -> Vec2 {
        Vec2::new(self.weapon_base_offset_px.0, self.weapon_base_offset_px.1)
    }

    #[must_use]
    pub const fn muzzle_flash_offset(&self) -> Vec2 {
        Vec2::new(self.muzzle_flash_offset.0, self.muzzle_flash_offset.1)
    }
}

#[derive(Clone, Debug)]
struct AtlasAnimation {
    frames: Vec<Arc<CxImage>>,
    duration_secs: f32,
}

impl AtlasAnimation {
    fn frame_loop(&self, elapsed_secs: f32) -> &Arc<CxImage> {
        let len = self.frames.len();
        if len == 1 || self.duration_secs <= f32::EPSILON {
            return &self.frames[0];
        }
        let t = (elapsed_secs / self.duration_secs).fract();
        let index = ((t * len as f32) as usize).min(len - 1);
        &self.frames[index]
    }

    fn frame_clamped(&self, elapsed_secs: f32) -> &Arc<CxImage> {
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
    gun_idle: AtlasAnimation,
    gun_shooting: AtlasAnimation,
    gun_muzzle_flash: AtlasAnimation,
}

impl PlayerAttackSprites {
    #[must_use]
    pub fn load() -> Self {
        Self {
            bullet: load_sprite_atlas!(
                "assets/sprites/attacks/player_bullet/atlas.px_atlas.ron",
                "assets/sprites/attacks/player_bullet/atlas.pxi",
                BULLET_REGION
            )
            .expect("player bullet atlas must load"),
            melee: load_sprite_atlas!(
                "assets/sprites/attacks/player_melee/atlas.px_atlas.ron",
                "assets/sprites/attacks/player_melee/atlas.pxi",
                MELEE_REGION
            )
            .expect("player melee atlas must load"),
            flame: load_sprite_atlas!(
                "assets/sprites/attacks/player_flame/atlas.px_atlas.ron",
                "assets/sprites/attacks/player_flame/atlas.pxi",
                FLAME_REGION
            )
            .expect("player flame atlas must load"),
            flame_wall_hit: load_sprite_atlas!(
                "assets/sprites/attacks/player_flame_wall/atlas.px_atlas.ron",
                "assets/sprites/attacks/player_flame_wall/atlas.pxi",
                FLAME_WALL_HIT_REGION
            )
            .expect("player flame wall hit atlas must load"),
            weapon_idle: load_sprite_atlas!(
                "assets/sprites/ui/stage_flamethrower_weapon/atlas.px_atlas.ron",
                "assets/sprites/ui/stage_flamethrower_weapon/atlas.pxi",
                FLAMETHROWER_IDLE_REGION
            )
            .expect("stage flamethrower idle weapon atlas must load"),
            weapon_shooting: load_sprite_atlas!(
                "assets/sprites/ui/stage_flamethrower_weapon_shooting/atlas.px_atlas.ron",
                "assets/sprites/ui/stage_flamethrower_weapon_shooting/atlas.pxi",
                FLAMETHROWER_SHOOTING_REGION
            )
            .expect("stage flamethrower shooting weapon atlas must load"),
            idle_flame: load_sprite_atlas!(
                "assets/sprites/ui/stage_flamethrower_flame/atlas.px_atlas.ron",
                "assets/sprites/ui/stage_flamethrower_flame/atlas.pxi",
                STAGE_IDLE_FLAME_REGION
            )
            .expect("stage flamethrower idle flame atlas must load"),
            gun_idle: load_sprite_atlas!(
                "assets/sprites/ui/stage_gun_weapon/atlas.px_atlas.ron",
                "assets/sprites/ui/stage_gun_weapon/atlas.pxi",
                GUN_IDLE_REGION
            )
            .expect("stage gun idle atlas must load"),
            gun_shooting: load_sprite_atlas!(
                "assets/sprites/ui/stage_gun_weapon_shooting/atlas.px_atlas.ron",
                "assets/sprites/ui/stage_gun_weapon_shooting/atlas.pxi",
                GUN_SHOOTING_REGION
            )
            .expect("stage gun shooting atlas must load"),
            gun_muzzle_flash: load_sprite_atlas!(
                "assets/sprites/ui/stage_gun_muzzle_flash/atlas.px_atlas.ron",
                "assets/sprites/ui/stage_gun_muzzle_flash/atlas.pxi",
                GUN_MUZZLE_FLASH_REGION
            )
            .expect("stage gun muzzle flash atlas must load"),
        }
    }

    #[must_use]
    pub fn flame_frame_loop(&self, elapsed_secs: f32) -> &Arc<CxImage> {
        self.flame.frame_loop(elapsed_secs)
    }

    #[must_use]
    pub fn flame_wall_hit_frame_loop(&self, elapsed_secs: f32) -> &Arc<CxImage> {
        self.flame_wall_hit.frame_loop(elapsed_secs)
    }
}

#[derive(Resource, Debug)]
pub struct PlayerAttackState {
    one_shots: Vec<OneShotEffect>,
    flamethrower: Option<ActiveFpFlamethrower>,
    /// Muzzle flash elapsed timer. `Some` while the flash animation is playing.
    gun_muzzle_flash_elapsed: Option<f32>,
    weapon_bob_offset: Vec2,
    /// Vertical camera bob offset in pixels, driven by walk animation.
    pub view_bob: f32,
    /// Current vertical offset for the idle-lowered / shooting-raised tween.
    /// Starts at the active weapon's `weapon_raise_px` (lowered) and lerps to 0 when shooting.
    weapon_raise_offset: f32,
    config: PlayerFlamethrower1pConfig,
    /// Cached copy of the shared flamethrower config. Kept in sync with
    /// `Res<PlayerFlamethrowerConfig>` by the hot reload system in `plugin.rs`.
    shared: carcinisation_fps_core::PlayerFlamethrowerConfig,
    gun_config: GunConfig,
}

impl PlayerAttackState {
    /// Trigger the pistol muzzle flash animation (used by multiplayer client on `MuzzleFlash` event).
    pub const fn trigger_muzzle_flash(&mut self) {
        self.gun_muzzle_flash_elapsed = Some(0.0);
    }

    /// Sync the cached shared config copy after a hot reload updates the Resource.
    pub const fn update_shared(&mut self, cfg: carcinisation_fps_core::PlayerFlamethrowerConfig) {
        self.shared = cfg;
    }
}

impl Default for PlayerAttackState {
    fn default() -> Self {
        let config = PlayerFlamethrower1pConfig::load();
        let shared = carcinisation_fps_core::PlayerFlamethrowerConfig::load();
        let gun_config = GunConfig::load();
        let weapon_raise_offset = config.weapon_raise_px;
        Self {
            one_shots: Vec::new(),
            flamethrower: None,
            gun_muzzle_flash_elapsed: None,
            weapon_bob_offset: Vec2::ZERO,
            view_bob: 0.0,
            weapon_raise_offset,
            config,
            shared,
            gun_config,
        }
    }
}

impl PlayerAttackState {
    /// Build from an already-loaded shared config (avoids re-parsing the RON file).
    #[must_use]
    pub fn new(shared: carcinisation_fps_core::PlayerFlamethrowerConfig) -> Self {
        let config = PlayerFlamethrower1pConfig::load();
        let gun_config = GunConfig::load();
        let weapon_raise_offset = config.weapon_raise_px;
        Self {
            one_shots: Vec::new(),
            flamethrower: None,
            gun_muzzle_flash_elapsed: None,
            weapon_bob_offset: Vec2::ZERO,
            view_bob: 0.0,
            weapon_raise_offset,
            config,
            shared,
            gun_config,
        }
    }

    #[must_use]
    pub const fn config(&self) -> &PlayerFlamethrower1pConfig {
        &self.config
    }

    #[must_use]
    pub const fn shared(&self) -> &carcinisation_fps_core::PlayerFlamethrowerConfig {
        &self.shared
    }

    /// Whether the flamethrower has been activated but is no longer spawning
    /// new samples (ammo depleted, draining). Used to suppress `fire_held`
    /// so the server clears `flame_active` for 3P rendering.
    #[must_use]
    pub fn is_flame_draining(&self) -> bool {
        self.flamethrower.as_ref().is_some_and(|ft| !ft.spawning)
    }

    /// Produce world-space billboards from the active flame stream samples.
    #[must_use]
    pub fn flame_chain_billboards(
        &self,
        camera: &Camera,
        sprites: &PlayerAttackSprites,
    ) -> Vec<Billboard> {
        let Some(active) = &self.flamethrower else {
            return Vec::new();
        };
        let config = &self.config;
        let shared = &self.shared;
        let max_age = shared.max_stream_age();

        let mut billboards = Vec::new();
        if active.spawning {
            let dir = camera.direction();
            let nozzle_pos = flame_nozzle_position(
                camera.position,
                dir,
                config.nozzle_forward,
                config.nozzle_lateral,
            );
            billboards.push(Billboard {
                position: nozzle_pos,
                height: config.nozzle_height,
                world_height: config.billboard_scale_near * config.nozzle_head_scale,
                sprite: Arc::clone(sprites.flame_frame_loop(active.elapsed + 0.07)),
                flip_x: false,
                palette_variant: None,
            });
        }
        for sample in &active.samples {
            let pos = sample.world_position(shared.speed);
            let t = (sample.age / max_age).clamp(0.0, 1.0);
            let world_scale = config.billboard_scale_near
                + (config.billboard_scale_far - config.billboard_scale_near) * t;
            let height = config.nozzle_height * (1.0 - t);
            let phase = active.elapsed + sample.age * 0.5;

            billboards.push(Billboard {
                position: pos,
                height,
                world_height: world_scale,
                sprite: Arc::clone(sprites.flame_frame_loop(phase)),
                flip_x: false,
                palette_variant: None,
            });
        }

        billboards
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
    elapsed: f32,
    spawn_cooldown: f32,
    sample_counter: u32,
    samples: Vec<FlameStreamSample>,
    wall_impact: Option<FlameWallImpact>,
    last_decal_impact: Option<FlameWallImpact>,
}

/// A single sample in the persistent flame stream.
#[derive(Clone, Debug)]
struct FlameStreamSample {
    emit_position: Vec2,
    emit_direction: Vec2,
    age: f32,
    #[allow(dead_code)]
    seed: u32,
}

impl FlameStreamSample {
    fn world_position(&self, speed: f32) -> Vec2 {
        self.emit_position + self.emit_direction * speed * self.age
    }
}

#[derive(Clone, Copy, Debug)]
struct FlameWallImpact {
    surface_id: WallSurfaceId,
    u: f32,
    v: f32,
    seed: u32,
}

#[allow(clippy::too_many_arguments)]
pub fn process_player_attacks(
    camera: &Camera,
    map: &Map,
    sprites: &PlayerAttackSprites,
    hitscan_damage: u32,
    dt: f32,
    elapsed_secs: f32,
    input: &mut AttackInput,
    loadout: &mut AttackLoadout,
    state: &mut PlayerAttackState,
    enemies: &mut [Enemy],
    mosquitons: &mut [Mosquiton],
    spideys: &mut [Spidey],
    projectiles: &mut Vec<Projectile>,
    impacts: &mut Vec<ProjectileImpact>,
    char_decals: &mut Vec<CharDecal>,
    screen_height_px: f32,
    legacy_shoot_request: &mut bool,
    burn_config: &carcinisation_fps_core::BurnConfig,
    view_bob_amplitude: f32,
    view_bob_freq_mult: f32,
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
            spideys,
            projectiles,
            impacts,
            hitscan_damage.saturating_mul(3),
            Some(MELEE_RANGE_UNITS),
        );
    } else {
        match loadout.current() {
            AttackId::Pistol => {
                if input.shoot_just_pressed || legacy_shot {
                    state.gun_muzzle_flash_elapsed = Some(0.0);
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
                        spideys,
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
                spideys,
                projectiles,
                impacts,
                char_decals,
                screen_height_px,
                burn_config,
            ),
        }
    }

    if loadout.current() != AttackId::Flamethrower {
        state.flamethrower = None;
    }
    if loadout.current() != AttackId::Pistol {
        state.gun_muzzle_flash_elapsed = None;
    }

    // Tick muzzle flash timer; expire when animation duration is exceeded.
    if let Some(elapsed) = &mut state.gun_muzzle_flash_elapsed {
        *elapsed += dt;
        if *elapsed >= sprites.gun_muzzle_flash.duration_secs {
            state.gun_muzzle_flash_elapsed = None;
        }
    }

    update_weapon_presentation(
        state,
        loadout.current(),
        input.moving_forward_back,
        dt,
        elapsed_secs,
        view_bob_amplitude,
        view_bob_freq_mult,
    );
    tick_one_shot_effects(&mut state.one_shots, dt, &state.shared);
    input.clear_edges();
}

fn update_weapon_presentation(
    state: &mut PlayerAttackState,
    current_weapon: AttackId,
    moving_forward_back: bool,
    dt: f32,
    elapsed_secs: f32,
    view_bob_amplitude: f32,
    view_bob_freq_mult: f32,
) {
    let (raise_px, raise_speed, bob_enabled, bob_h, bob_v, bob_speed, bob_return) =
        match current_weapon {
            AttackId::Flamethrower => (
                state.config.weapon_raise_px,
                state.config.weapon_raise_speed,
                state.config.weapon_bob_enabled,
                state.config.weapon_bob_horizontal_px,
                state.config.weapon_bob_vertical_px,
                state.config.weapon_bob_speed,
                state.config.weapon_bob_return_speed,
            ),
            AttackId::Pistol => (
                state.gun_config.weapon_raise_px,
                state.gun_config.weapon_raise_speed,
                state.gun_config.weapon_bob_enabled,
                state.gun_config.weapon_bob_horizontal_px,
                state.gun_config.weapon_bob_vertical_px,
                state.gun_config.weapon_bob_speed,
                state.gun_config.weapon_bob_return_speed,
            ),
        };

    // Weapon stays raised while flame chain is active (draining or spawning),
    // Weapon raised and bob suppressed only while actively spawning flames.
    // Draining flame chain keeps visuals alive but doesn't affect weapon pose.
    let (weapon_raised, suppress_bob) = match current_weapon {
        AttackId::Flamethrower => {
            let spawning = state.flamethrower.as_ref().is_some_and(|ft| ft.spawning);
            (spawning, spawning)
        }
        AttackId::Pistol => {
            let flash = state.gun_muzzle_flash_elapsed.is_some();
            (flash, flash)
        }
    };

    // Weapon raise/lower tween: 0.0 = raised (shooting), raise_px = lowered (idle).
    let raise_target = if weapon_raised { 0.0 } else { raise_px };
    let raise_t = (raise_speed * dt).clamp(0.0, 1.0);
    state.weapon_raise_offset += (raise_target - state.weapon_raise_offset) * raise_t;

    if bob_enabled && moving_forward_back && !suppress_bob {
        let phase = elapsed_secs * bob_speed;
        let horizontal = phase.sin();
        state.weapon_bob_offset = Vec2::new(horizontal * bob_h, -horizontal.abs() * bob_v);
        // Camera view bob: double-frequency vertical oscillation (head bobs
        // at walking cadence, weapon sways at arm cadence).
        state.view_bob = (phase * view_bob_freq_mult).sin() * view_bob_amplitude;
    } else {
        let t = (bob_return * dt).clamp(0.0, 1.0);
        state.weapon_bob_offset = state.weapon_bob_offset.lerp(Vec2::ZERO, t);
        state.view_bob += (0.0 - state.view_bob) * t;
    }
}

#[allow(clippy::too_many_arguments)]
fn update_flamethrower_attack(
    camera: &Camera,
    map: &Map,
    dt: f32,
    _elapsed_secs: f32,
    input: &AttackInput,
    state: &mut PlayerAttackState,
    enemies: &mut [Enemy],
    mosquitons: &mut [Mosquiton],
    spideys: &mut [Spidey],
    projectiles: &mut Vec<Projectile>,
    impacts: &mut Vec<ProjectileImpact>,
    char_decals: &mut Vec<CharDecal>,
    _screen_height_px: f32,
    burn_config: &carcinisation_fps_core::BurnConfig,
) {
    let config = &state.config;
    let shared = &state.shared;
    // Start or restart spawning. If a drain is in progress, re-enable
    // spawning on the existing chain with remaining ammo (no free refill).
    if input.shoot_just_pressed {
        if let Some(active) = &mut state.flamethrower {
            if !active.spawning {
                active.spawning = true;
                active.spawn_cooldown = 0.0;
            }
        } else {
            state.flamethrower = Some(ActiveFpFlamethrower {
                spawning: true,
                ammo: shared.max_ammo,
                elapsed: 0.0,
                spawn_cooldown: 0.0,
                sample_counter: 0,
                samples: Vec::new(),
                wall_impact: None,
                last_decal_impact: None,
            });
        }
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
        active.ammo -= dt * 1000.0 * shared.ammo_drain_per_ms;
    }

    // Age existing samples and expire old ones.
    let max_age = shared.max_stream_age();
    for sample in &mut active.samples {
        sample.age += dt;
    }
    active.samples.retain(|s| s.age < max_age);

    // Emit new samples at the nozzle while firing.
    if active.spawning {
        active.spawn_cooldown -= dt;
        let dir = camera.direction();
        let nozzle_pos = flame_nozzle_position(
            camera.position,
            dir,
            config.nozzle_forward,
            config.nozzle_lateral,
        );
        let emit_interval = shared.emit_interval_ms.get() as f32 / 1000.0;
        while active.spawn_cooldown <= 0.0 {
            let seed = sample_seed(active.sample_counter);
            active.samples.push(FlameStreamSample {
                emit_position: nozzle_pos,
                emit_direction: dir,
                age: 0.0,
                seed,
            });
            active.sample_counter = active.sample_counter.wrapping_add(1);
            active.spawn_cooldown += emit_interval;
        }
    }

    // Wall impact detection.
    active.wall_impact = find_flame_wall_impact(camera, map, config, shared, active);
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
        enemies,
        mosquitons,
        spideys,
        projectiles,
        impacts,
        burn_config,
        shared,
        dt,
    );

    if !active.spawning && active.samples.is_empty() {
        state.flamethrower = None;
    }
}

const fn sample_seed(counter: u32) -> u32 {
    counter.wrapping_mul(0x9E37_79B9) ^ 0xC2B2_AE35
}

fn screen_right_from_direction(dir: Vec2) -> Vec2 {
    Vec2::new(dir.y, -dir.x)
}

fn flame_nozzle_position(
    origin: Vec2,
    dir: Vec2,
    nozzle_forward: f32,
    nozzle_lateral: f32,
) -> Vec2 {
    origin + dir * nozzle_forward + screen_right_from_direction(dir) * nozzle_lateral
}

/// Check if the flame stream reaches a wall along the camera's forward direction.
fn find_flame_wall_impact(
    camera: &Camera,
    map: &Map,
    config: &PlayerFlamethrower1pConfig,
    shared: &carcinisation_fps_core::PlayerFlamethrowerConfig,
    active: &ActiveFpFlamethrower,
) -> Option<FlameWallImpact> {
    let dir = camera.direction();
    let ray_hit = cast_ray(map, camera.position, dir);
    if ray_hit.wall_id == 0 {
        return None;
    }
    let wall_dist = ray_hit.distance;
    let max_reach = config.nozzle_forward + active.elapsed * shared.speed;
    if max_reach < wall_dist || wall_dist > shared.range + config.nozzle_forward {
        return None;
    }
    let surface_id = ray_hit.surface_id?;
    Some(FlameWallImpact {
        surface_id,
        u: ray_hit.wall_x,
        v: 0.5,
        seed: wall_impact_seed(surface_id, ray_hit.wall_x, 0.5),
    })
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

const fn adjacent_wall_surface(surface_id: WallSurfaceId, tangent_step: i32) -> WallSurfaceId {
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
    if state.flamethrower.is_none() {
        return;
    }
    let flame_dir = camera.direction();
    for projectile in projectiles.iter_mut() {
        if projectile.alive
            && carcinisation_fps_core::flame_hits_position_configured(
                camera.position,
                flame_dir,
                projectile.position,
                map,
                &state.shared,
            )
        {
            projectile.alive = false;
            impacts.push(ProjectileImpact::destroy(
                projectile.position,
                projectile.kind,
                0.0,
            ));
        }
    }
    projectiles.retain(|p| p.alive);
}

#[allow(clippy::too_many_arguments)]
fn apply_flamethrower_damage(
    camera: &Camera,
    map: &Map,
    enemies: &mut [Enemy],
    mosquitons: &mut [Mosquiton],
    spideys: &mut [Spidey],
    projectiles: &mut Vec<Projectile>,
    impacts: &mut Vec<ProjectileImpact>,
    burn_config: &carcinisation_fps_core::BurnConfig,
    flame_cfg: &carcinisation_fps_core::PlayerFlamethrowerConfig,
    dt: f32,
) {
    let flame_dir = camera.direction();

    for enemy in enemies.iter_mut() {
        if !enemy.is_alive() {
            continue;
        }
        if carcinisation_fps_core::flame_hits_position_configured(
            camera.position,
            flame_dir,
            enemy.position,
            map,
            flame_cfg,
        ) {
            carcinisation_fps_core::apply_exposure(
                &mut enemy.burn_state,
                burn_config,
                burn_config.flame_exposure_per_sec,
                dt,
            );
        }
    }

    for mosquiton in mosquitons.iter_mut() {
        if !mosquiton.is_alive() {
            continue;
        }
        if carcinisation_fps_core::flame_hits_position_configured(
            camera.position,
            flame_dir,
            mosquiton.position,
            map,
            flame_cfg,
        ) {
            carcinisation_fps_core::apply_exposure(
                &mut mosquiton.burn_state,
                burn_config,
                burn_config.flame_exposure_per_sec,
                dt,
            );
        }
    }

    for spidey in spideys.iter_mut() {
        if !spidey.is_alive() {
            continue;
        }
        if carcinisation_fps_core::flame_hits_position_configured(
            camera.position,
            flame_dir,
            spidey.position,
            map,
            flame_cfg,
        ) {
            carcinisation_fps_core::apply_exposure(
                &mut spidey.burn_state,
                burn_config,
                burn_config.flame_exposure_per_sec,
                dt,
            );
        }
    }

    destroy_projectiles_touching_flame(camera, map, projectiles, impacts, flame_cfg);
}

fn destroy_projectiles_touching_flame(
    camera: &Camera,
    map: &Map,
    projectiles: &mut Vec<Projectile>,
    impacts: &mut Vec<ProjectileImpact>,
    flame_cfg: &carcinisation_fps_core::PlayerFlamethrowerConfig,
) {
    let flame_dir = camera.direction();
    for projectile in projectiles.iter_mut() {
        if projectile.alive
            && carcinisation_fps_core::flame_hits_position_configured(
                camera.position,
                flame_dir,
                projectile.position,
                map,
                flame_cfg,
            )
        {
            projectile.alive = false;
            impacts.push(ProjectileImpact::destroy(
                projectile.position,
                projectile.kind,
                0.0,
            ));
        }
    }
    projectiles.retain(|projectile| projectile.alive);
}

// flame_hits_target and helpers (flame_local_hit_point, retain_closest_hit,
// closest_point_on_segment) removed — replaced by fps_core::flame_hits_position.

fn tick_one_shot_effects(
    effects: &mut Vec<OneShotEffect>,
    dt: f32,
    shared: &carcinisation_fps_core::PlayerFlamethrowerConfig,
) {
    let max_duration = shared.max_stream_age();
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
    spideys: &mut [Spidey],
    projectiles: &mut Vec<Projectile>,
    impacts: &mut Vec<ProjectileImpact>,
    damage: u32,
    max_range: Option<f32>,
) {
    let enemy_hit = hitscan(camera, enemies, map);
    let mosquiton_hit = hitscan_mosquitons(camera, mosquitons, map);
    let spidey_hit = hitscan_spideys(camera, spideys, map);
    let projectile_hit = hitscan_projectiles(camera, projectiles, map);

    let mut hit = enemy_hit
        .enemy_idx
        .map(|enemy_idx| (FpShotHit::Enemy(enemy_idx), enemy_hit.distance));
    if let Some((mosquiton_idx, distance)) = mosquiton_hit
        && hit.is_none_or(|(_, current_distance)| distance < current_distance)
    {
        hit = Some((FpShotHit::Mosquiton(mosquiton_idx), distance));
    }
    if let Some((spidey_idx, distance)) = spidey_hit
        && hit.is_none_or(|(_, current_distance)| distance < current_distance)
    {
        hit = Some((FpShotHit::Spidey(spidey_idx), distance));
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
        FpShotHit::Spidey(spidey_idx) => spideys[spidey_idx].take_damage(damage),
        FpShotHit::Projectile(projectile_idx) => {
            if let Some(projectile) = projectiles.get_mut(projectile_idx) {
                projectile.alive = false;
                impacts.push(ProjectileImpact::destroy(
                    projectile.position,
                    projectile.kind,
                    0.0,
                ));
            }
            projectiles.retain(|projectile| projectile.alive);
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum FpShotHit {
    Enemy(usize),
    Mosquiton(usize),
    Spidey(usize),
    Projectile(usize),
}

pub fn draw_player_attack_overlays(
    image: &mut CxImage,
    _camera: &Camera,
    _map: &Map,
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
        let screen_height = image.height() as f32;
        let presentation_offset =
            state.weapon_bob_offset + Vec2::new(0.0, state.weapon_raise_offset);
        let weapon_center = flamethrower_weapon_center(screen_height, config, presentation_offset);

        // Flame chain is now rendered as world-space billboards via
        // flame_chain_billboards() — pushed to ExtraBillboards by the plugin.
        // Only the idle flame and weapon sprite remain as screen-space overlays.

        if state.flamethrower.is_none() {
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
    } else if loadout.current() == AttackId::Pistol {
        let gun_config = &state.gun_config;
        let screen_height = image.height() as f32;
        let presentation_offset =
            state.weapon_bob_offset + Vec2::new(0.0, state.weapon_raise_offset);
        let weapon_center = gun_weapon_center(screen_height, gun_config, presentation_offset);

        // Muzzle flash (drawn behind weapon).
        if let Some(elapsed) = state.gun_muzzle_flash_elapsed {
            let flash_offset = gun_config.muzzle_flash_offset();
            draw_image_scaled_center(
                image,
                sprites.gun_muzzle_flash.frame_clamped(elapsed),
                weapon_center + flash_offset,
                gun_config.muzzle_flash_scale,
            );
        }

        // Gun weapon sprite: idle shows first frame only, shooting loops.
        let gun_frame = if state.gun_muzzle_flash_elapsed.is_some() {
            sprites.gun_shooting.frame_loop(elapsed_secs)
        } else {
            sprites.gun_idle.frame_clamped(0.0)
        };
        draw_image_scaled_center(image, gun_frame, weapon_center, 1.0);
    }
}

fn flamethrower_weapon_animation<'a>(
    sprites: &'a PlayerAttackSprites,
    state: &PlayerAttackState,
) -> &'a AtlasAnimation {
    if state.flamethrower.as_ref().is_some_and(|ft| ft.spawning) {
        &sprites.weapon_shooting
    } else {
        &sprites.weapon_idle
    }
}

fn gun_weapon_center(screen_height: f32, config: &GunConfig, presentation_offset: Vec2) -> Vec2 {
    Vec2::new(80.0, screen_height - 20.0) + config.weapon_base_offset() + presentation_offset
}

fn flamethrower_weapon_center(
    screen_height: f32,
    config: &PlayerFlamethrower1pConfig,
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
    let frames: Vec<Arc<CxImage>> = region
        .frames
        .iter()
        .map(|rect| extract_atlas_rect(&atlas_pixels, atlas_width, *rect).map(Arc::new))
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
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use bevy::prelude::{IVec2, UVec2};

    fn open_test_map() -> Map {
        Map {
            width: 32,
            height: 32,
            cells: vec![0; 32 * 32],
        }
    }

    #[test]
    fn fps_attack_configs_load() {
        let _ = GroundFireVisualConfig::load();
        let _ = PlayerFlamethrower1pConfig::load();
        let _ = GunConfig::load();
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
        assert_eq!(sprites.idle_flame.frames.len(), 4);
        assert_eq!(sprites.idle_flame.frames[0].size(), UVec2::new(6, 8));
        assert_eq!(sprites.gun_idle.frames.len(), 4);
        assert_eq!(sprites.gun_shooting.frames.len(), 4);
        assert_eq!(sprites.gun_muzzle_flash.frames.len(), 4);
    }

    #[test]
    fn gun_muzzle_flash_spawns_on_shoot_and_expires() {
        let sprites = PlayerAttackSprites::load();
        let mut state = PlayerAttackState::default();
        let mut loadout = AttackLoadout::default();
        loadout.cycle(); // switch to Pistol
        assert_eq!(loadout.current(), AttackId::Pistol);

        // No flash initially.
        assert!(state.gun_muzzle_flash_elapsed.is_none());

        // Simulate a shot.
        let camera = Camera::default();
        let map = Map {
            width: 8,
            height: 8,
            cells: vec![0; 64],
        };
        let mut input = AttackInput {
            shoot_just_pressed: true,
            shoot_held: true,
            cursor_x: 80.0,
            ..Default::default()
        };
        let mut enemies = Vec::new();
        let mut mosquitons = Vec::new();
        let mut projectiles = Vec::new();
        let mut impacts = Vec::new();
        let mut char_decals = Vec::new();
        let mut shoot = false;

        process_player_attacks(
            &camera,
            &map,
            &sprites,
            37,
            1.0 / 60.0,
            0.0,
            &mut input,
            &mut loadout,
            &mut state,
            &mut enemies,
            &mut mosquitons,
            &mut [],
            &mut projectiles,
            &mut impacts,
            &mut char_decals,
            144.0,
            &mut shoot,
            &carcinisation_fps_core::BurnConfig::default(),
            1.5,
            2.0,
        );

        // Flash should be active.
        assert!(state.gun_muzzle_flash_elapsed.is_some());

        // Tick past the flash duration.
        input.shoot_just_pressed = false;
        input.shoot_held = false;
        process_player_attacks(
            &camera,
            &map,
            &sprites,
            37,
            sprites.gun_muzzle_flash.duration_secs + 0.01,
            1.0,
            &mut input,
            &mut loadout,
            &mut state,
            &mut enemies,
            &mut mosquitons,
            &mut [],
            &mut projectiles,
            &mut impacts,
            &mut char_decals,
            144.0,
            &mut shoot,
            &carcinisation_fps_core::BurnConfig::default(),
            1.5,
            2.0,
        );

        // Flash should have expired.
        assert!(state.gun_muzzle_flash_elapsed.is_none());
    }

    #[test]
    fn gun_muzzle_flash_clears_on_weapon_switch() {
        let sprites = PlayerAttackSprites::load();
        let mut state = PlayerAttackState::default();
        let mut loadout = AttackLoadout::default();
        loadout.cycle(); // Pistol

        state.gun_muzzle_flash_elapsed = Some(0.05);

        let camera = Camera::default();
        let map = Map {
            width: 8,
            height: 8,
            cells: vec![0; 64],
        };
        let mut input = AttackInput {
            cycle_requested: true,
            cursor_x: 80.0,
            ..Default::default()
        };
        let mut enemies = Vec::new();
        let mut mosquitons = Vec::new();
        let mut projectiles = Vec::new();
        let mut impacts = Vec::new();
        let mut char_decals = Vec::new();
        let mut shoot = false;

        process_player_attacks(
            &camera,
            &map,
            &sprites,
            37,
            1.0 / 60.0,
            0.0,
            &mut input,
            &mut loadout,
            &mut state,
            &mut enemies,
            &mut mosquitons,
            &mut [],
            &mut projectiles,
            &mut impacts,
            &mut char_decals,
            144.0,
            &mut shoot,
            &carcinisation_fps_core::BurnConfig::default(),
            1.5,
            2.0,
        );

        assert_eq!(loadout.current(), AttackId::Flamethrower);
        assert!(state.gun_muzzle_flash_elapsed.is_none());
    }

    #[test]
    fn flamethrower_weapon_animation_follows_spawning_state() {
        let sprites = PlayerAttackSprites::load();
        let mut state = PlayerAttackState::default();

        // No flame → idle animation (1 frame).
        assert_eq!(
            flamethrower_weapon_animation(&sprites, &state).frames.len(),
            1
        );

        // Draining (spawning=false, samples exist) → idle animation.
        state.flamethrower = Some(ActiveFpFlamethrower {
            spawning: false,
            ammo: 0.0,
            elapsed: 0.0,
            spawn_cooldown: 0.0,
            sample_counter: 0,
            samples: vec![FlameStreamSample {
                emit_position: Vec2::ZERO,
                emit_direction: Vec2::Y,
                age: 0.0,
                seed: 0,
            }],
            wall_impact: None,
            last_decal_impact: None,
        });
        assert_eq!(
            flamethrower_weapon_animation(&sprites, &state).frames.len(),
            1,
            "draining flame should use idle animation"
        );

        // Actively spawning → shooting animation (2 frames).
        state.flamethrower.as_mut().unwrap().spawning = true;
        assert_eq!(
            flamethrower_weapon_animation(&sprites, &state).frames.len(),
            2,
            "spawning flame should use shooting animation"
        );
    }

    #[test]
    fn idle_nozzle_flame_renders_behind_weapon() {
        let config = PlayerFlamethrower1pConfig::load();
        let sprites = PlayerAttackSprites::load();
        let idle_frame = sprites.idle_flame.frame_loop(0.0);
        let weapon_frame = sprites.weapon_idle.frame_loop(0.0);

        let flame_center_y = idle_frame.height() as f32 * config.idle_flame_scale * 0.5;
        let (ox, oy) = config.idle_flame_offset();
        let idle_flame_center = Vec2::new(ox, oy - flame_center_y);

        let idle_center_y = 124.0 + config.weapon_raise_px;
        let weapon_tl_x = 80 - weapon_frame.width() as i32 / 2;
        let weapon_tl_y = idle_center_y as i32 - weapon_frame.height() as i32 / 2;

        let flame_tl_x = (80.0 + idle_flame_center.x
            - idle_frame.width() as f32 * config.idle_flame_scale * 0.5)
            .round() as i32;
        let flame_tl_y = (idle_center_y + idle_flame_center.y - flame_center_y).round() as i32;

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
    fn stream_sample_advects_along_direction() {
        let sample = FlameStreamSample {
            emit_position: Vec2::new(1.0, 2.0),
            emit_direction: Vec2::new(1.0, 0.0),
            age: 0.5,
            seed: 0,
        };
        let pos = sample.world_position(10.0);
        assert!((pos.x - 6.0).abs() < f32::EPSILON);
        assert!((pos.y - 2.0).abs() < f32::EPSILON);
    }

    #[test]
    fn max_stream_age_is_range_over_speed() {
        let shared = carcinisation_fps_core::PlayerFlamethrowerConfig::load();
        let expected = shared.range / shared.speed;
        assert!((shared.max_stream_age() - expected).abs() < f32::EPSILON);
    }

    #[test]
    fn weapon_bob_is_high_at_horizontal_extremes() {
        let config = PlayerFlamethrower1pConfig::load();
        let bob = |t: f32| -> Vec2 {
            let phase = t * config.weapon_bob_speed;
            let h = phase.sin();
            Vec2::new(
                h * config.weapon_bob_horizontal_px,
                -h.abs() * config.weapon_bob_vertical_px,
            )
        };
        let center = bob(0.0);
        let extreme = bob(std::f32::consts::FRAC_PI_2 / config.weapon_bob_speed);

        assert!(center.x.abs() < 0.01);
        assert!(center.y.abs() < 0.01);
        assert!((extreme.x - config.weapon_bob_horizontal_px).abs() < 0.01);
        assert!((extreme.y + config.weapon_bob_vertical_px).abs() < 0.01);
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
        let camera = Camera {
            position: Vec2::new(1.5, 1.5),
            angle: 0.0,
            fov: 1.0,
            ..Default::default()
        };
        let mut map = Map {
            width: 4,
            height: 3,
            cells: vec![0; 12],
        };
        map.cells[map.width + 2] = 1; // wall at (2,1)
        let target = Vec2::new(2.5, 1.5); // behind wall

        let cfg = carcinisation_fps_core::PlayerFlamethrowerConfig::load();
        assert!(!carcinisation_fps_core::flame_hits_position_configured(
            camera.position,
            camera.direction(),
            target,
            &map,
            &cfg,
        ));
    }
}
