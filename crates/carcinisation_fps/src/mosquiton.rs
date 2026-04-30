//! FP Mosquiton enemy — flying ranged attacker with melee fallback.

use asset_pipeline::composed_ron::{CompactComposedAtlas, CompactFrame, CompactPose};
use bevy_math::Vec2;
use carapace::{image::CxImage, palette::TRANSPARENT_INDEX};
use flate2::bufread::DeflateDecoder;
use serde::Deserialize;
use std::{
    collections::{HashMap, HashSet},
    io::{Cursor, Read as _},
    time::Duration,
};

use crate::camera::FpCamera;
use crate::enemy::FpProjectile;
use crate::map::FpMap;
use crate::raycast::cast_ray;

/// Configuration for FP Mosquiton behaviour.
#[derive(Clone, Debug)]
pub struct FpMosquitonConfig {
    pub move_speed: f32,
    pub preferred_range: f32,
    pub melee_range: f32,
    pub shoot_range: f32,
    pub shoot_cooldown: Duration,
    pub melee_cooldown: Duration,
    pub melee_attack_duration: Duration,
    pub melee_damage: u32,
    pub blood_shot_speed: f32,
    pub blood_shot_damage: u32,
    pub collision_radius: f32,
    pub billboard_height: f32,
    pub hover_height: f32,
    pub health: u32,
}

impl Default for FpMosquitonConfig {
    fn default() -> Self {
        Self {
            move_speed: 2.0,
            preferred_range: 4.0,
            melee_range: 1.0,
            shoot_range: 8.0,
            shoot_cooldown: Duration::from_secs_f32(1.5),
            melee_cooldown: Duration::from_secs_f32(0.8),
            melee_attack_duration: Duration::from_secs_f32(0.6),
            melee_damage: 15,
            blood_shot_speed: 5.0,
            blood_shot_damage: 10,
            collision_radius: 0.3,
            billboard_height: 0.9,
            hover_height: 0.08,
            health: 40,
        }
    }
}

/// AI state for an FP Mosquiton.
#[derive(Clone, Debug)]
pub enum FpMosquitonState {
    /// Moving toward the player.
    Pursue,
    /// At preferred range, strafing and shooting.
    RangedAttack { strafe_dir: f32 },
    /// Close enough for melee.
    MeleeAttack { timer: f32, dealt_damage: bool },
    /// Brief pause after melee before re-engaging.
    Recover { timer: f32 },
    /// Playing death animation.
    Dying { timer: f32 },
    /// Fully dead.
    Dead,
}

/// Results produced by ticking FP Mosquiton AI.
#[derive(Clone, Debug, Default)]
pub struct FpMosquitonTickResult {
    pub projectiles: Vec<FpProjectile>,
    pub player_damage: u32,
    pub damage_source: Option<Vec2>,
}

/// A runtime FP Mosquiton instance.
#[derive(Clone, Debug)]
pub struct FpMosquiton {
    pub position: Vec2,
    pub height: f32,
    pub velocity: Vec2,
    pub animation_time: f32,
    pub health: u32,
    pub max_health: u32,
    pub state: FpMosquitonState,
    pub shoot_cooldown: f32,
    pub melee_cooldown: f32,
    pub decision_timer: f32,
    pub config: FpMosquitonConfig,
}

impl FpMosquiton {
    #[must_use]
    pub fn new(position: Vec2, config: FpMosquitonConfig) -> Self {
        let health = config.health;
        Self {
            position,
            height: config.hover_height,
            velocity: Vec2::ZERO,
            animation_time: 0.0,
            health,
            max_health: health,
            state: FpMosquitonState::Pursue,
            shoot_cooldown: 0.0,
            melee_cooldown: 0.0,
            decision_timer: 0.0,
            config,
        }
    }

    #[must_use]
    pub fn is_alive(&self) -> bool {
        !matches!(
            self.state,
            FpMosquitonState::Dying { .. } | FpMosquitonState::Dead
        )
    }

    pub fn take_damage(&mut self, amount: u32) {
        self.health = self.health.saturating_sub(amount);
        if self.health == 0 && self.is_alive() {
            self.state = FpMosquitonState::Dying { timer: 0.5 };
        }
    }
}

/// Check line of sight from a position to a target using raycasting.
#[must_use]
pub fn has_line_of_sight(from: Vec2, to: Vec2, map: &FpMap) -> bool {
    let dir = to - from;
    let dist = dir.length();
    if dist < 0.01 {
        return true;
    }
    let hit = cast_ray(map, from, dir / dist);
    hit.distance > dist
}

/// Tick all Mosquitons. Returns spawned projectiles and direct melee damage.
#[must_use]
pub fn tick_mosquitons(
    mosquitons: &mut [FpMosquiton],
    player_pos: Vec2,
    map: &FpMap,
    dt: f32,
) -> FpMosquitonTickResult {
    let mut result = FpMosquitonTickResult::default();

    for m in mosquitons.iter_mut() {
        // Tick cooldowns.
        m.shoot_cooldown = (m.shoot_cooldown - dt).max(0.0);
        m.melee_cooldown = (m.melee_cooldown - dt).max(0.0);
        m.decision_timer = (m.decision_timer - dt).max(0.0);
        if !matches!(m.state, FpMosquitonState::Dead) {
            m.animation_time += dt;
        }
        m.velocity = Vec2::ZERO;

        match &mut m.state {
            FpMosquitonState::Dead => continue,

            FpMosquitonState::Dying { timer } => {
                *timer -= dt;
                if *timer <= 0.0 {
                    m.state = FpMosquitonState::Dead;
                }
                continue;
            }

            FpMosquitonState::Recover { timer } => {
                *timer -= dt;
                if *timer <= 0.0 {
                    m.state = FpMosquitonState::Pursue;
                }
                continue;
            }

            FpMosquitonState::Pursue => {
                let to_player = player_pos - m.position;
                let dist = to_player.length();

                if dist <= m.config.melee_range {
                    if m.melee_cooldown <= 0.0 {
                        start_melee_attack(m);
                    } else {
                        back_off_from_player(m, to_player, dist, map, dt);
                    }
                    continue;
                }

                if dist <= m.config.preferred_range {
                    // Arrived at preferred range — switch to ranged.
                    let strafe_dir = if (m.position.x * 100.0) as i32 % 2 == 0 {
                        1.0
                    } else {
                        -1.0
                    };
                    m.state = FpMosquitonState::RangedAttack { strafe_dir };
                    continue;
                }

                // Move toward player.
                if dist > 0.01 {
                    let move_dir = to_player / dist;
                    let step = move_dir * m.config.move_speed * dt;
                    m.velocity = step / dt.max(f32::EPSILON);
                    crate::collision::try_move(
                        &mut m.position,
                        step,
                        m.config.collision_radius,
                        map,
                    );
                }

                // Shoot while approaching if LOS is clear.
                if m.shoot_cooldown <= 0.0
                    && dist < m.config.shoot_range
                    && has_line_of_sight(m.position, player_pos, map)
                {
                    if let Some(proj) =
                        FpProjectile::new(m.position, player_pos, m.config.blood_shot_damage)
                    {
                        let mut p = proj;
                        p.speed = m.config.blood_shot_speed;
                        p.radius = 0.2;
                        result.projectiles.push(p);
                    }
                    m.shoot_cooldown = m.config.shoot_cooldown.as_secs_f32();
                }
            }

            FpMosquitonState::RangedAttack { strafe_dir } => {
                let to_player = player_pos - m.position;
                let dist = to_player.length();

                if dist <= m.config.melee_range {
                    if m.melee_cooldown <= 0.0 {
                        start_melee_attack(m);
                    } else {
                        back_off_from_player(m, to_player, dist, map, dt);
                    }
                    continue;
                }

                if dist > m.config.preferred_range * 1.5 {
                    m.state = FpMosquitonState::Pursue;
                    continue;
                }

                if m.decision_timer <= 0.0 {
                    *strafe_dir *= -1.0;
                    m.decision_timer = 0.75;
                }

                // Strafe perpendicular to player direction.
                if dist > 0.01 {
                    let dir_to_player = to_player / dist;
                    let strafe = Vec2::new(-dir_to_player.y, dir_to_player.x) * *strafe_dir;
                    let step = strafe * m.config.move_speed * 0.5 * dt;
                    m.velocity = step / dt.max(f32::EPSILON);
                    crate::collision::try_move(
                        &mut m.position,
                        step,
                        m.config.collision_radius,
                        map,
                    );
                }

                // Shoot on cooldown.
                if m.shoot_cooldown <= 0.0 && has_line_of_sight(m.position, player_pos, map) {
                    if let Some(proj) =
                        FpProjectile::new(m.position, player_pos, m.config.blood_shot_damage)
                    {
                        let mut p = proj;
                        p.speed = m.config.blood_shot_speed;
                        p.radius = 0.2;
                        result.projectiles.push(p);
                    }
                    m.shoot_cooldown = m.config.shoot_cooldown.as_secs_f32();
                }
            }

            FpMosquitonState::MeleeAttack {
                timer,
                dealt_damage,
            } => {
                let dist = m.position.distance(player_pos);

                if dist > m.config.melee_range * 1.5 {
                    m.state = FpMosquitonState::Pursue;
                    continue;
                }

                if !*dealt_damage {
                    result.player_damage += m.config.melee_damage;
                    result.damage_source = Some(m.position);
                    *dealt_damage = true;
                    m.melee_cooldown = m.config.melee_cooldown.as_secs_f32();
                }

                *timer -= dt;
                if *timer <= 0.0 {
                    m.state = FpMosquitonState::Recover { timer: 0.2 };
                }
            }
        }
    }

    result
}

fn start_melee_attack(mosquiton: &mut FpMosquiton) {
    mosquiton.animation_time = 0.0;
    mosquiton.state = FpMosquitonState::MeleeAttack {
        timer: mosquiton.config.melee_attack_duration.as_secs_f32(),
        dealt_damage: false,
    };
}

fn back_off_from_player(
    mosquiton: &mut FpMosquiton,
    to_player: Vec2,
    dist: f32,
    map: &FpMap,
    dt: f32,
) {
    if dist <= 0.01 {
        return;
    }

    let step = -(to_player / dist) * mosquiton.config.move_speed * 0.35 * dt;
    mosquiton.velocity = step / dt.max(f32::EPSILON);
    crate::collision::try_move(
        &mut mosquiton.position,
        step,
        mosquiton.config.collision_radius,
        map,
    );
}

/// Hitscan check against Mosquitons. Returns index of closest hit.
#[must_use]
pub fn hitscan_mosquitons(
    camera: &FpCamera,
    mosquitons: &[FpMosquiton],
    map: &FpMap,
) -> Option<(usize, f32)> {
    let dir = camera.direction();
    let origin = camera.position;
    let wall_hit = cast_ray(map, origin, dir);
    let max_dist = wall_hit.distance;

    let mut closest: Option<(usize, f32)> = None;

    for (i, m) in mosquitons.iter().enumerate() {
        if !m.is_alive() {
            continue;
        }

        let to_enemy = m.position - origin;
        let along_ray = to_enemy.dot(dir);

        if along_ray <= 0.0 || along_ray > max_dist {
            continue;
        }

        let perp_dist_sq = to_enemy.length_squared() - along_ray * along_ray;
        let radius_sq = m.config.collision_radius * m.config.collision_radius;

        if perp_dist_sq < radius_sq && closest.is_none_or(|(_, d)| along_ray < d) {
            closest = Some((i, along_ray));
        }
    }

    closest
}

const MOSQUITON_COMPOSED_RON: &str =
    include_str!("../../../assets/sprites/enemies/mosquiton_3/atlas.composed.ron");
const MOSQUITON_PX_ATLAS_RON: &str =
    include_str!("../../../assets/sprites/enemies/mosquiton_3/atlas.px_atlas.ron");
const MOSQUITON_PXI: &[u8] =
    include_bytes!("../../../assets/sprites/enemies/mosquiton_3/atlas.pxi");
const MOSQUITON_IDLE_FLY_TAG: &str = "idle_fly";
const MOSQUITON_DEATH_FLY_TAG: &str = "death_fly";
const MOSQUITON_MELEE_FLY_TAG: &str = "melee_fly";
const MOSQUITON_WING_TAG: &str = "wings";
const BLOOD_SHOT_PX_ATLAS_RON: &str =
    include_str!("../../../assets/sprites/attacks/blood_shot/atlas.px_atlas.ron");
const BLOOD_SHOT_PXI: &[u8] =
    include_bytes!("../../../assets/sprites/attacks/blood_shot/atlas.pxi");
const BLOOD_SHOT_HOVER_REGION: &str = "hover";
const BLOOD_SHOT_HIT_REGION: &str = "hit";
const BLOOD_SHOT_DESTROY_REGION: &str = "destroy";

/// One rendered FP Mosquiton billboard frame.
#[derive(Clone, Debug)]
pub struct FpMosquitonBillboardFrame {
    pub sprite: CxImage,
    pub duration: f32,
}

/// Palette-indexed FP Mosquiton billboard sprites resolved from composed output.
#[derive(Clone, Debug)]
pub struct FpMosquitonBillboardSprites {
    pub alive: Vec<FpMosquitonBillboardFrame>,
    pub melee: Vec<FpMosquitonBillboardFrame>,
    pub death: CxImage,
}

impl FpMosquitonBillboardSprites {
    #[must_use]
    pub fn alive_sprite_at(&self, elapsed_secs: f32) -> &CxImage {
        animation_sprite_at(&self.alive, elapsed_secs)
    }

    #[must_use]
    pub fn melee_sprite_at(&self, elapsed_secs: f32) -> &CxImage {
        animation_sprite_at_clamped(&self.melee, elapsed_secs)
    }
}

/// Palette-indexed FP blood-shot sprites resolved from the ORS attack atlas.
#[derive(Clone, Debug)]
pub struct FpBloodShotBillboardSprites {
    pub hover: CxImage,
    pub hit: CxImage,
    pub destroy: Vec<FpMosquitonBillboardFrame>,
}

impl FpBloodShotBillboardSprites {
    #[must_use]
    pub fn destroy_sprite_at(&self, elapsed_secs: f32) -> &CxImage {
        animation_sprite_at_clamped(&self.destroy, elapsed_secs)
    }
}

/// Sample a looping animation at the given elapsed time.
fn animation_sprite_at(frames: &[FpMosquitonBillboardFrame], elapsed_secs: f32) -> &CxImage {
    debug_assert!(
        !frames.is_empty(),
        "animation_sprite_at requires non-empty frames"
    );
    let total_duration = frames
        .iter()
        .map(|frame| frame.duration.max(0.0))
        .sum::<f32>();
    if total_duration <= f32::EPSILON {
        return &frames[0].sprite;
    }

    let mut t = elapsed_secs.rem_euclid(total_duration);
    for frame in frames {
        let duration = frame.duration.max(0.0);
        if t < duration {
            return &frame.sprite;
        }
        t -= duration;
    }
    &frames[0].sprite
}

/// Sample a one-shot animation, clamping to the last frame when finished.
fn animation_sprite_at_clamped(
    frames: &[FpMosquitonBillboardFrame],
    elapsed_secs: f32,
) -> &CxImage {
    debug_assert!(
        !frames.is_empty(),
        "animation_sprite_at_clamped requires non-empty frames"
    );
    let mut t = elapsed_secs;
    for frame in frames {
        let duration = frame.duration.max(0.0);
        if t < duration {
            return &frame.sprite;
        }
        t -= duration;
    }
    &frames[frames.len() - 1].sprite
}

#[derive(Deserialize)]
struct PxAtlasDescriptor {
    regions: Vec<PxAtlasRegion>,
    #[serde(default)]
    names: HashMap<String, u32>,
    #[serde(default)]
    animations: HashMap<String, PxAtlasAnimation>,
}

#[derive(Deserialize)]
struct PxAtlasRegion {
    frames: Vec<PxAtlasRect>,
}

#[derive(Clone, Copy, Deserialize)]
struct PxAtlasRect {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
}

#[derive(Deserialize)]
struct PxAtlasAnimation {
    duration_ms: u64,
}

#[derive(Clone)]
struct ComposePart {
    parent: Option<u8>,
    pivot: (i16, i16),
    visual: bool,
    draw_order: u8,
}

#[derive(Clone)]
struct Placement {
    sprite_idx: usize,
    top_left: (i32, i32),
    size: (u32, u32),
    flip_x: bool,
    flip_y: bool,
}

/// Resolve the embedded Mosquiton composed output into FP billboard sprites.
///
/// This uses the same compact composed atlas and PXI atlas generated for the
/// ORS Mosquiton. FP loops the composed `idle_fly` frames so the authored wing
/// flap reads in the raycaster without baking new source assets.
///
/// # Errors
///
/// Returns an error if embedded generated assets are malformed.
pub fn make_mosquiton_billboard_sprites() -> Result<FpMosquitonBillboardSprites, String> {
    let composed: CompactComposedAtlas =
        ron::from_str(MOSQUITON_COMPOSED_RON).map_err(|err| err.to_string())?;
    let atlas: PxAtlasDescriptor =
        ron::from_str(MOSQUITON_PX_ATLAS_RON).map_err(|err| err.to_string())?;
    let (atlas_width, _atlas_height, atlas_pixels) = decode_pxi(MOSQUITON_PXI)?;

    let alive = compose_animation_frames_wing_only(
        &composed,
        &atlas,
        &atlas_pixels,
        atlas_width,
        MOSQUITON_IDLE_FLY_TAG,
    )?;
    let death = compose_first_animation_frame(
        &composed,
        &atlas,
        &atlas_pixels,
        atlas_width,
        MOSQUITON_DEATH_FLY_TAG,
    )?;
    let melee = compose_animation_frames_full(
        &composed,
        &atlas,
        &atlas_pixels,
        atlas_width,
        MOSQUITON_MELEE_FLY_TAG,
    )?;
    Ok(FpMosquitonBillboardSprites {
        alive,
        melee,
        death,
    })
}

/// Resolve all FP blood-shot billboard sprites from existing ORS attack assets.
///
/// # Errors
///
/// Returns an error if embedded generated assets are malformed.
pub fn make_blood_shot_billboard_sprites() -> Result<FpBloodShotBillboardSprites, String> {
    Ok(FpBloodShotBillboardSprites {
        hover: make_blood_shot_region_first_sprite(BLOOD_SHOT_HOVER_REGION)?,
        hit: make_blood_shot_region_first_sprite(BLOOD_SHOT_HIT_REGION)?,
        destroy: make_blood_shot_region_animation(BLOOD_SHOT_DESTROY_REGION)?,
    })
}

fn make_blood_shot_region_first_sprite(region_name: &str) -> Result<CxImage, String> {
    let atlas: PxAtlasDescriptor =
        ron::from_str(BLOOD_SHOT_PX_ATLAS_RON).map_err(|err| err.to_string())?;
    let (atlas_width, _atlas_height, atlas_pixels) = decode_pxi(BLOOD_SHOT_PXI)?;
    let region_idx = atlas
        .names
        .get(region_name)
        .copied()
        .ok_or_else(|| format!("blood shot atlas is missing {region_name} region"))?;
    let rect = atlas
        .regions
        .get(region_idx as usize)
        .and_then(|region| region.frames.first())
        .copied()
        .ok_or_else(|| format!("blood shot {region_name} region has no frame"))?;
    extract_atlas_rect(&atlas_pixels, atlas_width, rect)
        .and_then(trim_transparent)
        .ok_or_else(|| format!("blood shot {region_name} frame produced no visible pixels"))
}

fn make_blood_shot_region_animation(
    region_name: &str,
) -> Result<Vec<FpMosquitonBillboardFrame>, String> {
    let atlas: PxAtlasDescriptor =
        ron::from_str(BLOOD_SHOT_PX_ATLAS_RON).map_err(|err| err.to_string())?;
    let (atlas_width, _atlas_height, atlas_pixels) = decode_pxi(BLOOD_SHOT_PXI)?;
    let region_idx = atlas
        .names
        .get(region_name)
        .copied()
        .ok_or_else(|| format!("blood shot atlas is missing {region_name} region"))?;
    let region = atlas
        .regions
        .get(region_idx as usize)
        .ok_or_else(|| format!("blood shot {region_name} region is out of range"))?;
    if region.frames.is_empty() {
        return Err(format!("blood shot {region_name} region has no frames"));
    }

    let total_duration = atlas
        .animations
        .get(region_name)
        .map_or(0.3, |animation| animation.duration_ms as f32 / 1000.0);
    let frame_duration = (total_duration / region.frames.len() as f32).max(0.001);

    region
        .frames
        .iter()
        .copied()
        .map(|rect| {
            Ok(FpMosquitonBillboardFrame {
                sprite: extract_atlas_rect(&atlas_pixels, atlas_width, rect)
                    .and_then(trim_transparent)
                    .ok_or_else(|| {
                        format!("blood shot {region_name} frame produced no visible pixels")
                    })?,
                duration: frame_duration,
            })
        })
        .collect()
}

fn compose_animation_frames_wing_only(
    composed: &CompactComposedAtlas,
    atlas: &PxAtlasDescriptor,
    atlas_pixels: &[u8],
    atlas_width: u32,
    tag: &str,
) -> Result<Vec<FpMosquitonBillboardFrame>, String> {
    let animation = composed
        .animations
        .iter()
        .find(|animation| animation.tag == tag)
        .ok_or_else(|| format!("missing Mosquiton composed animation '{tag}'"))?;
    if animation.frames.is_empty() {
        return Err(format!(
            "Mosquiton composed animation '{tag}' has no frames"
        ));
    }
    let base_frame = animation.frames.first().expect("empty checked above");
    let wing_part_ids = composed
        .parts
        .iter()
        .filter(|part| part.tags.iter().any(|tag| tag == MOSQUITON_WING_TAG))
        .map(|part| part.id)
        .collect::<HashSet<_>>();

    render_stable_animation_frames(
        composed,
        atlas,
        atlas_pixels,
        atlas_width,
        tag,
        &animation.frames,
        |frame| merge_wing_frame_poses(base_frame, frame, &wing_part_ids),
    )
}

fn compose_animation_frames_full(
    composed: &CompactComposedAtlas,
    atlas: &PxAtlasDescriptor,
    atlas_pixels: &[u8],
    atlas_width: u32,
    tag: &str,
) -> Result<Vec<FpMosquitonBillboardFrame>, String> {
    let animation = composed
        .animations
        .iter()
        .find(|animation| animation.tag == tag)
        .ok_or_else(|| format!("missing Mosquiton composed animation '{tag}'"))?;
    if animation.frames.is_empty() {
        return Err(format!(
            "Mosquiton composed animation '{tag}' has no frames"
        ));
    }

    render_stable_animation_frames(
        composed,
        atlas,
        atlas_pixels,
        atlas_width,
        tag,
        &animation.frames,
        |frame| frame.poses.clone(),
    )
}

fn render_stable_animation_frames(
    composed: &CompactComposedAtlas,
    atlas: &PxAtlasDescriptor,
    atlas_pixels: &[u8],
    atlas_width: u32,
    tag: &str,
    frames: &[CompactFrame],
    mut poses_for_frame: impl FnMut(&CompactFrame) -> Vec<CompactPose>,
) -> Result<Vec<FpMosquitonBillboardFrame>, String> {
    // Compute placements for every frame so a single stable bounding box can
    // be used for the whole animation. Per-frame tight bounds make billboards
    // twitch when only a sub-part, such as a wing, changes size or position.
    let mut per_frame = Vec::with_capacity(frames.len());
    for frame in frames {
        let frame_poses = poses_for_frame(frame);
        let placements = compute_frame_placements(composed, &frame_poses)?;
        per_frame.push((placements, f32::from(frame.duration_ms) / 1000.0));
    }

    let union_bounds = union_placement_bounds(per_frame.iter().map(|(p, _)| p.as_slice()))
        .ok_or_else(|| format!("Mosquiton animation '{tag}' produced no placements"))?;
    let rendered: Vec<(CxImage, f32)> = per_frame
        .into_iter()
        .map(|(placements, duration)| {
            let image = render_placements_in_bounds(
                &placements,
                atlas,
                atlas_pixels,
                atlas_width,
                &union_bounds,
            )
            .ok_or_else(|| format!("Mosquiton animation '{tag}' produced no visible pixels"))?;
            Ok((image, duration))
        })
        .collect::<Result<_, String>>()?;
    let union_visible = rendered
        .iter()
        .filter_map(|(img, _)| visible_bounds(img))
        .reduce(|(a0, b0, c0, d0), (a1, b1, c1, d1)| {
            (a0.min(a1), b0.min(b1), c0.max(c1), d0.max(d1))
        })
        .ok_or_else(|| format!("Mosquiton animation '{tag}' produced no visible pixels"))?;

    Ok(rendered
        .iter()
        .map(|(img, duration)| FpMosquitonBillboardFrame {
            sprite: crop_to_rect(
                img,
                union_visible.0,
                union_visible.1,
                union_visible.2,
                union_visible.3,
            ),
            duration: *duration,
        })
        .collect())
}

fn compose_first_animation_frame(
    composed: &CompactComposedAtlas,
    atlas: &PxAtlasDescriptor,
    atlas_pixels: &[u8],
    atlas_width: u32,
    tag: &str,
) -> Result<CxImage, String> {
    let frame = composed
        .animations
        .iter()
        .find(|animation| animation.tag == tag)
        .and_then(|animation| animation.frames.first())
        .ok_or_else(|| format!("missing Mosquiton composed animation frame '{tag}'"))?;
    compose_frame_image(
        composed,
        atlas,
        atlas_pixels,
        atlas_width,
        tag,
        &frame.poses,
    )
}

fn compute_frame_placements(
    composed: &CompactComposedAtlas,
    frame_poses: &[CompactPose],
) -> Result<Vec<Placement>, String> {
    let parts = composed
        .parts
        .iter()
        .map(|part| {
            (
                part.id,
                ComposePart {
                    parent: part.parent,
                    pivot: part.pivot,
                    visual: part.visual,
                    draw_order: part.draw_order,
                },
            )
        })
        .collect::<HashMap<_, _>>();

    let mut poses = HashMap::<u8, Vec<&CompactPose>>::new();
    for pose in frame_poses {
        poses.entry(pose.p).or_default().push(pose);
    }
    for fragments in poses.values_mut() {
        fragments.sort_by_key(|pose| pose.frag);
    }

    let mut visual_part_ids = composed
        .parts
        .iter()
        .filter(|part| part.visual)
        .map(|part| part.id)
        .collect::<Vec<_>>();
    visual_part_ids.sort_by_key(|id| parts.get(id).map_or(u8::MAX, |part| part.draw_order));

    let mut resolved_pivots = HashMap::<u8, (i32, i32)>::new();
    let mut placements = Vec::new();
    for part_id in visual_part_ids {
        let Some(part) = parts.get(&part_id) else {
            continue;
        };
        if !part.visual {
            continue;
        }
        let Some(fragments) = poses.get(&part_id) else {
            continue;
        };
        let Some(primary) = fragments.first() else {
            continue;
        };
        let absolute_pivot = resolve_compose_pivot(part_id, &parts, &poses, &mut resolved_pivots)?;

        for pose in fragments {
            let frag_pivot = if std::ptr::eq(*pose, *primary) {
                absolute_pivot
            } else if part.parent.is_some() {
                let parent =
                    resolve_parent_compose_pivot(part_id, &parts, &poses, &mut resolved_pivots)?;
                (
                    parent.0 + i32::from(pose.o.0),
                    parent.1 + i32::from(pose.o.1),
                )
            } else {
                (i32::from(pose.o.0), i32::from(pose.o.1))
            };
            let size = composed
                .sprite_sizes
                .get(pose.s as usize)
                .ok_or_else(|| format!("sprite index {} out of range", pose.s))?;
            placements.push(Placement {
                sprite_idx: pose.s as usize,
                top_left: (
                    frag_pivot.0 - i32::from(part.pivot.0),
                    frag_pivot.1 - i32::from(part.pivot.1),
                ),
                size: (u32::from(size.0), u32::from(size.1)),
                flip_x: pose.fx,
                flip_y: pose.fy,
            });
        }
    }

    Ok(placements)
}

fn compose_frame_image(
    composed: &CompactComposedAtlas,
    atlas: &PxAtlasDescriptor,
    atlas_pixels: &[u8],
    atlas_width: u32,
    tag: &str,
    frame_poses: &[CompactPose],
) -> Result<CxImage, String> {
    let placements = compute_frame_placements(composed, frame_poses)?;
    render_placements(&placements, atlas, atlas_pixels, atlas_width)
        .ok_or_else(|| format!("Mosquiton animation '{tag}' produced no visible pixels"))
}

fn merge_wing_frame_poses(
    base_frame: &CompactFrame,
    wing_frame: &CompactFrame,
    wing_part_ids: &HashSet<u8>,
) -> Vec<CompactPose> {
    base_frame
        .poses
        .iter()
        .filter(|pose| !wing_part_ids.contains(&pose.p))
        .chain(
            wing_frame
                .poses
                .iter()
                .filter(|pose| wing_part_ids.contains(&pose.p)),
        )
        .cloned()
        .collect()
}

fn resolve_compose_pivot(
    part_id: u8,
    parts: &HashMap<u8, ComposePart>,
    poses: &HashMap<u8, Vec<&CompactPose>>,
    resolved: &mut HashMap<u8, (i32, i32)>,
) -> Result<(i32, i32), String> {
    if let Some(pivot) = resolved.get(&part_id) {
        return Ok(*pivot);
    }
    let part = parts
        .get(&part_id)
        .ok_or_else(|| format!("part index {part_id} out of range"))?;
    let primary = poses
        .get(&part_id)
        .and_then(|fragments| fragments.first())
        .ok_or_else(|| format!("part {part_id} missing primary pose"))?;
    let pivot = if part.parent.is_some() {
        let parent = resolve_parent_compose_pivot(part_id, parts, poses, resolved)?;
        (
            parent.0 + i32::from(primary.o.0),
            parent.1 + i32::from(primary.o.1),
        )
    } else {
        (i32::from(primary.o.0), i32::from(primary.o.1))
    };
    resolved.insert(part_id, pivot);
    Ok(pivot)
}

fn resolve_parent_compose_pivot(
    part_id: u8,
    parts: &HashMap<u8, ComposePart>,
    poses: &HashMap<u8, Vec<&CompactPose>>,
    resolved: &mut HashMap<u8, (i32, i32)>,
) -> Result<(i32, i32), String> {
    let mut parent_id = parts.get(&part_id).and_then(|part| part.parent);
    while let Some(current_parent_id) = parent_id {
        let parent = parts
            .get(&current_parent_id)
            .ok_or_else(|| format!("parent part index {current_parent_id} out of range"))?;
        if parent.visual {
            if poses.contains_key(&current_parent_id) {
                return resolve_compose_pivot(current_parent_id, parts, poses, resolved);
            }
            return Err(format!("visual parent {current_parent_id} has no pose"));
        }
        parent_id = parent.parent;
    }
    Ok((0, 0))
}

fn render_placements(
    placements: &[Placement],
    atlas: &PxAtlasDescriptor,
    atlas_pixels: &[u8],
    atlas_width: u32,
) -> Option<CxImage> {
    let bounds = PlacementBounds::from_placements(placements)?;
    let image = render_placements_in_bounds(placements, atlas, atlas_pixels, atlas_width, &bounds)?;
    trim_transparent(image)
}

/// Axis-aligned bounding box for a set of placements.
#[derive(Clone, Copy)]
struct PlacementBounds {
    min_x: i32,
    min_y: i32,
    max_x: i32,
    max_y: i32,
}

impl PlacementBounds {
    fn from_placements(placements: &[Placement]) -> Option<Self> {
        let min_x = placements.iter().map(|p| p.top_left.0).min()?;
        let min_y = placements.iter().map(|p| p.top_left.1).min()?;
        let max_x = placements
            .iter()
            .map(|p| p.top_left.0 + p.size.0 as i32)
            .max()?;
        let max_y = placements
            .iter()
            .map(|p| p.top_left.1 + p.size.1 as i32)
            .max()?;
        Some(Self {
            min_x,
            min_y,
            max_x,
            max_y,
        })
    }

    fn union(self, other: Self) -> Self {
        Self {
            min_x: self.min_x.min(other.min_x),
            min_y: self.min_y.min(other.min_y),
            max_x: self.max_x.max(other.max_x),
            max_y: self.max_y.max(other.max_y),
        }
    }
}

/// Compute the union bounding box across multiple frames' placements.
fn union_placement_bounds<'a>(
    frames: impl Iterator<Item = &'a [Placement]>,
) -> Option<PlacementBounds> {
    frames
        .filter_map(PlacementBounds::from_placements)
        .reduce(PlacementBounds::union)
}

/// Render placements into a fixed bounding box without per-frame trimming.
/// The caller is responsible for applying a consistent trim across all frames
/// via [`crop_to_rect`].
fn render_placements_in_bounds(
    placements: &[Placement],
    atlas: &PxAtlasDescriptor,
    atlas_pixels: &[u8],
    atlas_width: u32,
    bounds: &PlacementBounds,
) -> Option<CxImage> {
    let width = (bounds.max_x - bounds.min_x).max(1) as u32;
    let height = (bounds.max_y - bounds.min_y).max(1) as u32;
    let mut data = vec![TRANSPARENT_INDEX; (width * height) as usize];

    for placement in placements {
        let rect = atlas
            .regions
            .get(placement.sprite_idx)
            .and_then(|region| region.frames.first())
            .copied()?;
        for local_y in 0..rect.h {
            for local_x in 0..rect.w {
                let src_x = if placement.flip_x {
                    rect.x + rect.w - 1 - local_x
                } else {
                    rect.x + local_x
                };
                let src_y = if placement.flip_y {
                    rect.y + rect.h - 1 - local_y
                } else {
                    rect.y + local_y
                };
                let src_idx = (src_y * atlas_width + src_x) as usize;
                let pixel = *atlas_pixels.get(src_idx)?;
                if pixel == TRANSPARENT_INDEX {
                    continue;
                }
                let dst_x = (placement.top_left.0 - bounds.min_x) as u32 + local_x;
                let dst_y = (placement.top_left.1 - bounds.min_y) as u32 + local_y;
                data[(dst_y * width + dst_x) as usize] = pixel;
            }
        }
    }

    Some(CxImage::new(data, width as usize))
}

/// Find the visible (non-transparent) bounding rect within an image.
fn visible_bounds(image: &CxImage) -> Option<(u32, u32, u32, u32)> {
    let w = image.width() as i32;
    let h = image.height() as i32;
    let pixels = image.data();
    let mut min_x = w;
    let mut min_y = h;
    let mut max_x: i32 = -1;
    let mut max_y: i32 = -1;
    for y in 0..h {
        for x in 0..w {
            if pixels[(y * w + x) as usize] != TRANSPARENT_INDEX {
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
    }
    if max_x < min_x {
        return None;
    }
    Some((min_x as u32, min_y as u32, max_x as u32, max_y as u32))
}

/// Crop an image to the given pixel rect (inclusive).
fn crop_to_rect(image: &CxImage, min_x: u32, min_y: u32, max_x: u32, max_y: u32) -> CxImage {
    debug_assert!(min_x <= max_x && min_y <= max_y, "crop_to_rect: empty rect");
    debug_assert!(
        (max_x as usize) < image.width() && (max_y as usize) < image.height(),
        "crop_to_rect: rect exceeds image bounds"
    );
    let src_w = image.width();
    let out_w = (max_x - min_x + 1) as usize;
    let out_h = (max_y - min_y + 1) as usize;
    let pixels = image.data();
    let mut out = Vec::with_capacity(out_w * out_h);
    for y in min_y..=max_y {
        let start = y as usize * src_w + min_x as usize;
        out.extend_from_slice(&pixels[start..start + out_w]);
    }
    CxImage::new(out, out_w)
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

fn trim_transparent(image: CxImage) -> Option<CxImage> {
    let (min_x, min_y, max_x, max_y) = visible_bounds(&image)?;
    Some(crop_to_rect(&image, min_x, min_y, max_x, max_y))
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
    use crate::map::{FpMap, test_map};

    fn make_mosquiton(x: f32, y: f32) -> FpMosquiton {
        FpMosquiton::new(Vec2::new(x, y), FpMosquitonConfig::default())
    }

    fn pose_key(pose: &CompactPose) -> (u8, u8, (i8, i8), bool, bool, u8) {
        (pose.p, pose.s, pose.o, pose.fx, pose.fy, pose.frag)
    }

    #[test]
    fn pursue_moves_toward_player() {
        let map = test_map();
        let config = FpMosquitonConfig {
            preferred_range: 2.0,
            ..Default::default()
        };
        let mut ms = vec![FpMosquiton::new(Vec2::new(1.5, 1.5), config)];
        let player = Vec2::new(5.5, 1.5);
        let _ = tick_mosquitons(&mut ms, player, &map, 0.1);
        // Should have moved toward player (x increased).
        assert!(ms[0].position.x > 1.5);
        assert!(ms[0].animation_time > 0.0);
    }

    #[test]
    fn switches_to_ranged_at_preferred_range() {
        let map = test_map();
        let config = FpMosquitonConfig {
            preferred_range: 2.0,
            ..Default::default()
        };
        let mut ms = vec![FpMosquiton::new(Vec2::new(3.5, 1.5), config)];
        let player = Vec2::new(1.5, 1.5);
        // Distance = 2.0, at preferred range.
        let _ = tick_mosquitons(&mut ms, player, &map, 0.016);
        assert!(matches!(ms[0].state, FpMosquitonState::RangedAttack { .. }));
    }

    #[test]
    fn switches_to_melee_when_close() {
        let map = test_map();
        let config = FpMosquitonConfig {
            melee_range: 1.0,
            ..Default::default()
        };
        let mut ms = vec![FpMosquiton::new(Vec2::new(2.0, 1.5), config)];
        let player = Vec2::new(1.5, 1.5);
        // Distance = 0.5, within melee range.
        let _ = tick_mosquitons(&mut ms, player, &map, 0.016);
        assert!(matches!(ms[0].state, FpMosquitonState::MeleeAttack { .. }));
    }

    #[test]
    fn melee_attack_deals_direct_damage_without_projectile() {
        let map = test_map();
        let config = FpMosquitonConfig {
            melee_damage: 17,
            ..Default::default()
        };
        let melee_source = Vec2::new(2.0, 1.5);
        let mut ms = vec![FpMosquiton::new(melee_source, config)];
        ms[0].state = FpMosquitonState::MeleeAttack {
            timer: 0.2,
            dealt_damage: false,
        };

        let result = tick_mosquitons(&mut ms, Vec2::new(1.5, 1.5), &map, 0.016);

        assert_eq!(result.player_damage, 17);
        assert_eq!(result.damage_source, Some(melee_source));
        assert!(result.projectiles.is_empty());
        assert!(matches!(
            ms[0].state,
            FpMosquitonState::MeleeAttack {
                dealt_damage: true,
                ..
            }
        ));
    }

    #[test]
    fn melee_attack_deals_damage_once_per_animation() {
        let map = test_map();
        let mut ms = vec![make_mosquiton(2.0, 1.5)];
        ms[0].state = FpMosquitonState::MeleeAttack {
            timer: 0.2,
            dealt_damage: false,
        };

        let first = tick_mosquitons(&mut ms, Vec2::new(1.5, 1.5), &map, 0.016);
        let second = tick_mosquitons(&mut ms, Vec2::new(1.5, 1.5), &map, 0.016);

        assert!(first.player_damage > 0);
        assert_eq!(second.player_damage, 0);
    }

    #[test]
    fn shoots_while_pursuing_with_los() {
        let map = test_map();
        let config = FpMosquitonConfig {
            shoot_range: 10.0,
            preferred_range: 2.0,
            ..Default::default()
        };
        let mut ms = vec![FpMosquiton::new(Vec2::new(1.5, 1.5), config)];
        ms[0].shoot_cooldown = 0.0;
        let player = Vec2::new(5.5, 1.5);
        let result = tick_mosquitons(&mut ms, player, &map, 0.016);
        assert!(!result.projectiles.is_empty(), "should fire while pursuing");
    }

    #[test]
    fn no_shoot_without_los() {
        let map = FpMap {
            width: 5,
            height: 3,
            cells: vec![
                1, 1, 1, 1, 1, //
                1, 0, 1, 0, 1, //
                1, 1, 1, 1, 1,
            ],
        };
        let config = FpMosquitonConfig {
            shoot_range: 10.0,
            ..Default::default()
        };
        let mut ms = vec![FpMosquiton::new(Vec2::new(1.5, 1.5), config)];
        ms[0].shoot_cooldown = 0.0;
        let player = Vec2::new(3.5, 1.5);
        let result = tick_mosquitons(&mut ms, player, &map, 0.016);
        assert!(result.projectiles.is_empty());
        assert!(!has_line_of_sight(ms[0].position, player, &map));
    }

    #[test]
    fn dying_transitions_to_dead() {
        let map = test_map();
        let mut ms = vec![make_mosquiton(1.5, 1.5)];
        ms[0].state = FpMosquitonState::Dying { timer: 0.1 };
        let _ = tick_mosquitons(&mut ms, Vec2::ZERO, &map, 0.2);
        assert!(matches!(ms[0].state, FpMosquitonState::Dead));
    }

    #[test]
    fn hitscan_hits_mosquiton() {
        let map = test_map();
        let cam = FpCamera {
            position: Vec2::new(1.5, 1.5),
            angle: 0.0,
            ..Default::default()
        };
        let ms = vec![make_mosquiton(3.0, 1.5)];
        let hit = hitscan_mosquitons(&cam, &ms, &map);
        assert!(hit.is_some());
        assert_eq!(hit.unwrap().0, 0);
    }

    #[test]
    fn hitscan_misses_dead_mosquiton() {
        let map = test_map();
        let cam = FpCamera {
            position: Vec2::new(1.5, 1.5),
            angle: 0.0,
            ..Default::default()
        };
        let mut ms = vec![make_mosquiton(3.0, 1.5)];
        ms[0].state = FpMosquitonState::Dead;
        let hit = hitscan_mosquitons(&cam, &ms, &map);
        assert!(hit.is_none());
    }

    #[test]
    fn composed_billboard_sprites_are_resolved_from_embedded_assets() {
        let sprites = make_mosquiton_billboard_sprites().unwrap();
        assert!(sprites.alive.len() > 1);
        assert!(sprites.melee.len() > 1);
        assert!(sprites.death.width() > 1);
        assert!(sprites.death.height() > 1);
        assert!(
            sprites
                .alive
                .iter()
                .all(|frame| frame.sprite.width() > 1 && frame.sprite.height() > 1)
        );
        assert!(sprites.alive.iter().all(|frame| {
            frame
                .sprite
                .data()
                .iter()
                .any(|&pixel| pixel != TRANSPARENT_INDEX)
        }));
        assert!(sprites.melee.iter().all(|frame| {
            frame
                .sprite
                .data()
                .iter()
                .any(|&pixel| pixel != TRANSPARENT_INDEX)
        }));
        assert!(
            sprites
                .death
                .data()
                .iter()
                .any(|&pixel| pixel != TRANSPARENT_INDEX)
        );
    }

    #[test]
    fn melee_billboard_sprite_uses_melee_fly_animation() {
        let sprites = make_mosquiton_billboard_sprites().unwrap();
        assert!(
            sprites.melee.iter().any(|melee| {
                sprites.alive.iter().all(|alive| {
                    (
                        melee.sprite.width(),
                        melee.sprite.height(),
                        melee.sprite.data(),
                    ) != (
                        alive.sprite.width(),
                        alive.sprite.height(),
                        alive.sprite.data(),
                    )
                })
            }),
            "melee animation should contain authored frames outside the idle fly loop"
        );
    }

    #[test]
    fn alive_billboard_sprite_loops_idle_fly_frames() {
        let sprites = make_mosquiton_billboard_sprites().unwrap();
        let first = sprites.alive_sprite_at(0.0).data().to_vec();
        let second = sprites
            .alive_sprite_at(sprites.alive[0].duration + 0.001)
            .data()
            .to_vec();
        assert_ne!(first, second);
    }

    #[test]
    fn idle_fly_composition_animates_wings_only() {
        let composed: CompactComposedAtlas = ron::from_str(MOSQUITON_COMPOSED_RON).unwrap();
        let animation = composed
            .animations
            .iter()
            .find(|animation| animation.tag == MOSQUITON_IDLE_FLY_TAG)
            .unwrap();
        assert!(animation.frames.len() > 1);

        let wing_part_ids = composed
            .parts
            .iter()
            .filter(|part| part.tags.iter().any(|tag| tag == MOSQUITON_WING_TAG))
            .map(|part| part.id)
            .collect::<HashSet<_>>();
        assert!(!wing_part_ids.is_empty());

        let base_frame = &animation.frames[0];
        let wing_frame = &animation.frames[1];
        let merged = merge_wing_frame_poses(base_frame, wing_frame, &wing_part_ids);

        let base_non_wing = base_frame
            .poses
            .iter()
            .filter(|pose| !wing_part_ids.contains(&pose.p))
            .map(pose_key)
            .collect::<HashSet<_>>();
        let merged_non_wing = merged
            .iter()
            .filter(|pose| !wing_part_ids.contains(&pose.p))
            .map(pose_key)
            .collect::<HashSet<_>>();
        assert_eq!(
            base_non_wing, merged_non_wing,
            "body and other non-wing poses should stay locked to the base frame"
        );

        let merged_wings = merged
            .iter()
            .filter(|pose| wing_part_ids.contains(&pose.p))
            .map(pose_key)
            .collect::<HashSet<_>>();
        let animated_wings = wing_frame
            .poses
            .iter()
            .filter(|pose| wing_part_ids.contains(&pose.p))
            .map(pose_key)
            .collect::<HashSet<_>>();
        let base_wings = base_frame
            .poses
            .iter()
            .filter(|pose| wing_part_ids.contains(&pose.p))
            .map(pose_key)
            .collect::<HashSet<_>>();

        assert_eq!(animated_wings, merged_wings);
        assert_ne!(base_wings, merged_wings);
    }

    #[test]
    fn clamped_animation_holds_last_frame() {
        let sprites = make_blood_shot_billboard_sprites().unwrap();
        assert!(sprites.destroy.len() > 1, "need multi-frame destroy anim");

        let total: f32 = sprites.destroy.iter().map(|f| f.duration).sum();
        let first = sprites.destroy_sprite_at(0.0).data();
        let last_expected = &sprites.destroy[sprites.destroy.len() - 1].sprite;

        // Past the end should clamp to last frame, not loop back to first.
        let past_end = sprites.destroy_sprite_at(total + 1.0);
        assert!(
            std::ptr::eq(past_end, last_expected),
            "clamped animation should hold last frame, not loop"
        );
        // And the last frame should differ from the first (otherwise this test is vacuous).
        assert_ne!(first, past_end.data());
    }

    #[test]
    fn blood_shot_billboard_sprite_uses_embedded_asset() {
        let sprites = make_blood_shot_billboard_sprites().unwrap();
        assert!(sprites.hover.width() > 3);
        assert!(sprites.hover.height() > 3);
        assert!(sprites.hit.width() > sprites.hover.width());
        assert!(!sprites.destroy.is_empty());
        assert!(
            sprites
                .hover
                .data()
                .iter()
                .any(|&pixel| pixel != TRANSPARENT_INDEX)
        );
        assert!(
            sprites
                .hit
                .data()
                .iter()
                .any(|&pixel| pixel != TRANSPARENT_INDEX)
        );
    }

    #[test]
    fn idle_fly_frames_have_stable_dimensions() {
        let sprites = make_mosquiton_billboard_sprites().unwrap();
        let w = sprites.alive[0].sprite.width();
        let h = sprites.alive[0].sprite.height();
        for (i, frame) in sprites.alive.iter().enumerate() {
            assert_eq!(
                frame.sprite.width(),
                w,
                "frame {i} width {} != expected {w}",
                frame.sprite.width()
            );
            assert_eq!(
                frame.sprite.height(),
                h,
                "frame {i} height {} != expected {h}",
                frame.sprite.height()
            );
        }
    }

    #[test]
    fn default_hover_height_keeps_billboard_near_eye_level() {
        let mosquiton = make_mosquiton(2.0, 2.0);
        assert!(mosquiton.height <= 0.1);
        assert!(mosquiton.height > 0.0);
    }
}
