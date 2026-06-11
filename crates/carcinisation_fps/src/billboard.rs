//! Billboard sprite projection and rendering.
//!
//! # Colour transformation order
//!
//! When rendering a billboard pixel the transformations are applied in this
//! order, from earliest to latest:
//!
//! 1. **Sprite source** — raw palette index from the sprite data.
//! 2. **Pre-billboard transforms** — applied at [`Billboard`] construction
//!    time (e.g. [`make_damage_invert_sprite`], [`make_charred_sprite`]).
//!    These produce a new sprite; they are NOT visible inside this module.
//! 3. **Avatar palette remap** — [`AvatarPaletteRemap`] lookup that permutes
//!    colour-group indices (applied in [`draw_billboard`]).
//! 4. **Fog** — dithered distance fog replaces the pixel with `fog_color`
//!    (applied in [`draw_billboard`], after the remap).
//! 5. **Framebuffer write** — final pixel written to the output image.
//!
//! The remap is deliberately the *last* colour-space transformation so that
//! fog (which is palette-index agnostic — it just writes `fog_color`) does
//! not see stale pre-remap indices.

use std::collections::HashMap;
use std::sync::Arc;

use asset_pipeline::composed_ron::{CompactComposedAtlas, CompactPose};
use bevy_math::Vec2;
use carapace::image::CxImage;
use carapace::palette::TRANSPARENT_INDEX;

use crate::avatar_palette::AvatarPaletteRemap;
use crate::camera::Camera;
use carcinisation_fps_core::ProjectileKind;
use carcinisation_net::AvatarPaletteVariant;

use crate::enemy::{
    CRIT_IMPACT_HEIGHT_SCALE, Enemy, EnemyState, Projectile, ProjectileImpact, ProjectileImpactKind,
};
use crate::mosquiton::{
    BloodShotBillboardSprites, Mosquiton, MosquitonBillboardSprites, MosquitonState,
};
use crate::spidey::{SpiderShotBillboardSprites, Spidey, SpideyBillboardSprites, SpideyState};

const FP_DAMAGE_INVERT_MIN_COLOR_INDEX: u8 = 1;
const FP_DAMAGE_INVERT_MAX_COLOR_INDEX: u8 = 3;

/// A billboard entity in map space.
///
/// `sprite` is `Arc`-wrapped so cloning a billboard (e.g. when the renderer
/// collects all billboard sources into a single `Vec`) is a refcount bump
/// instead of a deep copy of pixel data.
#[derive(Clone, Debug)]
pub struct Billboard {
    /// Position in map-space units.
    pub position: Vec2,
    /// Vertical offset from the view centre, in map-space units.
    pub height: f32,
    /// Rendered billboard height, in map-space units.
    pub world_height: f32,
    /// Billboard sprite (palette-indexed, single frame).
    pub sprite: Arc<CxImage>,
    /// When true, the sprite is rendered horizontally mirrored.
    pub flip_x: bool,
    /// Server-assigned palette variant for avatar colour differentiation.
    /// `None` (the common case for enemies, effects, etc.) produces the
    /// identity remap at projection time.
    pub palette_variant: Option<AvatarPaletteVariant>,
}

/// Projected billboard ready for rendering.
pub(crate) struct ProjectedBillboard<'a> {
    /// Screen-space X of the billboard center.
    pub screen_x: f32,
    /// Perpendicular distance from camera plane.
    pub distance: f32,
    /// Sprite height in screen pixels.
    pub screen_h: i32,
    /// Sprite width in screen pixels.
    pub screen_w: i32,
    /// Screen-space vertical shift, in pixels.
    pub vertical_shift: i32,
    /// Reference to the source sprite.
    pub sprite: &'a CxImage,
    /// When true, the sprite is rendered horizontally mirrored.
    pub flip_x: bool,
    /// Palette remap table for avatar colour variation.
    /// Always present (defaults to identity for non-player billboards).
    pub palette_remap: AvatarPaletteRemap,
}

/// Project a billboard into screen space.
///
/// Returns `None` if the billboard is behind the camera.
pub(crate) fn project_billboard<'a>(
    billboard: &'a Billboard,
    camera: &Camera,
    screen_w: i32,
    screen_h: i32,
) -> Option<ProjectedBillboard<'a>> {
    let dir = camera.direction();
    let plane = camera.plane();

    // Vector from camera to billboard.
    let rel = billboard.position - camera.position;

    // Transform to camera space.
    // inv_det = 1 / (plane.x * dir.y - dir.x * plane.y)
    let det = plane.x.mul_add(dir.y, -(dir.x * plane.y));
    if det.abs() < 1e-10 {
        return None;
    }
    let inv_det = 1.0 / det;
    // transform_x = lateral offset in camera plane
    // transform_y = depth (forward distance)
    let transform_x = inv_det * dir.y.mul_add(rel.x, -(dir.x * rel.y));
    let transform_y = inv_det * (-plane.y).mul_add(rel.x, plane.x * rel.y);

    // Behind camera.
    if transform_y <= 0.05 {
        return None;
    }

    // Screen X position.
    let sx = (screen_w as f32 / 2.0) * (1.0 + transform_x / transform_y);

    // Scale by distance.
    let sprite_screen_h = (screen_h as f32 * billboard.world_height / transform_y).abs() as i32;
    let aspect = billboard.sprite.width() as f32 / billboard.sprite.height().max(1) as f32;
    let sprite_screen_w = (sprite_screen_h as f32 * aspect) as i32;
    let vertical_shift = (screen_h as f32 * billboard.height / transform_y) as i32;

    // Too small to draw — avoid zero-division in texture sampling.
    if sprite_screen_h < 1 || sprite_screen_w < 1 {
        return None;
    }

    // Too large — billboard fills the screen, looks broken at close range.
    if sprite_screen_h > screen_h + screen_h / 2 || sprite_screen_w > screen_w + screen_w / 2 {
        return None;
    }

    Some(ProjectedBillboard {
        screen_x: sx,
        distance: transform_y,
        screen_h: sprite_screen_h,
        screen_w: sprite_screen_w,
        vertical_shift,
        sprite: &billboard.sprite,
        flip_x: billboard.flip_x,
        palette_remap: billboard
            .palette_variant
            .map(AvatarPaletteRemap::from_variant)
            .unwrap_or_default(),
    })
}

/// Render a projected billboard into the image, respecting the per-column z-buffer.
///
/// Colour transformation order (latest first):
///   source pixel → [pre-billboard transforms] → **palette remap** → fog → fb write
///
/// The palette remap is applied *before* fog so that fog (which is
/// palette-index agnostic) sees the final colour index.
///
/// Applies distance fog when `fog` is `Some((fog_color, fog_t))`.
/// `view_bob_px` shifts the billboard vertically to match the wall horizon bob.
pub(crate) fn draw_billboard(
    image: &mut CxImage,
    zbuffer: &[f32],
    proj: &ProjectedBillboard,
    screen_h: i32,
    fog: Option<(u8, f32)>,
    view_bob_px: i32,
) {
    let img_w = image.width() as i32;
    let half_h = screen_h / 2 + view_bob_px;

    let draw_start_y = half_h - proj.screen_h / 2 - proj.vertical_shift;
    let draw_end_y = draw_start_y + proj.screen_h;
    let draw_start_x = (proj.screen_x as i32) - proj.screen_w / 2;
    let draw_end_x = draw_start_x + proj.screen_w;

    let tex_w = proj.sprite.width() as i32;
    let tex_h = proj.sprite.height() as i32;
    if tex_w == 0 || tex_h == 0 {
        return;
    }

    let sprite_data = proj.sprite.data();
    let img_h = image.height() as i32;
    let img_data = image.data_mut();

    for x in draw_start_x.max(0)..draw_end_x.min(img_w) {
        // Z-buffer check: only draw if billboard is closer than wall.
        if proj.distance >= zbuffer[x as usize] {
            continue;
        }

        let raw_tex_x = ((x - draw_start_x) * tex_w / proj.screen_w).min(tex_w - 1);
        let tex_x = if proj.flip_x {
            tex_w - 1 - raw_tex_x
        } else {
            raw_tex_x
        };

        for y in draw_start_y.max(0)..draw_end_y.min(img_h) {
            let tex_y = ((y - draw_start_y) * tex_h / proj.screen_h).min(tex_h - 1);
            let pixel = sprite_data[(tex_y * tex_w + tex_x) as usize];
            if pixel != TRANSPARENT_INDEX {
                // Step 3: avatar palette remap (always applied; identity for
                // non-player billboards, so this is a no-op for them).
                let after_remap = proj.palette_remap.apply(pixel);

                // Step 4: fog (if applicable).
                let final_pixel = if let Some((fog_color, fog_t)) = fog {
                    let fog_level = (fog_t * 16.0) as u8;
                    let threshold = crate::render::BAYER_4X4[(y & 3) as usize][(x & 3) as usize];
                    if fog_level > threshold {
                        fog_color
                    } else {
                        after_remap
                    }
                } else {
                    after_remap
                };
                img_data[(y * img_w + x) as usize] = final_pixel;
            }
        }
    }
}

/// Create a simple procedural pillar sprite.
#[must_use]
pub fn make_pillar_sprite(width: u32, height: u32, color: u8) -> CxImage {
    let mut data = vec![TRANSPARENT_INDEX; (width * height) as usize];

    for y in 0..height {
        for x in 0..width {
            // Simple rectangle with 1px border darkening.
            let border = x == 0 || x == width - 1 || y == 0 || y == height - 1;
            data[(y * width + x) as usize] = if border {
                color.saturating_sub(1).max(1)
            } else {
                color
            };
        }
    }

    CxImage::new(data, width as usize)
}

/// Create a simple procedural enemy sprite (diamond shape).
#[must_use]
pub fn make_enemy_sprite(size: u32, color: u8) -> CxImage {
    let mut data = vec![TRANSPARENT_INDEX; (size * size) as usize];
    let half = size as i32 / 2;

    for y in 0..size as i32 {
        for x in 0..size as i32 {
            let dx = (x - half).abs();
            let dy = (y - half).abs();
            if dx + dy <= half {
                let edge = dx + dy > half - 2;
                data[(y as u32 * size + x as u32) as usize] = if edge {
                    color.saturating_sub(1).max(1)
                } else {
                    color
                };
            }
        }
    }

    CxImage::new(data, size as usize)
}

/// Create a clearly Mosquiton-shaped temporary network billboard fallback.
///
/// Multiplayer clients normally render `NetEnemy` Mosquitons using the composed
/// Mosquiton billboard sprites loaded by the FPS plugin. This fallback keeps
/// the enemy visually distinct from generic diamonds in headless tests or
/// minimal harnesses that do not load the composed sprite resource.
#[allow(clippy::similar_names)]
#[must_use]
pub fn make_mosquiton_placeholder_sprite(size: u32, color: u8) -> CxImage {
    let mut data = vec![TRANSPARENT_INDEX; (size * size) as usize];
    let half = size as i32 / 2;
    let body_rx = (size as i32 / 7).max(2);
    let body_ry = (size as i32 / 4).max(3);
    let wing_rx = (size as i32 / 4).max(4);
    let wing_ry = (size as i32 / 6).max(3);

    for y in 0..size as i32 {
        for x in 0..size as i32 {
            let dx = x - half;
            let dy = y - half;
            let body = dx * dx * body_ry * body_ry + dy * dy * body_rx * body_rx
                <= body_rx * body_rx * body_ry * body_ry;
            let left_wing_dx = x - (half - body_rx - wing_rx / 2);
            let right_wing_dx = x - (half + body_rx + wing_rx / 2);
            let wing_dy = y - (half - body_ry / 2);
            let left_wing = left_wing_dx * left_wing_dx * wing_ry * wing_ry
                + wing_dy * wing_dy * wing_rx * wing_rx
                <= wing_rx * wing_rx * wing_ry * wing_ry;
            let right_wing = right_wing_dx * right_wing_dx * wing_ry * wing_ry
                + wing_dy * wing_dy * wing_rx * wing_rx
                <= wing_rx * wing_rx * wing_ry * wing_ry;
            if body || left_wing || right_wing {
                data[(y as u32 * size + x as u32) as usize] = if body {
                    color
                } else {
                    color.saturating_add(1).min(15)
                };
            }
        }
    }

    CxImage::new(data, size as usize)
}

/// Create a death-frame sprite (X shape).
#[must_use]
pub fn make_death_sprite(size: u32, color: u8) -> CxImage {
    let mut data = vec![TRANSPARENT_INDEX; (size * size) as usize];

    for i in 0..size {
        // Two diagonals forming an X.
        let j = size - 1 - i;
        for &(x, y) in &[(i, i), (i, j)] {
            if x < size && y < size {
                data[(y * size + x) as usize] = color;
                // Thicken: adjacent pixel.
                if x + 1 < size {
                    data[(y * size + x + 1) as usize] = color;
                }
            }
        }
    }

    CxImage::new(data, size as usize)
}

/// Create a small filled circle sprite for enemy blood-shot projectiles.
#[must_use]
pub fn make_blood_shot_sprite(size: u32, color: u8) -> CxImage {
    let mut data = vec![TRANSPARENT_INDEX; (size * size) as usize];
    let half = size as f32 / 2.0;
    let radius_sq = half * half;

    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 + 0.5 - half;
            let dy = y as f32 + 0.5 - half;
            if dx * dx + dy * dy <= radius_sq {
                data[(y * size + x) as usize] = color;
            }
        }
    }

    CxImage::new(data, size as usize)
}

/// Create a small green-cross sprite for health pickup feedback billboards.
#[must_use]
pub fn make_health_pickup_sprite(size: u32) -> CxImage {
    let mut data = vec![TRANSPARENT_INDEX; (size * size) as usize];
    let half = size as i32 / 2;

    for y in 0..size as i32 {
        for x in 0..size as i32 {
            let dx = (x - half).abs();
            let dy = (y - half).abs();
            if dx + dy <= half {
                let is_cross = (dx <= 1 && dy < half) || (dy <= 1 && dx < half);
                data[(y as u32 * size + x as u32) as usize] = if is_cross { 5 } else { 4 };
            }
        }
    }

    CxImage::new(data, size as usize)
}

/// Darken a palette-indexed sprite for fire-death corpses.
#[must_use]
pub fn make_charred_sprite(sprite: &CxImage) -> CxImage {
    let data = sprite
        .data()
        .iter()
        .map(|pixel| {
            if *pixel == TRANSPARENT_INDEX {
                TRANSPARENT_INDEX
            } else {
                1
            }
        })
        .collect();
    CxImage::new(data, sprite.width())
}

/// Temporary fire-death corpse sprite hook.
///
/// Dedicated burn animation can replace this without changing callers.
#[must_use]
pub fn make_burning_corpse_sprite(sprite: &CxImage) -> CxImage {
    make_charred_sprite(sprite)
}

/// Which transient flash an enemy billboard should render this frame.
///
/// Combat-feedback differentiation: a hit and a poise-break/stagger are
/// distinct outcomes and must read differently. Priority is `Stagger > Hit >
/// None` — a stun tick usually coincides with a recent hit flicker, so the
/// stagger visual must win or it would be masked by the hit flash. Driven
/// entirely by shared simulation state (`EnemyReactionState`/`NetEnemy.stunned`
/// and the local damage flicker), never inferred from presentation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnemyFlash {
    None,
    /// Brief white invert on a hit (the existing damage flicker).
    Hit,
    /// Sustained "dazed" dim while hit-stunned (poise break / stagger).
    Stagger,
}

/// Apply the per-frame flash transform for `flash` to a billboard sprite.
///
/// Recomputed per frame like the existing invert (no caching) — the flash is
/// brief and rare, so the allocation matches the established cost model.
#[must_use]
pub fn flash_sprite(sprite: &Arc<CxImage>, flash: EnemyFlash) -> Arc<CxImage> {
    match flash {
        EnemyFlash::Hit => Arc::new(make_damage_invert_sprite(sprite)),
        EnemyFlash::Stagger => Arc::new(make_stagger_tint_sprite(sprite)),
        EnemyFlash::None => Arc::clone(sprite),
    }
}

/// Stagger tint: dim every opaque pixel one step down the brightness ramp,
/// floored at the darkest opaque index, preserving transparency and silhouette
/// (`3→2`, `2→1`, `1→1`, `0` stays transparent). Distinct from the white
/// damage-invert (which brightens) and from the flat charred fill — reads as a
/// "dazed/reeling" enemy. Presentation only.
#[must_use]
pub fn make_stagger_tint_sprite(sprite: &CxImage) -> CxImage {
    let data = sprite
        .data()
        .iter()
        .map(|pixel| {
            if *pixel == TRANSPARENT_INDEX {
                TRANSPARENT_INDEX
            } else {
                pixel.saturating_sub(1).max(1)
            }
        })
        .collect();
    CxImage::new(data, sprite.width())
}

/// Apply the same visual role as the ORS hit invert filter to an FP sprite.
#[must_use]
pub fn make_damage_invert_sprite(sprite: &CxImage) -> CxImage {
    let data = sprite
        .data()
        .iter()
        .map(|pixel| {
            if *pixel == TRANSPARENT_INDEX {
                TRANSPARENT_INDEX
            } else if (FP_DAMAGE_INVERT_MIN_COLOR_INDEX..=FP_DAMAGE_INVERT_MAX_COLOR_INDEX)
                .contains(pixel)
            {
                FP_DAMAGE_INVERT_MAX_COLOR_INDEX + FP_DAMAGE_INVERT_MIN_COLOR_INDEX - *pixel
            } else {
                FP_DAMAGE_INVERT_MAX_COLOR_INDEX
            }
        })
        .collect();
    CxImage::new(data, sprite.width())
}

fn enemy_presentation_sprite(enemy: &Enemy, alive: &CxImage, death: &CxImage) -> Arc<CxImage> {
    match enemy.state {
        EnemyState::BurningCorpse { .. } => Arc::new(make_burning_corpse_sprite(alive)),
        _ if enemy.showing_damage_invert() => Arc::new(make_damage_invert_sprite(alive)),
        EnemyState::Dying { .. } => Arc::new(death.clone()),
        _ => Arc::new(alive.clone()),
    }
}

/// Build billboard list from enemies (alive and dying).
/// Dead enemies are excluded. Dying enemies use the death sprite.
#[must_use]
pub fn billboards_from_enemies(
    enemies: &[Enemy],
    alive_sprite: &CxImage,
    death_sprite: &CxImage,
) -> Vec<Billboard> {
    enemies
        .iter()
        .filter(|e| !matches!(e.state, EnemyState::Dead))
        .map(|e| Billboard {
            position: e.position,
            height: 0.0,
            world_height: 1.0,
            sprite: enemy_presentation_sprite(e, alive_sprite, death_sprite),
            flip_x: false,
            palette_variant: None,
        })
        .collect()
}

/// Build billboard list from enemies with per-enemy sprite pairs.
///
/// `sprite_indices[i]` maps enemy `i` to a sprite pair in `sprite_pairs`.
/// Enemies without a valid index use the first pair as fallback.
/// Build a single billboard from one enemy (used during setup before entities exist).
#[must_use]
pub fn billboard_from_enemy(
    enemy: &Enemy,
    sprite_index: usize,
    sprite_pairs: &[(CxImage, CxImage)],
) -> Billboard {
    let (alive, death) = sprite_pairs.get(sprite_index).unwrap_or(&sprite_pairs[0]);
    Billboard {
        position: enemy.position,
        height: 0.0,
        world_height: 1.0,
        sprite: enemy_presentation_sprite(enemy, alive, death),
        flip_x: false,
        palette_variant: None,
    }
}

#[must_use]
pub fn billboards_from_enemies_indexed(
    enemies: &[Enemy],
    sprite_indices: &[usize],
    sprite_pairs: &[(CxImage, CxImage)],
) -> Vec<Billboard> {
    enemies
        .iter()
        .enumerate()
        .filter(|(_, e)| !matches!(e.state, EnemyState::Dead))
        .map(|(i, e)| {
            let pair_idx = sprite_indices.get(i).copied().unwrap_or(0);
            let (alive, death) = sprite_pairs.get(pair_idx).unwrap_or(&sprite_pairs[0]);
            Billboard {
                position: e.position,
                height: 0.0,
                world_height: 1.0,
                sprite: enemy_presentation_sprite(e, alive, death),
                flip_x: false,
                palette_variant: None,
            }
        })
        .collect()
}

/// Build billboard list from active projectiles, using the appropriate sprite
/// based on `ProjectileKind`.
#[must_use]
pub fn billboards_from_projectiles(
    projectiles: &[Projectile],
    blood_shot_sprite: &Arc<CxImage>,
    spider_shot_sprite: &Arc<CxImage>,
) -> Vec<Billboard> {
    projectiles
        .iter()
        .filter(|p| p.alive)
        .map(|p| {
            let (sprite, height) = match p.kind {
                ProjectileKind::BloodShot => (Arc::clone(blood_shot_sprite), 0.15),
                ProjectileKind::WebShot { .. } => {
                    // Lob arc from spider body height toward ground level.
                    let elapsed = p.initial_lifetime - p.lifetime;
                    let t = (elapsed / p.initial_lifetime.max(f32::EPSILON)).clamp(0.0, 1.0);
                    // Start below horizon (-0.2), arc up (+0.15 peak), descend.
                    let arc = 0.25 * 4.0 * t * (1.0 - t);
                    (Arc::clone(spider_shot_sprite), -0.2 + arc)
                }
            };
            Billboard {
                position: p.position,
                height,
                world_height: 0.3,
                sprite,
                flip_x: false,
                palette_variant: None,
            }
        })
        .collect()
}

/// Build billboard list from projectile impact effects.
#[must_use]
pub fn billboards_from_projectile_impacts(
    impacts: &[ProjectileImpact],
    blood_sprites: &BloodShotBillboardSprites,
    spider_sprites: &SpiderShotBillboardSprites,
) -> Vec<Billboard> {
    impacts
        .iter()
        .map(|impact| {
            let is_web = matches!(impact.source_kind, ProjectileKind::WebShot { .. });
            let sprite = match impact.kind {
                ProjectileImpactKind::Hit => {
                    if is_web {
                        Arc::clone(&spider_sprites.hit)
                    } else {
                        Arc::clone(&blood_sprites.hit)
                    }
                }
                ProjectileImpactKind::Destroy => {
                    if is_web {
                        Arc::clone(spider_sprites.destroy_sprite_at(impact.age))
                    } else {
                        Arc::clone(blood_sprites.destroy_sprite_at(impact.age))
                    }
                }
            };
            // WebShot impacts inherit the arc height; BloodShot stays at 0.15.
            let height = if matches!(impact.source_kind, ProjectileKind::WebShot { .. })
                && impact.visual_height != 0.0
            {
                impact.visual_height
            } else if matches!(impact.source_kind, ProjectileKind::WebShot { .. }) {
                -0.1 // default web impact near ground
            } else {
                0.15
            };
            Billboard {
                position: impact.position,
                height,
                // Critical (weak-point) hits render a larger splat for feedback
                // emphasis (presentation only — see CRIT_IMPACT_HEIGHT_SCALE).
                world_height: match impact.kind {
                    ProjectileImpactKind::Hit if impact.critical => 0.42 * CRIT_IMPACT_HEIGHT_SCALE,
                    ProjectileImpactKind::Hit => 0.42,
                    ProjectileImpactKind::Destroy => 0.36,
                },
                sprite,
                flip_x: false,
                palette_variant: None,
            }
        })
        .collect()
}

/// Build billboard list from Mosquiton enemies.
/// Build a single billboard from one mosquiton (used during setup before entities exist).
#[must_use]
pub fn billboard_from_mosquiton(
    mosquiton: &Mosquiton,
    sprites: &MosquitonBillboardSprites,
) -> Billboard {
    Billboard {
        position: mosquiton.position,
        height: mosquiton.height,
        world_height: mosquiton.config.billboard_height,
        palette_variant: None,
        sprite: {
            // Death/burning branches select their own sprite and never flash.
            let flash = mosquiton_flash(mosquiton);
            match mosquiton.state {
                MosquitonState::Dying { .. } => Arc::clone(&sprites.death),
                MosquitonState::BurningCorpse { .. } => {
                    Arc::new(make_burning_corpse_sprite(sprites.alive_sprite_at(0.0)))
                }
                MosquitonState::MeleeAttack { .. } => {
                    flash_sprite(sprites.melee_sprite_at(mosquiton.animation_time), flash)
                }
                _ => flash_sprite(sprites.alive_sprite_at(mosquiton.animation_time), flash),
            }
        },
        flip_x: false,
    }
}

/// Classify the transient flash for a Mosquiton from shared sim state.
/// `Stagger > Hit > None`; dead/dying enemies never flash.
fn mosquiton_flash(m: &Mosquiton) -> EnemyFlash {
    if !m.is_alive() {
        EnemyFlash::None
    } else if m.reaction.is_stunned() {
        EnemyFlash::Stagger
    } else if m.showing_hit_flash() {
        EnemyFlash::Hit
    } else {
        EnemyFlash::None
    }
}

/// Classify the transient flash for a Spidey from shared sim state.
/// `Stagger > Hit > None`; dead/dying enemies never flash.
fn spidey_flash(s: &Spidey) -> EnemyFlash {
    if !s.is_alive() {
        EnemyFlash::None
    } else if s.reaction.is_stunned() {
        EnemyFlash::Stagger
    } else if s.showing_hit_flash() {
        EnemyFlash::Hit
    } else {
        EnemyFlash::None
    }
}

#[must_use]
pub fn billboards_from_mosquitons(
    mosquitons: &[Mosquiton],
    sprites: &MosquitonBillboardSprites,
) -> Vec<Billboard> {
    mosquitons
        .iter()
        .filter(|m| !matches!(m.state, MosquitonState::Dead))
        .map(|m| Billboard {
            position: m.position,
            height: m.height,
            world_height: m.config.billboard_height,
            palette_variant: None,
            sprite: {
                let flash = mosquiton_flash(m);
                match m.state {
                    MosquitonState::Dying { .. } => Arc::clone(&sprites.death),
                    MosquitonState::BurningCorpse { .. } => {
                        Arc::new(make_burning_corpse_sprite(sprites.alive_sprite_at(0.0)))
                    }
                    MosquitonState::MeleeAttack { .. } => {
                        flash_sprite(sprites.melee_sprite_at(m.animation_time), flash)
                    }
                    _ => {
                        let sprite = m.shoot_anim_elapsed.map_or_else(
                            || sprites.alive_sprite_at(m.animation_time),
                            |elapsed| sprites.shoot_sprite_at(elapsed),
                        );
                        flash_sprite(sprite, flash)
                    }
                }
            },
            flip_x: false,
        })
        .collect()
}

/// Build a single billboard from one Spidey (used during setup before entities exist).
#[must_use]
pub fn billboard_from_spidey(spidey: &Spidey, sprites: &SpideyBillboardSprites) -> Billboard {
    // Anchor bottom edge to floor: center = -0.5 + half_height + visual arc.
    let grounded_height =
        crate::spidey::FLOOR_OFFSET + spidey.config.billboard_height / 2.0 + spidey.visual_height;
    Billboard {
        position: spidey.position,
        height: grounded_height,
        world_height: spidey.config.billboard_height,
        sprite: spidey_presentation_sprite(spidey, sprites),
        flip_x: false,
        palette_variant: None,
    }
}

/// Build billboard list from Spidey enemies using composed billboard sprites.
#[must_use]
pub fn billboards_from_spideys(
    spideys: &[Spidey],
    sprites: &SpideyBillboardSprites,
) -> Vec<Billboard> {
    spideys
        .iter()
        .filter(|s| !matches!(s.state, SpideyState::Dead))
        .map(|s| {
            let grounded_height =
                crate::spidey::FLOOR_OFFSET + s.config.billboard_height / 2.0 + s.visual_height;
            Billboard {
                position: s.position,
                height: grounded_height,
                world_height: s.config.billboard_height,
                sprite: spidey_presentation_sprite(s, sprites),
                flip_x: false,
                palette_variant: None,
            }
        })
        .collect()
}

/// Shared Spidey sprite selection from `EnemyPresentationState`.
///
/// Used by both SP (from local sim state) and MP client (from net state).
/// `animation_time` is used for idle/moving/recover looping animation where
/// the presentation state does not carry a phase.
#[must_use]
pub fn spidey_sprite_for_presentation(
    state: &carcinisation_fps_core::EnemyPresentationState,
    sprites: &SpideyBillboardSprites,
    flash: EnemyFlash,
    animation_time: f32,
) -> Arc<CxImage> {
    use carcinisation_fps_core::presentation::{AttackPresentationKind, EnemyPresentationState};

    match state {
        EnemyPresentationState::Dead { burn: true }
        | EnemyPresentationState::Dying { burn: true, .. } => {
            Arc::new(make_burning_corpse_sprite(sprites.alive_sprite_at(0.0)))
        }
        EnemyPresentationState::Dead { burn: false }
        | EnemyPresentationState::Dying { burn: false, .. } => Arc::clone(&sprites.death),
        EnemyPresentationState::Hopping { phase, .. } => {
            // Convert normalized phase (0-1) to elapsed seconds so the full
            // hop animation plays across one hop. Identical for SP and MP.
            let elapsed = phase * sprites.hop_total_duration();
            flash_sprite(sprites.hop_sprite_at(elapsed), flash)
        }
        EnemyPresentationState::Windup {
            attack: AttackPresentationKind::Ranged,
            phase,
        }
        | EnemyPresentationState::Attacking {
            attack: AttackPresentationKind::Ranged,
            phase,
        } => flash_sprite(sprites.shoot_sprite_at(*phase), flash),
        EnemyPresentationState::Windup {
            attack: AttackPresentationKind::Melee,
            phase,
        }
        | EnemyPresentationState::Attacking {
            attack: AttackPresentationKind::Melee,
            phase,
        } => {
            // Lunge animation — clamped to last frame until landing.
            flash_sprite(sprites.lunge_sprite_at(*phase), flash)
        }
        // Idle, Moving, Recover — use looping idle animation.
        EnemyPresentationState::Idle
        | EnemyPresentationState::Moving
        | EnemyPresentationState::Recover => {
            flash_sprite(sprites.alive_sprite_at(animation_time), flash)
        }
    }
}

/// Resolve the current display sprite for a Spidey based on its state.
///
/// Delegates to `spidey_sprite_for_presentation` via the SP presentation
/// adapter.
fn spidey_presentation_sprite(spidey: &Spidey, sprites: &SpideyBillboardSprites) -> Arc<CxImage> {
    let pres = crate::spidey::spidey_presentation_state(
        &spidey.state,
        spidey.animation_time,
        spidey.visual_height,
    );
    spidey_sprite_for_presentation(&pres, sprites, spidey_flash(spidey), spidey.animation_time)
}

// ── Pickup billboard sprites from composed atlas ──────────────────────────

/// Cached FPS billboard sprites for each pickup kind.
#[derive(bevy::prelude::Resource)]
pub struct PickupBillboardSprites {
    pub health: Arc<CxImage>,
    pub ammo: Arc<CxImage>,
    pub weapon: Arc<CxImage>,
}

impl PickupBillboardSprites {
    #[must_use]
    pub const fn sprite_for_kind(&self, kind: carcinisation_net::NetPickupKind) -> &Arc<CxImage> {
        match kind {
            carcinisation_net::NetPickupKind::Health => &self.health,
            carcinisation_net::NetPickupKind::Ammo => &self.ammo,
            carcinisation_net::NetPickupKind::Weapon => &self.weapon,
        }
    }
}

const PICKUP_COMPOSED_RON: &str =
    include_str!("../../../assets/sprites/pickups/pickup_3/atlas.composed.ron");
const PICKUP_PX_ATLAS_RON: &str =
    include_str!("../../../assets/sprites/pickups/pickup_3/atlas.px_atlas.ron");
const PICKUP_PXI: &[u8] = include_bytes!("../../../assets/sprites/pickups/pickup_3/atlas.pxi");

/// Compose a billboard sprite from the pickup composed atlas, including
/// only the parts whose semantic name matches one of `part_names`.
///
/// # Errors
///
/// # Panics
///
/// If part names are not found in the composed atlas's string table.
///
/// Returns an error if any name in `part_names` is not found in the
/// composed atlas's part string table, or if the PXI data is malformed.
fn compose_pickup_sprite(
    composed: &CompactComposedAtlas,
    atlas: &crate::mosquiton::PxAtlasDescriptor,
    atlas_pixels: &[u8],
    atlas_width: u32,
    part_names: &[&str],
) -> Result<Arc<CxImage>, String> {
    struct Placement {
        sprite_idx: usize,
        top_left: (i32, i32),
        size: (u32, u32),
        flip_x: bool,
        flip_y: bool,
    }

    let include_ids: Vec<u8> = part_names
        .iter()
        .map(|name| {
            composed
                .part_names
                .iter()
                .position(|n| n == name)
                .map(|i| i as u8)
                .ok_or_else(|| format!("Pickup part '{name}' not found in composed atlas"))
        })
        .collect::<Result<Vec<_>, _>>()?;

    let idle = composed
        .animations
        .iter()
        .find(|a| a.tag == "idle")
        .ok_or_else(|| "No 'idle' animation in pickup atlas".to_string())?;
    let frame = idle
        .frames
        .first()
        .ok_or_else(|| "Empty 'idle' animation in pickup atlas".to_string())?;

    let filtered: Vec<&CompactPose> = frame
        .poses
        .iter()
        .filter(|pose| include_ids.contains(&pose.p))
        .collect();
    if filtered.is_empty() {
        return Err("No poses match the requested pickup parts".to_string());
    }

    let mut grouped: HashMap<u8, Vec<&CompactPose>> = HashMap::new();
    for &pose in &filtered {
        grouped.entry(pose.p).or_default().push(pose);
    }
    for fragments in grouped.values_mut() {
        fragments.sort_by_key(|pose| pose.frag);
    }

    let part_map: HashMap<u8, &asset_pipeline::composed_ron::CompactPart> =
        composed.parts.iter().map(|part| (part.id, part)).collect();

    let mut placements = Vec::new();
    for (&part_id, fragments) in &grouped {
        let Some(part) = part_map.get(&part_id) else {
            continue;
        };
        let Some(primary) = fragments.first() else {
            continue;
        };
        let absolute_pivot = (i32::from(primary.o.0), i32::from(primary.o.1));

        for &pose in fragments {
            let frag_pivot = if pose.frag == 0 {
                absolute_pivot
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

    let min_x = placements.iter().map(|p| p.top_left.0).min().unwrap_or(0);
    let min_y = placements.iter().map(|p| p.top_left.1).min().unwrap_or(0);
    let max_x = placements
        .iter()
        .map(|p| p.top_left.0 + p.size.0 as i32)
        .max()
        .unwrap_or(1);
    let max_y = placements
        .iter()
        .map(|p| p.top_left.1 + p.size.1 as i32)
        .max()
        .unwrap_or(1);

    let width = (max_x - min_x).max(1) as u32;
    let height = (max_y - min_y).max(1) as u32;
    let mut data = vec![TRANSPARENT_INDEX; (width * height) as usize];

    for placement in &placements {
        let rect = atlas
            .regions
            .get(placement.sprite_idx)
            .and_then(|region| region.frames.first())
            .copied()
            .ok_or_else(|| format!("missing atlas rect for sprite {}", placement.sprite_idx))?;

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
                let pixel = atlas_pixels
                    .get(src_idx)
                    .copied()
                    .unwrap_or(TRANSPARENT_INDEX);
                if pixel == TRANSPARENT_INDEX {
                    continue;
                }
                let dst_x = (placement.top_left.0 - min_x) as u32 + local_x;
                let dst_y = (placement.top_left.1 - min_y) as u32 + local_y;
                if dst_x < width && dst_y < height {
                    data[(dst_y * width + dst_x) as usize] = pixel;
                }
            }
        }
    }

    Ok(Arc::new(CxImage::new(data, width as usize)))
}

/// Build cached pickup billboard sprites from the embedded composed atlas.
///
/// # Errors
///
/// Returns an error if the embedded RON/PXI data is malformed, or if a
/// required part name (e.g. `health_l`) is missing from the composed atlas.
pub fn make_pickup_billboard_sprites() -> Result<PickupBillboardSprites, String> {
    use crate::mosquiton::{PxAtlasDescriptor, decode_pxi};

    let composed: CompactComposedAtlas =
        ron::from_str(PICKUP_COMPOSED_RON).map_err(|e| e.to_string())?;
    let atlas: PxAtlasDescriptor = ron::from_str(PICKUP_PX_ATLAS_RON).map_err(|e| e.to_string())?;
    let (pxi_w, _pxi_h, pxi_pixels) = decode_pxi(PICKUP_PXI)?;

    Ok(PickupBillboardSprites {
        health: compose_pickup_sprite(&composed, &atlas, &pxi_pixels, pxi_w, &["health_l"])?,
        ammo: compose_pickup_sprite(&composed, &atlas, &pxi_pixels, pxi_w, &["icon_bullet"])?,
        weapon: compose_pickup_sprite(&composed, &atlas, &pxi_pixels, pxi_w, &["weapon_case"])?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera::Camera;

    fn visible_bounds(sprite: &CxImage) -> Option<(usize, usize, usize, usize)> {
        let mut min_x = usize::MAX;
        let mut min_y = usize::MAX;
        let mut max_x = 0;
        let mut max_y = 0;
        let mut any = false;
        for y in 0..sprite.height() {
            for x in 0..sprite.width() {
                if sprite.data()[y * sprite.width() + x] == TRANSPARENT_INDEX {
                    continue;
                }
                any = true;
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
        any.then_some((min_x, min_y, max_x, max_y))
    }

    #[test]
    fn billboard_in_front_projects_near_center() {
        let cam = Camera::default(); // at (4,4), facing east
        let bb = Billboard {
            position: Vec2::new(6.0, 4.0), // directly ahead
            height: 0.0,
            world_height: 1.0,
            sprite: Arc::new(make_pillar_sprite(8, 16, 5)),
            flip_x: false,
            palette_variant: None,
        };
        let proj = project_billboard(&bb, &cam, 160, 144).unwrap();
        // Should be near screen center (x=80).
        assert!(
            (proj.screen_x - 80.0).abs() < 5.0,
            "screen_x should be near center, got {}",
            proj.screen_x
        );
        assert!(proj.distance > 0.0);
    }

    #[test]
    fn pickup_health_sprite_uses_tight_composed_bounds() {
        let sprites = make_pickup_billboard_sprites().expect("pickup sprites should load");
        assert_eq!(sprites.health.width(), 25);
        assert_eq!(sprites.health.height(), 23);
        assert_eq!(visible_bounds(&sprites.health), Some((0, 0, 24, 22)));
    }

    #[test]
    fn billboard_behind_camera_returns_none() {
        let cam = Camera::default(); // facing east
        let bb = Billboard {
            position: Vec2::new(2.0, 4.0), // behind
            height: 0.0,
            world_height: 1.0,
            sprite: Arc::new(make_pillar_sprite(8, 16, 5)),
            flip_x: false,
            palette_variant: None,
        };
        assert!(project_billboard(&bb, &cam, 160, 144).is_none());
    }

    #[test]
    fn billboard_very_far_returns_none_due_to_zero_size() {
        let cam = Camera::default();
        let bb = Billboard {
            position: Vec2::new(10000.0, 4.0),
            height: 0.0,
            world_height: 1.0,
            sprite: Arc::new(make_pillar_sprite(8, 16, 5)),
            flip_x: false,
            palette_variant: None,
        };
        // At extreme distance, projected size rounds to 0 → filtered out.
        assert!(project_billboard(&bb, &cam, 160, 144).is_none());
    }

    #[test]
    fn billboard_to_the_right_has_higher_screen_x() {
        let cam = Camera::default(); // facing east
        let center = Billboard {
            position: Vec2::new(6.0, 4.0),
            height: 0.0,
            world_height: 1.0,
            sprite: Arc::new(make_pillar_sprite(8, 16, 5)),
            flip_x: false,
            palette_variant: None,
        };
        let right = Billboard {
            position: Vec2::new(6.0, 3.0), // south = screen-right
            height: 0.0,
            world_height: 1.0,
            sprite: Arc::new(make_pillar_sprite(8, 16, 5)),
            flip_x: false,
            palette_variant: None,
        };
        let pc = project_billboard(&center, &cam, 160, 144).unwrap();
        let pr = project_billboard(&right, &cam, 160, 144).unwrap();
        assert!(
            pr.screen_x > pc.screen_x,
            "right billboard ({}) should have higher screen_x than center ({})",
            pr.screen_x,
            pc.screen_x
        );
    }

    #[test]
    fn burning_corpse_sprite_overrides_damage_invert() {
        let alive = make_enemy_sprite(8, 2);
        let death = make_death_sprite(8, 3);
        let mut enemy = Enemy::new(Vec2::new(6.0, 4.0), 10, 1.0);
        enemy.take_damage(1);
        enemy.damage_flicker = enemy.damage_flicker.and_then(|flicker| flicker.tick(0.1));
        assert!(enemy.showing_damage_invert());
        enemy.state = EnemyState::BurningCorpse {
            timer: 1.0,
            seed: 123,
        };

        let billboard = billboards_from_enemies(&[enemy], &alive, &death)
            .pop()
            .expect("burning corpse should still render");

        assert!(
            billboard
                .sprite
                .data()
                .iter()
                .filter(|pixel| **pixel != TRANSPARENT_INDEX)
                .all(|pixel| *pixel == 1)
        );
    }

    #[test]
    fn damage_invert_keeps_opaque_pixels_visible() {
        let sprite = CxImage::new(vec![TRANSPARENT_INDEX, 1, 2, 3, 4], 5);

        let inverted = make_damage_invert_sprite(&sprite);

        assert_eq!(inverted.data(), &[TRANSPARENT_INDEX, 3, 2, 1, 3]);
    }

    // -----------------------------------------------------------------------
    // Palette variant / remap tests
    // -----------------------------------------------------------------------

    #[test]
    fn project_billboard_palette_variant_none_uses_identity_remap() {
        let cam = Camera::default();
        let bb = Billboard {
            position: Vec2::new(6.0, 4.0),
            height: 0.0,
            world_height: 1.0,
            sprite: Arc::new(make_pillar_sprite(8, 16, 5)),
            flip_x: false,
            palette_variant: None,
        };
        let proj = project_billboard(&bb, &cam, 160, 144).unwrap();
        // Identity remap: every index maps to itself.
        for i in 0..16u8 {
            assert_eq!(
                proj.palette_remap.apply(i),
                i,
                "identity remap should preserve index {i}"
            );
        }
    }

    #[test]
    fn project_billboard_palette_variant_some_uses_variant_remap() {
        use carcinisation_net::AvatarPaletteVariant;
        let cam = Camera::default();
        let bb = Billboard {
            position: Vec2::new(6.0, 4.0),
            height: 0.0,
            world_height: 1.0,
            sprite: Arc::new(make_pillar_sprite(8, 16, 5)),
            flip_x: false,
            palette_variant: Some(AvatarPaletteVariant::Acb),
        };
        let proj = project_billboard(&bb, &cam, 160, 144).unwrap();
        // Acb swaps B and C groups.
        let [_a, b, c] = *crate::avatar_palette::colour_groups();
        assert_eq!(proj.palette_remap.apply(b), c, "Acb should map B→C");
        assert_eq!(proj.palette_remap.apply(c), b, "Acb should map C→B");
    }

    #[test]
    fn draw_billboard_applies_remap_to_opaque_pixels() {
        // Create a sprite with known palette indices.
        // Index 0 = transparent, a and b are colour groups, other indices are protected.
        use carcinisation_net::AvatarPaletteVariant;

        let [a, b, _c] = *crate::avatar_palette::colour_groups();
        let sprite_data = vec![0u8, 1, a, b, 0, 1, a, b];
        let sprite = Arc::new(CxImage::new(sprite_data.clone(), 4));

        let cam = Camera::default();
        let bb = Billboard {
            position: Vec2::new(6.0, 4.0),
            height: 0.0,
            world_height: 1.0,
            sprite,
            flip_x: false,
            palette_variant: Some(AvatarPaletteVariant::Bac), // A↔B swap
        };
        let proj = project_billboard(&bb, &cam, 160, 144).unwrap();

        let mut image = CxImage::empty(bevy_math::UVec2::new(160, 144));
        let zbuffer = vec![f32::MAX; 160];
        draw_billboard(&mut image, &zbuffer, &proj, 144, None, 0);

        // Find the non-zero pixels in the output and check remap.
        let img_data = image.data();
        let rendered: Vec<u8> = img_data.iter().copied().filter(|&p| p != 0).collect();

        // Bac swaps A↔B: index `a` should become `b`, index `b` should become `a`.
        // Protected indices in the input pass through unchanged.
        let prots_in_input: Vec<u8> = crate::avatar_palette::protected_indices()
            .iter()
            .copied()
            .filter(|&p| sprite_data.contains(&p) && p != 0)
            .collect();
        for &p in &prots_in_input {
            assert!(
                rendered.contains(&p),
                "protected index {p} must survive remap unchanged"
            );
        }
        assert!(
            rendered.contains(&b),
            "remapped colour group A should contain original B index"
        );
        assert!(
            rendered.contains(&a),
            "remapped colour group B should contain original A index"
        );
    }

    #[test]
    fn draw_billboard_identity_variant_does_not_change_pixels() {
        let sprite_data = vec![0u8, 1, 2, 3, 4, 5, 0, 1, 2, 3, 4, 5];
        let sprite = Arc::new(CxImage::new(sprite_data.clone(), 6));

        let cam = Camera::default();
        let bb = Billboard {
            position: Vec2::new(6.0, 4.0),
            height: 0.0,
            world_height: 1.0,
            sprite: Arc::clone(&sprite),
            flip_x: false,
            palette_variant: None,
        };
        let proj = project_billboard(&bb, &cam, 160, 144).unwrap();

        let mut image = CxImage::empty(bevy_math::UVec2::new(160, 144));
        let zbuffer = vec![f32::MAX; 160];
        draw_billboard(&mut image, &zbuffer, &proj, 144, None, 0);

        // Identity remap: every non-zero rendered pixel must be one of the input indices.
        let src_indices: std::collections::HashSet<u8> =
            sprite_data.iter().copied().filter(|&p| p != 0).collect();
        let rendered: std::collections::HashSet<u8> =
            image.data().iter().copied().filter(|&p| p != 0).collect();
        assert!(
            rendered.is_subset(&src_indices),
            "identity remap produced pixel indices not in source: {:?}",
            rendered
                .difference(&src_indices)
                .copied()
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn draw_billboard_remap_with_fog_does_not_crash() {
        use carcinisation_net::AvatarPaletteVariant;

        let [a, b, _c] = *crate::avatar_palette::colour_groups();
        let sprite_data = vec![0u8, a, b, 1, 0, a, b, 1];
        let sprite = Arc::new(CxImage::new(sprite_data, 4));

        let cam = Camera::default();
        let bb = Billboard {
            position: Vec2::new(6.0, 4.0),
            height: 0.0,
            world_height: 1.0,
            sprite,
            flip_x: false,
            palette_variant: Some(AvatarPaletteVariant::Acb),
        };
        let proj = project_billboard(&bb, &cam, 160, 144).unwrap();

        let mut image = CxImage::empty(bevy_math::UVec2::new(160, 144));
        let zbuffer = vec![f32::MAX; 160];
        // Fog at 50%: should not panic or produce 0 from remap+fog.
        draw_billboard(&mut image, &zbuffer, &proj, 144, Some((1, 0.5)), 0);

        // At least some pixels should be non-zero (remapped or fogged).
        assert!(
            image.data().iter().any(|&p| p != 0),
            "remap with fog should produce some visible pixels"
        );
    }

    #[test]
    fn draw_billboard_remap_with_flip_x_still_applies() {
        use carcinisation_net::AvatarPaletteVariant;

        let sprite_data = vec![0u8, 2, 3, 1, 0, 2, 3, 1];
        let sprite = Arc::new(CxImage::new(sprite_data, 4));

        let cam = Camera::default();
        let bb_flip = Billboard {
            position: Vec2::new(6.0, 4.0),
            height: 0.0,
            world_height: 1.0,
            sprite: Arc::clone(&sprite),
            flip_x: true,
            palette_variant: Some(AvatarPaletteVariant::Bac),
        };
        let bb_noflip = Billboard {
            position: Vec2::new(6.0, 4.0),
            height: 0.0,
            world_height: 1.0,
            sprite: Arc::clone(&sprite),
            flip_x: false,
            palette_variant: Some(AvatarPaletteVariant::Bac),
        };
        let proj_flip = project_billboard(&bb_flip, &cam, 160, 144).unwrap();
        let proj_noflip = project_billboard(&bb_noflip, &cam, 160, 144).unwrap();

        // Both should have the same remap (flip_x does not affect palette_remap).
        assert_eq!(
            proj_flip.palette_remap, proj_noflip.palette_remap,
            "flip_x must not change the palette remap table"
        );
    }

    // -----------------------------------------------------------------------
    // Combat-feedback differentiation (hit vs stagger)
    // -----------------------------------------------------------------------

    use crate::mosquiton::{Mosquiton, MosquitonConfig, MosquitonState};
    use crate::spidey::{Spidey, SpideyConfig, SpideyState};
    use carcinisation_fps_core::enemy::DamageFlicker;

    /// A flicker advanced into its inverted phase (a fresh flicker starts in the
    /// non-inverted "regular" phase).
    fn inverting_flicker() -> DamageFlicker {
        let f = DamageFlicker::new()
            .tick(0.15)
            .expect("flicker still active");
        assert!(f.showing_invert(), "flicker must be in the invert phase");
        f
    }

    #[test]
    fn stagger_tint_dims_one_step_and_keeps_transparency() {
        // 0 transparent; opaque dims one step floored at 1: 3→2, 2→1, 1→1.
        let src = CxImage::new(vec![TRANSPARENT_INDEX, 1, 2, 3], 2);
        let out = make_stagger_tint_sprite(&src);
        assert_eq!(out.data(), &[TRANSPARENT_INDEX, 1, 1, 2]);
    }

    #[test]
    fn flash_sprite_selects_distinct_transforms() {
        let src = Arc::new(CxImage::new(vec![TRANSPARENT_INDEX, 1, 2, 3], 2));
        let none = flash_sprite(&src, EnemyFlash::None);
        let hit = flash_sprite(&src, EnemyFlash::Hit);
        let stagger = flash_sprite(&src, EnemyFlash::Stagger);
        assert_eq!(none.data(), src.data(), "None is the untouched sprite");
        assert_eq!(
            hit.data(),
            make_damage_invert_sprite(&src).data(),
            "Hit is the white damage invert"
        );
        assert_eq!(
            stagger.data(),
            make_stagger_tint_sprite(&src).data(),
            "Stagger is the dim tint"
        );
        // The three outcomes are visually distinct.
        assert_ne!(hit.data(), stagger.data(), "hit and stagger must differ");
        assert_ne!(none.data(), hit.data());
        assert_ne!(none.data(), stagger.data());
    }

    #[test]
    fn mosquiton_flash_priority_stagger_over_hit() {
        let mut m = Mosquiton::new(Vec2::ZERO, MosquitonConfig::default());
        // Both a stun and a hit flicker active → Stagger wins (a stun tick
        // usually coincides with a recent hit flicker).
        m.reaction.hit_stun_remaining = 0.5;
        m.damage_flicker = Some(inverting_flicker());
        assert_eq!(mosquiton_flash(&m), EnemyFlash::Stagger);

        // Hit flicker only → Hit.
        m.reaction.hit_stun_remaining = 0.0;
        assert_eq!(mosquiton_flash(&m), EnemyFlash::Hit);

        // Neither → None.
        m.damage_flicker = None;
        assert_eq!(mosquiton_flash(&m), EnemyFlash::None);

        // Dead enemies never flash, even with stun/flicker set (corpse guard).
        m.reaction.hit_stun_remaining = 0.5;
        m.damage_flicker = Some(inverting_flicker());
        m.state = MosquitonState::Dead;
        assert_eq!(mosquiton_flash(&m), EnemyFlash::None);
    }

    #[test]
    fn spidey_flash_priority_stagger_over_hit() {
        let mut s = Spidey::new(Vec2::ZERO, SpideyConfig::default());
        s.reaction.hit_stun_remaining = 0.5;
        s.damage_flicker = Some(inverting_flicker());
        assert_eq!(spidey_flash(&s), EnemyFlash::Stagger);

        s.reaction.hit_stun_remaining = 0.0;
        assert_eq!(spidey_flash(&s), EnemyFlash::Hit);

        s.damage_flicker = None;
        assert_eq!(spidey_flash(&s), EnemyFlash::None);

        s.reaction.hit_stun_remaining = 0.5;
        s.damage_flicker = Some(inverting_flicker());
        s.state = SpideyState::Dead;
        assert_eq!(spidey_flash(&s), EnemyFlash::None);
    }
}
