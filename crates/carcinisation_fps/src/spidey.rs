//! FP Spidey enemy — hopping ground attacker with web ranged and lunge melee.
//!
//! Pure sim in `carcinisation_fps_core::spidey`. This module adds the ECS
//! wrapper, billboard sprites, and damage/hitscan utilities.

use bevy::prelude::Component;
use bevy_math::Vec2;
use carcinisation_fps_core::burning::BurnState;
use carcinisation_fps_core::fire_death::DamageKind;

use carcinisation_fps_core::FpsCombatConfig;

use crate::camera::Camera;
use crate::enemy::{DamageFlicker, Projectile};
use crate::map::Map;

/// Billboard height in map units. Shared between SP and MP.
pub const SPIDEY_BILLBOARD_HEIGHT: f32 = 0.45;
/// Minimum `visual_height` to detect a mid-hop (vs grounded).
pub const SPIDEY_HOP_DETECTION_THRESHOLD: f32 = 0.02;
/// Floor level relative to horizon. Billboard bottom anchors here.
pub const FLOOR_OFFSET: f32 = -0.5;

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

/// Configuration for FP Spidey behaviour.
///
/// Contains `SpideySimConfig` for gameplay and adds presentation-only fields.
#[derive(Clone, Debug)]
pub struct SpideyConfig {
    /// Core sim config (gameplay parameters).
    pub sim: carcinisation_fps_core::spidey::SpideySimConfig,
    /// Billboard rendering height (map-space units).
    pub billboard_height: f32,
    pub health: u32,
    /// `WebShot` slow multiplier applied to player movement on hit.
    pub web_slow_multiplier: f32,
    /// `WebShot` slow duration in seconds.
    pub web_slow_duration: f32,
    /// Projectile lifetime in seconds (from `FpsCombatConfig`).
    pub projectile_lifetime: f32,
}

impl Default for SpideyConfig {
    fn default() -> Self {
        Self::from_combat_config(&FpsCombatConfig::default())
    }
}

impl SpideyConfig {
    /// Build from a loaded `FpsCombatConfig` resource (uses RON values, not Rust defaults).
    #[must_use]
    pub const fn from_combat_config(combat: &carcinisation_fps_core::FpsCombatConfig) -> Self {
        Self {
            sim: combat.spidey_sim_config(),
            billboard_height: SPIDEY_BILLBOARD_HEIGHT,
            health: combat.spidey.health,
            web_slow_multiplier: combat.spidey.web_slow_multiplier,
            web_slow_duration: combat.spidey.web_slow_duration,
            projectile_lifetime: combat.projectile_lifetime,
        }
    }

    /// Apply map-authored movement speed to hop/lunge movement fields.
    #[must_use]
    pub fn with_authored_speed(mut self, speed: f32) -> Self {
        self.sim = self.sim.with_authored_speed(speed);
        self
    }
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

/// FP Spidey AI state — re-exported from `fps_core` which now derives `Component`.
pub type SpideyState = carcinisation_fps_core::spidey::SpideySimState;

// ---------------------------------------------------------------------------
// Tick result
// ---------------------------------------------------------------------------

/// Results produced by ticking FP Spidey AI.
#[derive(Clone, Debug, Default)]
pub struct SpideyTickResult {
    pub projectiles: Vec<Projectile>,
    pub player_damage: u32,
    pub damage_source: Option<Vec2>,
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

/// A runtime FP Spidey instance.
#[derive(Clone, Debug, Component)]
pub struct Spidey {
    pub position: Vec2,
    /// Visual height from sim output (hop/lunge arc).
    pub visual_height: f32,
    pub velocity: Vec2,
    pub animation_time: f32,
    pub health: u32,
    pub max_health: u32,
    pub state: SpideyState,
    pub web_cooldown: f32,
    pub lunge_cooldown: f32,
    /// When `Some(elapsed)`, a web animation is playing. The projectile
    /// spawns when `elapsed >= config.web_cue_secs`.
    pub web_anim_elapsed: Option<f32>,
    pub config: SpideyConfig,
    pub damage_flicker: Option<DamageFlicker>,
    /// Stable per-instance seed for deterministic sim decisions.
    pub seed: u32,
    pub burn_state: BurnState,
    /// Hit-reaction runtime state (poise, stun, knockback) — round-tripped
    /// through the shared sim each tick, written by the hitscan damage path.
    pub reaction: carcinisation_fps_core::EnemyReactionState,
}

impl Spidey {
    #[must_use]
    pub fn new(position: Vec2, config: SpideyConfig) -> Self {
        let health = config.health;
        Self {
            position,
            visual_height: 0.0,
            velocity: Vec2::ZERO,
            animation_time: 0.0,
            health,
            max_health: health,
            state: SpideyState::Idle,
            web_cooldown: 0.0,
            lunge_cooldown: 0.0,
            web_anim_elapsed: None,
            config,
            damage_flicker: None,
            burn_state: BurnState::default(),
            seed: carcinisation_fps_core::corpse_seed(position),
            reaction: carcinisation_fps_core::EnemyReactionState::default(),
        }
    }

    #[must_use]
    pub const fn is_alive(&self) -> bool {
        !matches!(
            self.state,
            SpideyState::Dying { .. } | SpideyState::BurningCorpse { .. } | SpideyState::Dead
        )
    }

    pub fn take_damage(&mut self, amount: u32) {
        self.take_damage_from(amount, DamageKind::Physical, 0.5);
    }

    pub fn take_damage_from(&mut self, amount: u32, kind: DamageKind, fire_death_secs: f32) {
        if !self.is_alive() {
            return;
        }
        match carcinisation_fps_core::apply_damage(
            &mut self.health,
            &mut self.damage_flicker,
            amount,
            kind,
            fire_death_secs,
            self.position,
        ) {
            carcinisation_fps_core::DamageOutcome::Survived => {}
            carcinisation_fps_core::DamageOutcome::KilledPhysical => {
                self.web_anim_elapsed = None;
                self.state = SpideyState::Dying {
                    timer: self.config.sim.death_secs,
                };
                self.velocity = Vec2::ZERO;
            }
            carcinisation_fps_core::DamageOutcome::KilledByFire { timer, seed } => {
                self.web_anim_elapsed = None;
                self.state = SpideyState::BurningCorpse { timer, seed };
                self.velocity = Vec2::ZERO;
            }
        }
    }

    #[must_use]
    pub fn showing_damage_invert(&self) -> bool {
        self.is_alive() && carcinisation_fps_core::is_showing_damage_invert(&self.damage_flicker)
    }
}

// ---------------------------------------------------------------------------
// Presentation state adapter (SP)
// ---------------------------------------------------------------------------

use carcinisation_fps_core::presentation::{AttackPresentationKind, EnemyPresentationState};

/// Convert local `SpideyState` into the shared `EnemyPresentationState`.
///
/// This is the SP adapter: single-player has full sim state, so the mapping is
/// precise. The MP adapter lives in the app crate where both `fps` and `net`
/// types are available.
#[must_use]
pub fn spidey_presentation_state(
    state: &SpideyState,
    animation_time: f32,
    visual_height: f32,
) -> EnemyPresentationState {
    match state {
        SpideyState::Idle | SpideyState::HopWait { .. } => EnemyPresentationState::Idle,
        SpideyState::HopMove {
            timer, duration, ..
        } => {
            let phase = 1.0 - timer / duration.max(f32::EPSILON);
            EnemyPresentationState::Hopping {
                phase,
                visual_height,
            }
        }
        SpideyState::WebWindup { .. } => EnemyPresentationState::Windup {
            attack: AttackPresentationKind::Ranged,
            phase: animation_time,
        },
        SpideyState::LungeWindup { .. } => EnemyPresentationState::Windup {
            attack: AttackPresentationKind::Melee,
            phase: animation_time,
        },
        SpideyState::LungeAttack { .. } => EnemyPresentationState::Attacking {
            attack: AttackPresentationKind::Melee,
            phase: animation_time,
        },
        SpideyState::Recover { .. } => EnemyPresentationState::Recover,
        SpideyState::Dying { .. } => EnemyPresentationState::Dying {
            burn: false,
            phase: animation_time,
        },
        SpideyState::BurningCorpse { .. } => EnemyPresentationState::Dying {
            burn: true,
            phase: animation_time,
        },
        SpideyState::Dead => EnemyPresentationState::Dead { burn: false },
    }
}

// ---------------------------------------------------------------------------
// Tick
// ---------------------------------------------------------------------------

/// Tick a single Spidey for one frame.
///
/// Delegates gameplay logic to `carcinisation_fps_core::spidey::tick_spidey_sim`
/// and handles rendering concerns (animation time, damage flicker, velocity,
/// visual height).
#[must_use]
pub fn tick_single_spidey(
    spidey: &mut Spidey,
    player_pos: Vec2,
    map: &Map,
    dt: f32,
) -> (Option<Projectile>, Option<(u32, Vec2)>) {
    use carcinisation_fps_core::spidey::{SpideySim, tick_spidey_sim};

    // Tick rendering-only state.
    if let Some(flicker) = spidey.damage_flicker {
        spidey.damage_flicker = flicker.tick(dt);
    }
    if !matches!(
        spidey.state,
        SpideyState::Dead | SpideyState::BurningCorpse { .. }
    ) {
        spidey.animation_time += dt;
    }

    // Track pre-tick state for animation change detection.
    let was_hop = matches!(spidey.state, SpideyState::HopMove { .. });

    let mut sim = SpideySim {
        position: spidey.position,
        state: spidey.state.clone(),
        web_cooldown: spidey.web_cooldown,
        lunge_cooldown: spidey.lunge_cooldown,
        web_anim_elapsed: spidey.web_anim_elapsed,
        seed: spidey.seed,
        reaction: spidey.reaction,
    };

    let output = tick_spidey_sim(&mut sim, &spidey.config.sim, player_pos, map, dt);

    // Write back sim state.
    spidey.position = sim.position;
    spidey.state = sim.state;
    spidey.web_cooldown = sim.web_cooldown;
    spidey.lunge_cooldown = sim.lunge_cooldown;
    spidey.web_anim_elapsed = sim.web_anim_elapsed;
    spidey.seed = sim.seed;
    spidey.reaction = sim.reaction;
    spidey.velocity = output.velocity;
    spidey.visual_height = output.visual_height;

    // Reset animation time on one-shot animation starts (rendering concern).
    let hop_started = !was_hop && matches!(spidey.state, SpideyState::HopMove { .. });
    if output.started_lunge || output.started_web_anim || hop_started {
        spidey.animation_time = 0.0;
    }

    // Tag web projectiles with WebShot kind + override lifetime from loaded config.
    let projectile = output.projectile.map(|mut p| {
        p.kind = carcinisation_fps_core::ProjectileKind::WebShot {
            slow_multiplier: spidey.config.web_slow_multiplier,
            slow_duration: spidey.config.web_slow_duration,
        };
        p.initial_lifetime = spidey.config.projectile_lifetime;
        p.lifetime = spidey.config.projectile_lifetime;
        p
    });

    (projectile, output.melee_damage)
}

/// Tick all Spideys. Returns spawned projectiles and direct melee damage.
#[must_use]
pub fn tick_spideys(
    spideys: &mut [Spidey],
    player_pos: Vec2,
    map: &Map,
    dt: f32,
) -> SpideyTickResult {
    let mut result = SpideyTickResult::default();

    for s in spideys.iter_mut() {
        let (proj, dmg) = tick_single_spidey(s, player_pos, map, dt);
        if let Some(p) = proj {
            result.projectiles.push(p);
        }
        if let Some((amount, source)) = dmg {
            result.player_damage += amount;
            result.damage_source = Some(source);
        }
    }

    result
}

// ---------------------------------------------------------------------------
// Hitscan
// ---------------------------------------------------------------------------

/// Hitscan check against Spideys. Returns index of closest hit.
#[must_use]
pub fn hitscan_spideys(camera: &Camera, spideys: &[Spidey], map: &Map) -> Option<(usize, f32)> {
    carcinisation_fps_core::hitscan_generic(
        camera,
        map,
        spideys
            .iter()
            .map(|s| (s.position, s.config.sim.collision_radius, s.is_alive())),
    )
}

// ---------------------------------------------------------------------------
// Spidey billboard sprites
// ---------------------------------------------------------------------------

use crate::mosquiton::MosquitonBillboardFrame;
use carapace::image::CxImage;
use std::sync::Arc;

const SPIDEY_COMPOSED_RON: &str =
    include_str!("../../../assets/sprites/enemies/spidey_3/atlas.composed.ron");
const SPIDEY_PX_ATLAS_RON: &str =
    include_str!("../../../assets/sprites/enemies/spidey_3/atlas.px_atlas.ron");
const SPIDEY_PXI: &[u8] = include_bytes!("../../../assets/sprites/enemies/spidey_3/atlas.pxi");

/// Palette-indexed FP Spidey billboard sprites resolved from composed output.
///
/// Sprites are `Arc`-wrapped so billboard construction can share them
/// via `Arc::clone()` instead of deep-copying pixel data.
#[derive(Clone, Debug)]
pub struct SpideyBillboardSprites {
    pub alive: Vec<MosquitonBillboardFrame>,
    pub shoot: Vec<MosquitonBillboardFrame>,
    /// Hop/jump animation (`front_jump`). Used during `HopMove`.
    pub hop: Vec<MosquitonBillboardFrame>,
    /// Lunge attack animation (`front_lunge` tag in Aseprite).
    /// Used during LungeWindup/LungeAttack. Clamped to last frame until landing.
    pub lunge: Vec<MosquitonBillboardFrame>,
    pub death: Arc<CxImage>,
}

impl SpideyBillboardSprites {
    /// Sample the looping idle animation.
    #[must_use]
    pub fn alive_sprite_at(&self, elapsed_secs: f32) -> &Arc<CxImage> {
        crate::mosquiton::animation_sprite_at(&self.alive, elapsed_secs)
    }

    /// Sample the one-shot shoot (web) animation, clamped to the last frame.
    #[must_use]
    pub fn shoot_sprite_at(&self, elapsed_secs: f32) -> &Arc<CxImage> {
        if self.shoot.is_empty() {
            return self.alive_sprite_at(elapsed_secs);
        }
        crate::mosquiton::animation_sprite_at_clamped(&self.shoot, elapsed_secs)
    }

    /// Sample the hop (jump) animation, clamped to the last frame.
    #[must_use]
    pub fn hop_sprite_at(&self, elapsed_secs: f32) -> &Arc<CxImage> {
        if self.hop.is_empty() {
            return self.alive_sprite_at(elapsed_secs);
        }
        crate::mosquiton::animation_sprite_at_clamped(&self.hop, elapsed_secs)
    }

    /// Sample the lunge animation, clamped to the last frame.
    #[must_use]
    pub fn lunge_sprite_at(&self, elapsed_secs: f32) -> &Arc<CxImage> {
        if self.lunge.is_empty() {
            return self.alive_sprite_at(elapsed_secs);
        }
        crate::mosquiton::animation_sprite_at_clamped(&self.lunge, elapsed_secs)
    }

    /// Total duration of the hop/jump animation in seconds.
    #[must_use]
    pub fn hop_total_duration(&self) -> f32 {
        self.hop.iter().map(|f| f.duration.max(0.0)).sum()
    }

    /// Total duration of the lunge animation in seconds.
    #[must_use]
    pub fn lunge_total_duration(&self) -> f32 {
        self.lunge.iter().map(|f| f.duration.max(0.0)).sum()
    }
}

/// Resolve the embedded Spidey composed output into FP billboard sprites.
///
/// Uses the same compact composed atlas and PXI atlas generated for the
/// ORS Spidey. FP uses front-facing direction only.
///
/// # Errors
///
/// Returns an error if embedded generated assets are malformed.
pub fn make_spidey_billboard_sprites() -> Result<SpideyBillboardSprites, String> {
    use crate::mosquiton::{PxAtlasDescriptor, compose_animation_frames_full, decode_pxi};
    use asset_pipeline::composed_ron::CompactComposedAtlas;
    use carcinisation_base::direction::SpriteDirection;

    let composed: CompactComposedAtlas =
        ron::from_str(SPIDEY_COMPOSED_RON).map_err(|err| err.to_string())?;
    let atlas: PxAtlasDescriptor =
        ron::from_str(SPIDEY_PX_ATLAS_RON).map_err(|err| err.to_string())?;
    let (atlas_width, _atlas_height, atlas_pixels) = decode_pxi(SPIDEY_PXI)?;

    let idle_tag = SpriteDirection::Front.tag_name("idle");
    let shoot_tag = SpriteDirection::Front.tag_name("shoot");
    let jump_tag = SpriteDirection::Front.tag_name("jump");
    let lunge_tag = SpriteDirection::Front.tag_name("lunge");
    let landing_tag = SpriteDirection::Front.tag_name("landing");

    let alive =
        compose_animation_frames_full(&composed, &atlas, &atlas_pixels, atlas_width, &idle_tag)?;
    let shoot =
        compose_animation_frames_full(&composed, &atlas, &atlas_pixels, atlas_width, &shoot_tag)?;
    let hop =
        compose_animation_frames_full(&composed, &atlas, &atlas_pixels, atlas_width, &jump_tag)?;
    let lunge =
        compose_animation_frames_full(&composed, &atlas, &atlas_pixels, atlas_width, &lunge_tag)?;

    // Death frame: first frame of front_landing animation.
    let landing_frames =
        compose_animation_frames_full(&composed, &atlas, &atlas_pixels, atlas_width, &landing_tag)?;
    let death = landing_frames
        .into_iter()
        .next()
        .map(|f| f.sprite)
        .ok_or_else(|| "Spidey landing animation has no frames for death sprite".to_string())?;

    // Pad all sprites to a unified canvas so switching animations doesn't
    // resize the billboard. Find max dimensions across all tags + death.
    let all_sprites: Vec<&Arc<CxImage>> = alive
        .iter()
        .chain(shoot.iter())
        .chain(hop.iter())
        .chain(lunge.iter())
        .map(|f| &f.sprite)
        .chain(std::iter::once(&death))
        .collect();
    let max_w = all_sprites.iter().map(|s| s.width()).max().unwrap_or(1);
    let max_h = all_sprites.iter().map(|s| s.height()).max().unwrap_or(1);

    let pad = |frames: Vec<MosquitonBillboardFrame>| -> Vec<MosquitonBillboardFrame> {
        frames
            .into_iter()
            .map(|f| MosquitonBillboardFrame {
                sprite: Arc::new(pad_sprite_centered(&f.sprite, max_w, max_h)),
                duration: f.duration,
            })
            .collect()
    };
    let death = Arc::new(pad_sprite_centered(&death, max_w, max_h));

    Ok(SpideyBillboardSprites {
        alive: pad(alive),
        shoot: pad(shoot),
        hop: pad(hop),
        lunge: pad(lunge),
        death,
    })
}

/// Pad a sprite to `target_w × target_h`, centering the original content.
/// Transparent pixels fill the padding.
fn pad_sprite_centered(sprite: &CxImage, target_w: usize, target_h: usize) -> CxImage {
    use carapace::palette::TRANSPARENT_INDEX;

    let src_w = sprite.width();
    let src_h = sprite.height();
    if src_w == target_w && src_h == target_h {
        return sprite.clone();
    }
    let mut data = vec![TRANSPARENT_INDEX; target_w * target_h];
    let off_x = (target_w.saturating_sub(src_w)) / 2;
    let off_y = (target_h.saturating_sub(src_h)) / 2;
    let pixels = sprite.data();
    for y in 0..src_h {
        for x in 0..src_w {
            data[(off_y + y) * target_w + (off_x + x)] = pixels[y * src_w + x];
        }
    }
    CxImage::new(data, target_w)
}

// ---------------------------------------------------------------------------
// Spider-shot billboard sprites
// ---------------------------------------------------------------------------

const SPIDER_SHOT_PX_ATLAS_RON: &str =
    include_str!("../../../assets/sprites/attacks/spider_shot/atlas.px_atlas.ron");
const SPIDER_SHOT_PXI: &[u8] =
    include_bytes!("../../../assets/sprites/attacks/spider_shot/atlas.pxi");

/// Palette-indexed FP spider-shot billboard sprites.
#[derive(Clone, Debug)]
pub struct SpiderShotBillboardSprites {
    pub hover: Arc<CxImage>,
    pub hit: Arc<CxImage>,
    pub destroy: Vec<MosquitonBillboardFrame>,
}

impl SpiderShotBillboardSprites {
    #[must_use]
    pub fn destroy_sprite_at(&self, elapsed_secs: f32) -> &Arc<CxImage> {
        // Clamp to last frame (one-shot animation).
        let mut t = elapsed_secs;
        for frame in &self.destroy {
            if t < frame.duration {
                return &frame.sprite;
            }
            t -= frame.duration;
        }
        self.destroy.last().map_or(&self.hover, |f| &f.sprite)
    }
}

/// Load spider-shot sprites from embedded atlas assets.
///
/// # Errors
/// Returns an error if embedded generated assets are malformed.
pub fn make_spider_shot_billboard_sprites() -> Result<SpiderShotBillboardSprites, String> {
    use crate::mosquiton::{PxAtlasDescriptor, decode_pxi, extract_atlas_rect, trim_transparent};

    let atlas: PxAtlasDescriptor =
        ron::from_str(SPIDER_SHOT_PX_ATLAS_RON).map_err(|err| err.to_string())?;
    let (atlas_width, _atlas_height, atlas_pixels) = decode_pxi(SPIDER_SHOT_PXI)?;

    let load_region_sprite = |name: &str| -> Result<CxImage, String> {
        let idx = atlas
            .names
            .get(name)
            .copied()
            .ok_or_else(|| format!("spider shot atlas missing {name} region"))?;
        let rect = atlas
            .regions
            .get(idx as usize)
            .and_then(|r| r.frames.first())
            .copied()
            .ok_or_else(|| format!("spider shot {name} region has no frame"))?;
        extract_atlas_rect(&atlas_pixels, atlas_width, rect)
            .and_then(trim_transparent)
            .ok_or_else(|| format!("spider shot {name} produced no visible pixels"))
    };

    let hover = Arc::new(load_region_sprite("hover")?);
    let hit = Arc::new(load_region_sprite("hit")?);

    // Destroy animation — single frame for spider_shot.
    let destroy_idx = atlas
        .names
        .get("destroy")
        .copied()
        .ok_or("spider shot atlas missing destroy region")?;
    let destroy_region = atlas
        .regions
        .get(destroy_idx as usize)
        .ok_or("spider shot destroy region out of range")?;
    let total_duration = atlas
        .animations
        .get("destroy")
        .map_or(0.3, |a| a.duration_ms as f32 / 1000.0);
    let frame_duration = (total_duration / destroy_region.frames.len().max(1) as f32).max(0.001);
    let destroy: Vec<MosquitonBillboardFrame> = destroy_region
        .frames
        .iter()
        .copied()
        .map(|rect| {
            Ok(MosquitonBillboardFrame {
                sprite: Arc::new(
                    extract_atlas_rect(&atlas_pixels, atlas_width, rect)
                        .and_then(trim_transparent)
                        .ok_or("spider shot destroy frame produced no visible pixels")?,
                ),
                duration: frame_duration,
            })
        })
        .collect::<Result<_, &str>>()
        .map_err(ToString::to_string)?;

    Ok(SpiderShotBillboardSprites {
        hover,
        hit,
        destroy,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::map::test_map;
    use carcinisation_fps_core::FpsCombatConfig;

    fn make_spidey(x: f32, y: f32) -> Spidey {
        Spidey::new(Vec2::new(x, y), SpideyConfig::default())
    }

    #[test]
    fn idle_transitions_on_aggro() {
        let map = test_map();
        let config = SpideyConfig::default();
        let mut ss = vec![Spidey::new(Vec2::new(2.0, 3.5), config)];
        let player = Vec2::new(3.0, 3.5);
        let _ = tick_spideys(&mut ss, player, &map, 0.016);
        // Should have left idle (either hop wait, leap windup, or web windup).
        assert!(
            !matches!(ss[0].state, SpideyState::Idle),
            "should leave idle when player is in aggro range"
        );
    }

    #[test]
    fn fire_damage_transitions_to_burning_corpse() {
        let mut spidey = make_spidey(2.0, 1.5);
        spidey.take_damage_from(100, DamageKind::Fire, 1.25);
        assert_eq!(spidey.health, 0);
        assert!(matches!(
            spidey.state,
            SpideyState::BurningCorpse { timer, .. } if (timer - 1.25).abs() < 0.001
        ));
        assert!(!spidey.is_alive());
    }

    #[test]
    fn dying_transitions_to_dead() {
        let map = test_map();
        let mut ss = vec![make_spidey(1.5, 1.5)];
        ss[0].state = SpideyState::Dying { timer: 0.1 };
        let _ = tick_spideys(&mut ss, Vec2::ZERO, &map, 0.2);
        assert!(matches!(ss[0].state, SpideyState::Dead));
    }

    #[test]
    fn hitscan_hits_spidey() {
        let map = test_map();
        let cam = Camera {
            position: Vec2::new(1.5, 1.5),
            angle: 0.0,
            ..Default::default()
        };
        let ss = vec![make_spidey(3.0, 1.5)];
        let hit = hitscan_spideys(&cam, &ss, &map);
        assert!(hit.is_some());
        assert_eq!(hit.unwrap().0, 0);
    }

    #[test]
    fn hitscan_misses_dead_spidey() {
        let map = test_map();
        let cam = Camera {
            position: Vec2::new(1.5, 1.5),
            angle: 0.0,
            ..Default::default()
        };
        let mut ss = vec![make_spidey(3.0, 1.5)];
        ss[0].state = SpideyState::Dead;
        let hit = hitscan_spideys(&cam, &ss, &map);
        assert!(hit.is_none());
    }

    #[test]
    fn web_projectile_has_webshot_kind() {
        let map = test_map();
        let mut config = SpideyConfig::default();
        config.sim.web_range = 10.0;
        config.sim.aggro_range = 12.0;
        config.sim.web_cue_secs = 0.1;
        let player = Vec2::new(5.5, 3.5);
        let mut spidey = Spidey::new(Vec2::new(1.5, 3.5), config.clone());
        spidey.web_anim_elapsed = Some(0.0);
        spidey.web_cooldown = 10.0; // Prevent re-trigger.
        spidey.state = SpideyState::WebWindup {
            timer: config.sim.web_cue_secs + config.sim.recover_secs,
        };

        // Tick past the cue.
        let (proj, _) = tick_single_spidey(&mut spidey, player, &map, 0.2);
        let proj = proj.expect("should spawn projectile past cue");
        assert!(
            matches!(
                proj.kind,
                carcinisation_fps_core::ProjectileKind::WebShot { .. }
            ),
            "spidey projectile should be WebShot, got {:?}",
            proj.kind
        );
    }

    #[test]
    fn seed_advances_across_ticks_preventing_lockstep() {
        let map = test_map();
        let player = Vec2::new(5.5, 3.5);
        // Two Spideys at different positions produce different initial seeds.
        let mut a = make_spidey(1.5, 3.5);
        let mut b = make_spidey(2.5, 3.5);

        let initial_seed_a = a.seed;
        let initial_seed_b = b.seed;
        assert_ne!(
            initial_seed_a, initial_seed_b,
            "different positions should give different seeds"
        );

        // Tick both through several hop cycles.
        for _ in 0..200 {
            let _ = tick_single_spidey(&mut a, player, &map, 0.016);
            let _ = tick_single_spidey(&mut b, player, &map, 0.016);
        }

        // Seeds should have advanced from their initial values.
        assert_ne!(
            a.seed, initial_seed_a,
            "seed A should advance after hop cycles"
        );
        assert_ne!(
            b.seed, initial_seed_b,
            "seed B should advance after hop cycles"
        );
        // And they should still differ from each other.
        assert_ne!(
            a.seed, b.seed,
            "two Spideys should not converge to same seed"
        );
    }

    #[test]
    fn default_config_matches_fps_core_combat_config() {
        let c = SpideyConfig::default();
        let combat = FpsCombatConfig::default();

        assert_eq!(
            c.sim.move_speed, combat.spidey.move_speed,
            "move_speed drift"
        );
        assert_eq!(
            c.sim.collision_radius, combat.spidey.collision_radius,
            "collision_radius drift"
        );
        assert_eq!(
            c.sim.lunge_melee_damage, combat.spidey.lunge_melee_damage,
            "lunge_melee_damage drift"
        );
        assert_eq!(
            c.sim.web_cooldown, combat.spidey.web_cooldown,
            "web_cooldown drift"
        );
        assert_eq!(
            c.sim.lunge_cooldown, combat.spidey.lunge_cooldown,
            "lunge_cooldown drift"
        );
        assert_eq!(
            c.sim.web_projectile_speed, combat.spidey.web_projectile_speed,
            "web_projectile_speed drift"
        );
        assert_eq!(
            c.sim.web_projectile_damage, combat.spidey.web_projectile_damage,
            "web_projectile_damage drift"
        );
        assert_eq!(c.health, combat.spidey.health, "health drift");
        assert_eq!(
            c.web_slow_multiplier, combat.spidey.web_slow_multiplier,
            "web_slow_multiplier drift"
        );
        assert_eq!(
            c.web_slow_duration, combat.spidey.web_slow_duration,
            "web_slow_duration drift"
        );
    }

    #[test]
    fn spidey_billboard_sprites_load_from_composed_atlas() {
        let sprites = make_spidey_billboard_sprites().unwrap();
        assert!(sprites.alive.len() >= 2, "idle should have multiple frames");
        assert!(!sprites.shoot.is_empty(), "shoot should have frames");
        assert!(!sprites.hop.is_empty(), "hop should have frames");
        assert!(!sprites.lunge.is_empty(), "lunge should have frames");
        assert!(sprites.death.width() > 1, "death should be non-trivial");
        // All alive frames should have same dimensions (stable bounding box).
        let w = sprites.alive[0].sprite.width();
        let h = sprites.alive[0].sprite.height();
        for frame in &sprites.alive {
            assert_eq!(frame.sprite.width(), w);
            assert_eq!(frame.sprite.height(), h);
        }
        // All alive frames should have visible pixels.
        assert!(sprites.alive.iter().all(|frame| {
            frame
                .sprite
                .data()
                .iter()
                .any(|&pixel| pixel != carapace::palette::TRANSPARENT_INDEX)
        }));
    }

    #[test]
    fn spider_shot_billboard_sprites_load_from_embedded_assets() {
        let sprites = make_spider_shot_billboard_sprites().unwrap();
        assert!(
            sprites.hover.width() > 1,
            "hover sprite should be non-trivial"
        );
        assert!(sprites.hover.height() > 1);
        assert!(sprites.hit.width() > 1, "hit sprite should be non-trivial");
        assert!(sprites.hit.height() > 1);
        assert!(
            sprites
                .hover
                .data()
                .iter()
                .any(|&px| px != carapace::palette::TRANSPARENT_INDEX),
            "hover sprite should have visible pixels"
        );
    }

    // -- Presentation adapter tests --

    #[test]
    fn presentation_idle_from_idle_and_hop_wait() {
        assert_eq!(
            spidey_presentation_state(&SpideyState::Idle, 1.0, 0.0),
            EnemyPresentationState::Idle,
        );
        assert_eq!(
            spidey_presentation_state(&SpideyState::HopWait { timer: 0.5 }, 1.0, 0.0),
            EnemyPresentationState::Idle,
        );
    }

    #[test]
    fn presentation_hopping_from_hop_move() {
        let state = SpideyState::HopMove {
            direction: Vec2::X,
            timer: 0.2,
            duration: 0.4,
            height_scale: 1.0,
        };
        let pres = spidey_presentation_state(&state, 0.5, 0.3);
        match pres {
            EnemyPresentationState::Hopping {
                phase,
                visual_height,
            } => {
                assert!((phase - 0.5).abs() < 0.01, "phase should be ~0.5");
                assert!((visual_height - 0.3).abs() < f32::EPSILON);
            }
            other => panic!("expected Hopping, got {other:?}"),
        }
    }

    #[test]
    fn presentation_windup_from_web_and_lunge() {
        let web = spidey_presentation_state(&SpideyState::WebWindup { timer: 1.0 }, 0.3, 0.0);
        assert!(matches!(
            web,
            EnemyPresentationState::Windup {
                attack: AttackPresentationKind::Ranged,
                ..
            }
        ));

        let lunge = spidey_presentation_state(&SpideyState::LungeWindup { timer: 0.2 }, 0.1, 0.0);
        assert!(matches!(
            lunge,
            EnemyPresentationState::Windup {
                attack: AttackPresentationKind::Melee,
                ..
            }
        ));
    }

    #[test]
    fn presentation_attacking_from_leap_attack() {
        let state = SpideyState::LungeAttack {
            target: Vec2::ZERO,
            timer: 0.3,
            dealt_damage: false,
        };
        let pres = spidey_presentation_state(&state, 0.2, 0.4);
        assert!(matches!(
            pres,
            EnemyPresentationState::Attacking {
                attack: AttackPresentationKind::Melee,
                ..
            }
        ));
    }

    #[test]
    fn presentation_dying_preserves_burn_flag() {
        let dying = spidey_presentation_state(&SpideyState::Dying { timer: 0.3 }, 0.1, 0.0);
        assert!(matches!(
            dying,
            EnemyPresentationState::Dying { burn: false, .. }
        ));

        let burning = spidey_presentation_state(
            &SpideyState::BurningCorpse {
                timer: 1.0,
                seed: 42,
            },
            0.2,
            0.0,
        );
        assert!(matches!(
            burning,
            EnemyPresentationState::Dying { burn: true, .. }
        ));
    }

    #[test]
    fn presentation_dead() {
        assert_eq!(
            spidey_presentation_state(&SpideyState::Dead, 0.0, 0.0),
            EnemyPresentationState::Dead { burn: false },
        );
    }

    // Round-trip test removed: SpideyState is now a type alias for SpideySimState,
    // so there is no conversion to test.
}
