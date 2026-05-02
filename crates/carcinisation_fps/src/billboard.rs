//! Billboard sprite projection and rendering.

use bevy_math::Vec2;
use carapace::image::CxImage;
use carapace::palette::TRANSPARENT_INDEX;

use crate::camera::Camera;
use crate::enemy::{Enemy, EnemyState, Projectile, ProjectileImpact, ProjectileImpactKind};
use crate::mosquiton::{
    BloodShotBillboardSprites, Mosquiton, MosquitonBillboardSprites, MosquitonState,
};

const FP_DAMAGE_INVERT_MIN_COLOR_INDEX: u8 = 1;
const FP_DAMAGE_INVERT_MAX_COLOR_INDEX: u8 = 3;

/// A billboard entity in map space.
#[derive(Clone, Debug)]
pub struct Billboard {
    /// Position in map-space units.
    pub position: Vec2,
    /// Vertical offset from the view centre, in map-space units.
    pub height: f32,
    /// Rendered billboard height, in map-space units.
    pub world_height: f32,
    /// Billboard sprite (palette-indexed, single frame).
    pub sprite: CxImage,
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
    let det = plane.x * dir.y - dir.x * plane.y;
    if det.abs() < 1e-10 {
        return None;
    }
    let inv_det = 1.0 / det;
    // transform_x = lateral offset in camera plane
    // transform_y = depth (forward distance)
    let transform_x = inv_det * (dir.y * rel.x - dir.x * rel.y);
    let transform_y = inv_det * (-plane.y * rel.x + plane.x * rel.y);

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

    Some(ProjectedBillboard {
        screen_x: sx,
        distance: transform_y,
        screen_h: sprite_screen_h,
        screen_w: sprite_screen_w,
        vertical_shift,
        sprite: &billboard.sprite,
    })
}

/// Render a projected billboard into the image, respecting the per-column z-buffer.
/// Applies distance fog when `fog` is `Some((fog_color, fog_t))`.
pub(crate) fn draw_billboard(
    image: &mut CxImage,
    zbuffer: &[f32],
    proj: &ProjectedBillboard,
    screen_h: i32,
    fog: Option<(u8, f32)>,
) {
    let img_w = image.width() as i32;
    let half_h = screen_h / 2;

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

        let tex_x = ((x - draw_start_x) * tex_w / proj.screen_w).min(tex_w - 1);

        for y in draw_start_y.max(0)..draw_end_y.min(img_h) {
            let tex_y = ((y - draw_start_y) * tex_h / proj.screen_h).min(tex_h - 1);
            let pixel = sprite_data[(tex_y * tex_w + tex_x) as usize];
            if pixel != TRANSPARENT_INDEX {
                let final_pixel = if let Some((fog_color, fog_t)) = fog {
                    let fog_level = (fog_t * 16.0) as u8;
                    let threshold = crate::render::BAYER_4X4[(y & 3) as usize][(x & 3) as usize];
                    if fog_level > threshold {
                        fog_color
                    } else {
                        pixel
                    }
                } else {
                    pixel
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

fn enemy_presentation_sprite(enemy: &Enemy, alive: &CxImage, death: &CxImage) -> CxImage {
    match enemy.state {
        EnemyState::Dying { .. } => death.clone(),
        EnemyState::BurningCorpse { .. } => make_burning_corpse_sprite(alive),
        _ if enemy.showing_damage_invert() => make_damage_invert_sprite(alive),
        _ => alive.clone(),
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
    }
}

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
            }
        })
        .collect()
}

/// Build billboard list from active projectiles.
#[must_use]
pub fn billboards_from_projectiles(projectiles: &[Projectile], sprite: &CxImage) -> Vec<Billboard> {
    projectiles
        .iter()
        .filter(|p| p.alive)
        .map(|p| Billboard {
            position: p.position,
            height: 0.15,
            world_height: 0.3,
            sprite: sprite.clone(),
        })
        .collect()
}

/// Build billboard list from projectile impact effects.
#[must_use]
pub fn billboards_from_projectile_impacts(
    impacts: &[ProjectileImpact],
    sprites: &BloodShotBillboardSprites,
) -> Vec<Billboard> {
    impacts
        .iter()
        .map(|impact| {
            let sprite = match impact.kind {
                ProjectileImpactKind::Hit => sprites.hit.clone(),
                ProjectileImpactKind::Destroy => sprites.destroy_sprite_at(impact.age).clone(),
            };
            Billboard {
                position: impact.position,
                height: 0.15,
                world_height: match impact.kind {
                    ProjectileImpactKind::Hit => 0.42,
                    ProjectileImpactKind::Destroy => 0.36,
                },
                sprite,
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
        sprite: match mosquiton.state {
            MosquitonState::Dying { .. } => sprites.death.clone(),
            MosquitonState::BurningCorpse { .. } => {
                make_burning_corpse_sprite(sprites.alive_sprite_at(0.0))
            }
            MosquitonState::MeleeAttack { .. } => {
                let sprite = sprites.melee_sprite_at(mosquiton.animation_time);
                if mosquiton.showing_damage_invert() {
                    make_damage_invert_sprite(sprite)
                } else {
                    sprite.clone()
                }
            }
            _ => {
                let sprite = sprites.alive_sprite_at(mosquiton.animation_time);
                if mosquiton.showing_damage_invert() {
                    make_damage_invert_sprite(sprite)
                } else {
                    sprite.clone()
                }
            }
        },
    }
}

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
            sprite: match m.state {
                MosquitonState::Dying { .. } => sprites.death.clone(),
                MosquitonState::BurningCorpse { .. } => {
                    make_burning_corpse_sprite(sprites.alive_sprite_at(0.0))
                }
                MosquitonState::MeleeAttack { .. } => {
                    let sprite = sprites.melee_sprite_at(m.animation_time);
                    if m.showing_damage_invert() {
                        make_damage_invert_sprite(sprite)
                    } else {
                        sprite.clone()
                    }
                }
                _ => {
                    let sprite = sprites.alive_sprite_at(m.animation_time);
                    if m.showing_damage_invert() {
                        make_damage_invert_sprite(sprite)
                    } else {
                        sprite.clone()
                    }
                }
            },
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera::Camera;

    #[test]
    fn billboard_in_front_projects_near_center() {
        let cam = Camera::default(); // at (4,4), facing east
        let bb = Billboard {
            position: Vec2::new(6.0, 4.0), // directly ahead
            height: 0.0,
            world_height: 1.0,
            sprite: make_pillar_sprite(8, 16, 5),
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
    fn billboard_behind_camera_returns_none() {
        let cam = Camera::default(); // facing east
        let bb = Billboard {
            position: Vec2::new(2.0, 4.0), // behind
            height: 0.0,
            world_height: 1.0,
            sprite: make_pillar_sprite(8, 16, 5),
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
            sprite: make_pillar_sprite(8, 16, 5),
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
            sprite: make_pillar_sprite(8, 16, 5),
        };
        let right = Billboard {
            position: Vec2::new(6.0, 3.0), // south = screen-right
            height: 0.0,
            world_height: 1.0,
            sprite: make_pillar_sprite(8, 16, 5),
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
        enemy.damage_flicker = enemy.damage_flicker.and_then(|flicker| flicker.tick(0.2));
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
}
