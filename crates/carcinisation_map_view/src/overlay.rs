//! Entity markers and player arrow for the map view overlay.

use bevy::prelude::*;
use carapace::image::CxImage;
use carapace::prelude::*;
use carcinisation_fps::mosquiton::Mosquiton;
use carcinisation_fps::player_attack::{PlayerAttackSprites, PlayerAttackState};
use carcinisation_fps::plugin::{
    CameraRes, Config as FpsConfig, EnemySpriteIndex, Projectiles, SpritePairs,
};
use carcinisation_fps::plugin::{MosquitonSprites, SpideySprites};
use carcinisation_fps::spidey::Spidey;
use carcinisation_fps_core::enemy::Enemy;

use crate::MapViewConfig;
use crate::MapViewOverlayLayer;
use crate::MapViewToggle;

/// Palette index for the player marker fill (dark, high contrast against floor).
const PLAYER_COLOR: u8 = 1;
/// Palette index for the player marker outline (contrasting against fill).
const PLAYER_OUTLINE_COLOR: u8 = 4;
/// Palette index for enemy projectile dots on the map overlay.
const PROJECTILE_COLOR: u8 = 2;
/// Palette index for flame stream dots on the map overlay.
const FLAME_COLOR: u8 = 4;

/// Per-frame snapshot of one entity on the map view overlay.
pub struct MapViewEntityMarker {
    /// Pixel position (centre) in overlay image space.
    pub centre_x: i32,
    pub centre_y: i32,
    /// Pre-scaled, pre-rotated marker sprite. Palette index 0 = transparent.
    pub sprite: CxImage,
}

/// Per-frame dynamic overlay data — built once per frame.
#[derive(Resource, Default)]
pub struct MapViewOverlay {
    pub markers: Vec<MapViewEntityMarker>,
    /// Pixel dimensions of the overlay image (matches base map CxSprite).
    pub pixel_width: u32,
    pub pixel_height: u32,
    /// Grid dimensions in cells.
    pub grid_width: u32,
    pub grid_height: u32,
    /// Handle to the overlay sprite asset (mutated each frame).
    pub handle: Option<Handle<CxSpriteAsset>>,
}

/// Marker component for the enemy/entity overlay layer.
#[derive(Component)]
pub struct MapViewMarkerOverlay;

/// Marker component for the camera-anchored player arrow.
#[derive(Component)]
pub struct MapViewPlayerMarker;

// --- helper functions ---

/// Nearest-neighbour scale to `marker_size` × `marker_size`.
fn scale(source: &CxImage, marker_size: u32) -> CxImage {
    let sw = source.width() as u32;
    let sh = source.height() as u32;
    let ms = marker_size;
    let mut data = vec![0u8; (ms * ms) as usize];
    for ty in 0..ms {
        for tx in 0..ms {
            let sx = (tx * sw / ms).min(sw - 1) as usize;
            let sy = (ty * sh / ms).min(sh - 1) as usize;
            data[(ty * ms + tx) as usize] = source.data()[sy * source.width() + sx];
        }
    }
    CxImage::new(data, ms as usize)
}

/// Nearest-neighbour rotate by `angle` radians around centre.
fn rotate(source: &CxImage, angle: f32) -> CxImage {
    if angle.abs() < f32::EPSILON {
        return source.clone();
    }
    let w = source.width() as i32;
    let h = source.height() as i32;
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    let (sin_a, cos_a) = angle.sin_cos();
    let mut data = vec![0u8; source.data().len()];
    for dy in 0..h {
        for dx in 0..w {
            let rx = dx as f32 - cx;
            let ry = dy as f32 - cy;
            let sx = (rx * cos_a - ry * sin_a + cx).round() as i32;
            let sy = (rx * sin_a + ry * cos_a + cy).round() as i32;
            if sx >= 0 && sx < w && sy >= 0 && sy < h {
                data[(dy * w + dx) as usize] = source.data()[(sy * w + sx) as usize];
            }
        }
    }
    CxImage::new(data, w as usize)
}

/// Angle in radians from `from` toward `to`.
fn angle_toward(from: Vec2, to: Vec2) -> f32 {
    let d = to - from;
    f32::atan2(d.y, d.x)
}

/// Build a circle-with-nose sprite pointing east (→) for the player marker.
///
/// Filled circle + triangular nose with a 1 px contrasting outline.
fn player_marker_sprite(size: u32) -> CxImage {
    let s = size.max(5) as f32;
    let si = size.max(5) as i32;
    let mut data = vec![0u8; (si * si) as usize];
    let c = s / 2.0;
    let r = s * 0.35;
    let outline_width = 1.0_f32;

    for y in 0..si {
        for x in 0..si {
            let px = x as f32 + 0.5 - c;
            let py = y as f32 + 0.5 - c;
            let d = (px * px + py * py).sqrt();

            // Check if pixel is inside the circle body.
            if d <= r {
                let color = if d > r - outline_width {
                    PLAYER_OUTLINE_COLOR
                } else {
                    PLAYER_COLOR
                };
                data[(y * si + x) as usize] = color;
            } else if px > 0.0 {
                // Triangular nose extending east from the circle.
                let nose_end = c - 0.5;
                let t = (px - r) / (nose_end - r);
                if (0.0..=1.0).contains(&t) {
                    let half_width = r * 0.7 * (1.0 - t);
                    if py.abs() <= half_width {
                        // Outline: pixels near the nose edge.
                        let at_side_edge = py.abs() > half_width - outline_width;
                        let at_tip = t > 1.0 - outline_width / (nose_end - r).max(1.0);
                        let at_base = d > r + outline_width;
                        let color = if (at_side_edge || at_tip) && at_base {
                            PLAYER_OUTLINE_COLOR
                        } else if d > r {
                            // Transition zone near circle/nose junction — fill.
                            PLAYER_COLOR
                        } else {
                            PLAYER_COLOR
                        };
                        data[(y * si + x) as usize] = color;
                    }
                }
            }
        }
    }
    CxImage::new(data, si as usize)
}

/// Player marker is slightly larger than enemies for visual prominence.
fn player_marker_size(base: u32) -> u32 {
    (base + 2).max(5)
}

/// Enemy markers are 50% larger than the base marker size.
fn enemy_marker_size(base: u32) -> u32 {
    base + base / 2
}

/// Filled circle with a 1 px outline for projectile markers.
fn circle_sprite(size: u32, fill: u8, outline: u8) -> CxImage {
    let s = size.max(3) as f32;
    let si = size.max(3) as i32;
    let mut data = vec![0u8; (si * si) as usize];
    let c = s / 2.0;
    let r = c - 0.5;
    for y in 0..si {
        for x in 0..si {
            let dx = x as f32 + 0.5 - c;
            let dy = y as f32 + 0.5 - c;
            let d = (dx * dx + dy * dy).sqrt();
            if d <= r {
                data[(y * si + x) as usize] = if d > r - 1.0 { outline } else { fill };
            }
        }
    }
    CxImage::new(data, si as usize)
}

/// Convert map-cell coordinate to overlay pixel centre.
pub(crate) fn cell_to_pixel(coord: f32, tile_size: u32) -> i32 {
    (coord * tile_size as f32) as i32
}

/// Convert world Y to overlay image row in Y-flipped orientation.
///
/// The base map is rendered with `render_map_view` which flips Y so that
/// grid row 0 (south) appears at the bottom of the image. This function
/// applies the same continuous transform to marker positions and scroll
/// offsets — no integer truncation, so it scrolls smoothly across cell
/// boundaries.
pub(crate) fn flip_y(y: f32, tile_size: u32, grid_height: u32) -> i32 {
    ((grid_height as f32 - y) * tile_size as f32) as i32
}

// --- systems ---

/// Cached static marker sprites (projectiles).
///
/// Created once on first use, reused every frame. These never change because
/// `marker_size` is fixed at init.
#[derive(Default)]
pub struct CachedMarkerSprites {
    blood_circle: Option<CxImage>,
    web_circle: Option<CxImage>,
}

/// Build the per-frame entity marker snapshot.
///
/// Includes enemies, enemy projectiles, and active flame samples.
/// The player marker is a separate camera-anchored entity updated by
/// [`update_player_marker`].
#[allow(clippy::too_many_arguments)]
pub fn build_entity_snapshot(
    camera: Res<CameraRes>,
    sprite_pairs: Res<SpritePairs>,
    enemy_q: Query<(&Enemy, &EnemySpriteIndex)>,
    mosquiton_q: Query<&Mosquiton>,
    spidey_q: Query<&Spidey>,
    mosquiton_sprites: Res<MosquitonSprites>,
    spidey_sprites: Res<SpideySprites>,
    projectiles: Res<Projectiles>,
    attack_state: Res<PlayerAttackState>,
    attack_sprites: Res<PlayerAttackSprites>,
    time: Res<Time>,
    config: Res<MapViewConfig>,
    mut overlay: ResMut<MapViewOverlay>,
    mut cached: Local<CachedMarkerSprites>,
) {
    let ts = config.tile_size;
    let ms = config.marker_size;
    let ems = enemy_marker_size(ms);
    let gh = overlay.grid_height;
    overlay.markers.clear();
    let player_pos = camera.0.position;

    // Basic enemies (guards).
    for (enemy, idx) in enemy_q.iter() {
        if !enemy.is_alive() {
            continue;
        }
        let pair_idx = idx.0;
        let Some((alive, _death)) = sprite_pairs.0.get(pair_idx) else {
            continue;
        };
        let scaled = scale(alive, ems);
        let rotated = rotate(&scaled, angle_toward(enemy.position, player_pos));
        overlay.markers.push(MapViewEntityMarker {
            centre_x: cell_to_pixel(enemy.position.x, ts),
            centre_y: flip_y(enemy.position.y, ts, gh),
            sprite: rotated,
        });
    }

    // Mosquitons.
    for mosquiton in mosquiton_q.iter() {
        if !mosquiton.is_alive() {
            continue;
        }
        let elapsed = time.elapsed_secs();
        let sprite = mosquiton_sprites.0.alive_sprite_at(elapsed);
        let scaled = scale(sprite, ems);
        let rotated = rotate(&scaled, angle_toward(mosquiton.position, player_pos));
        overlay.markers.push(MapViewEntityMarker {
            centre_x: cell_to_pixel(mosquiton.position.x, ts),
            centre_y: flip_y(mosquiton.position.y, ts, gh),
            sprite: rotated,
        });
    }

    // Spideys.
    for spidey in spidey_q.iter() {
        if !spidey.is_alive() {
            continue;
        }
        let elapsed = time.elapsed_secs();
        let sprite = spidey_sprites.0.alive_sprite_at(elapsed);
        let scaled = scale(sprite, ems);
        let rotated = rotate(&scaled, angle_toward(spidey.position, player_pos));
        overlay.markers.push(MapViewEntityMarker {
            centre_x: cell_to_pixel(spidey.position.x, ts),
            centre_y: flip_y(spidey.position.y, ts, gh),
            sprite: rotated,
        });
    }

    // Ensure static marker sprites are cached (created once, reused every frame).
    let proj_size = (ms / 2).max(3);
    if cached.blood_circle.is_none() {
        cached.blood_circle = Some(circle_sprite(proj_size, PROJECTILE_COLOR, 1));
    }
    if cached.web_circle.is_none() {
        cached.web_circle = Some(circle_sprite(proj_size, FLAME_COLOR, 1));
    }
    let blood_circle = cached.blood_circle.as_ref().unwrap();
    let web_circle = cached.web_circle.as_ref().unwrap();

    // Enemy projectiles — circles distinguished by kind.
    for proj in &projectiles.0 {
        if !proj.alive {
            continue;
        }
        let sprite = match proj.kind {
            carcinisation_fps_core::enemy::ProjectileKind::BloodShot => blood_circle.clone(),
            carcinisation_fps_core::enemy::ProjectileKind::WebShot { .. } => web_circle.clone(),
        };
        overlay.markers.push(MapViewEntityMarker {
            centre_x: cell_to_pixel(proj.position.x, ts),
            centre_y: flip_y(proj.position.y, ts, gh),
            sprite,
        });
    }

    // Active flame samples — scaled-down actual flame sprites.
    let flame_size = (ms / 2).max(3);
    let elapsed = time.elapsed_secs();
    let flame_frame = attack_sprites.flame_frame_loop(elapsed);
    let scaled_flame = scale(flame_frame, flame_size);
    for pos in attack_state.flame_world_positions() {
        overlay.markers.push(MapViewEntityMarker {
            centre_x: cell_to_pixel(pos.x, ts),
            centre_y: flip_y(pos.y, ts, gh),
            sprite: scaled_flame.clone(),
        });
    }
}

/// Spawn the marker overlay CxSprite entity on its own layer.
///
/// Must run after the base map sprite is initialised (reads `MapRes`).
pub fn init_marker_overlay<L: CxLayer>(
    mut commands: Commands,
    map_res: Res<carcinisation_fps::plugin::MapRes>,
    mut sprite_assets: ResMut<Assets<CxSpriteAsset>>,
    mut overlay: ResMut<MapViewOverlay>,
    config: Res<MapViewConfig>,
    toggle: Res<MapViewToggle>,
    layer: Res<MapViewOverlayLayer<L>>,
) {
    let ts = config.tile_size;
    let w = (map_res.0.width as u32 * ts).max(1);
    let h = (map_res.0.height as u32 * ts).max(1);
    overlay.pixel_width = w;
    overlay.pixel_height = h;
    overlay.grid_width = map_res.0.width as u32;
    overlay.grid_height = map_res.0.height as u32;

    let image = CxImage::empty(UVec2::new(w, h));
    let asset = CxSpriteAsset::from_raw(image.data().to_vec(), image.width());
    let handle = sprite_assets.add(asset);
    overlay.handle = Some(handle.clone());

    let vis = if toggle.enabled {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };

    commands.spawn((
        CxSprite(handle),
        CxPosition(IVec2::ZERO),
        CxAnchor::BottomLeft,
        layer.0.clone(),
        CxRenderSpace::Camera,
        vis,
        MapViewMarkerOverlay,
    ));
}

/// Spawn the camera-anchored player marker at screen centre.
///
/// Always renders at the fixed screen midpoint — the map scrolls underneath.
/// Uses a larger sprite than enemy markers for visual prominence.
pub fn init_player_marker<L: CxLayer>(
    mut commands: Commands,
    mut sprite_assets: ResMut<Assets<CxSpriteAsset>>,
    config: Res<MapViewConfig>,
    fps_config: Res<FpsConfig>,
    toggle: Res<MapViewToggle>,
    layer: Res<MapViewOverlayLayer<L>>,
) {
    let pms = player_marker_size(config.marker_size);
    let sprite = player_marker_sprite(pms);
    let asset = CxSpriteAsset::from_raw(sprite.data().to_vec(), sprite.width());
    let handle = sprite_assets.add(asset);

    let cx = fps_config.screen_width as i32 / 2;
    let cy = fps_config.screen_height as i32 / 2;

    let vis = if toggle.enabled {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };

    commands.spawn((
        CxSprite(handle),
        CxPosition(IVec2::new(cx, cy)),
        CxAnchor::Center,
        layer.0.clone(),
        CxRenderSpace::Camera,
        vis,
        MapViewPlayerMarker,
    ));
}

/// Update the marker overlay each frame with markers from [`MapViewOverlay`].
pub fn update_marker_overlay(
    mut sprite_assets: ResMut<Assets<CxSpriteAsset>>,
    overlay: Res<MapViewOverlay>,
) {
    let Some(handle) = overlay.handle.as_ref() else {
        info!("update_marker_overlay: no handle");
        return;
    };
    let Some(asset) = sprite_assets.get_mut(handle) else {
        info!("update_marker_overlay: handle not resolved");
        return;
    };
    let w = asset.width();
    let h = asset.frame_height();
    let data = asset.data_mut();
    data.fill(0);
    for marker in &overlay.markers {
        let mw = marker.sprite.width() as i32;
        let mh = marker.sprite.height() as i32;
        let ox = marker.centre_x - mw / 2;
        let oy = marker.centre_y - mh / 2;
        for sy in 0..mh {
            for sx in 0..mw {
                let px = ox + sx;
                let py = oy + sy;
                if px >= 0 && px < w as i32 && py >= 0 && py < h as i32 {
                    let si = (sy * mw + sx) as usize;
                    let pi = marker.sprite.data().get(si).copied().unwrap_or(0);
                    if pi != 0 {
                        data[py as usize * w + px as usize] = pi;
                    }
                }
            }
        }
    }
}

/// Rotate the camera-anchored player marker to match the current facing angle.
///
/// The base (un-rotated) sprite is cached in a `Local` to avoid regenerating
/// it every frame — only the rotation changes.
pub fn update_player_marker(
    camera: Res<CameraRes>,
    config: Res<MapViewConfig>,
    marker_q: Query<&CxSprite, With<MapViewPlayerMarker>>,
    mut sprite_assets: ResMut<Assets<CxSpriteAsset>>,
    mut cached_base: Local<Option<CxImage>>,
) {
    let Ok(sprite) = marker_q.single() else {
        return;
    };
    let Some(asset) = sprite_assets.get_mut(&sprite.0) else {
        return;
    };
    let pms = player_marker_size(config.marker_size);
    let base = cached_base.get_or_insert_with(|| player_marker_sprite(pms));
    let rotated = rotate(base, camera.0.angle);
    let data = asset.data_mut();
    data.copy_from_slice(rotated.data());
}
