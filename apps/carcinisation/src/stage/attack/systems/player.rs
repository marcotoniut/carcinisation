use bevy::asset::{AssetEvent, AssetId};
use bevy::prelude::*;
use bevy::reflect::{Reflect, ReflectRef};
use seldom_pixel::prelude::*;
use std::{collections::HashMap, sync::Arc};

use crate::{
    game::score::components::Score,
    stage::{
        attack::components::*,
        components::interactive::{ColliderData, Hittable},
        enemy::components::Enemy,
        messages::DamageMessage,
        player::components::{
            PlayerAttack, UnhittableList, Weapon, ATTACK_GUN_DAMAGE, ATTACK_PINCER_DAMAGE,
        },
    },
};
use colored::*;

const CRITICAL_THRESHOLD: f32 = 0.5;
// Ordered 4x4 dithering threshold map used by PxFrameTransition::Dither.
const DITHERING: [u16; 16] = [
    0b0000_0000_0000_0000,
    0b1000_0000_0000_0000,
    0b1000_0000_0010_0000,
    0b1010_0000_0010_0000,
    0b1010_0000_1010_0000,
    0b1010_0100_1010_0000,
    0b1010_0100_1010_0001,
    0b1010_0101_1010_0001,
    0b1010_0101_1010_0101,
    0b1110_0101_1010_0101,
    0b1110_0101_1011_0101,
    0b1111_0101_1011_0101,
    0b1111_0101_1111_0101,
    0b1111_1101_1111_0101,
    0b1111_1101_1111_0111,
    0b1111_1111_1111_0111,
];

#[derive(Default)]
pub(crate) struct PixelCollisionCache {
    sprites: HashMap<AssetId<PxSpriteAsset>, Arc<SpritePixelData>>,
}

struct SpritePixelData {
    width: u32,
    height: u32,
    frame_count: usize,
    pixels: Vec<u8>,
    segments_per_row: usize,
    // Row-major u64 bitmasks for fast pixel overlap.
    row_masks: Vec<u64>,
}

impl SpritePixelData {
    fn from_asset(asset: &PxSpriteAsset) -> Option<Self> {
        // PxSpriteAsset hides pixel buffers; use reflection to build a collision snapshot.
        let ReflectRef::Struct(sprite_struct) = (asset as &dyn Reflect).reflect_ref() else {
            return None;
        };
        let frame_size = sprite_struct
            .field("frame_size")
            .and_then(|value| value.try_downcast_ref::<usize>().map(|value| *value))?;
        let data = sprite_struct.field("data")?;
        let ReflectRef::Struct(image_struct) = data.reflect_ref() else {
            return None;
        };
        let width = image_struct
            .field("width")
            .and_then(|value| value.try_downcast_ref::<usize>().map(|value| *value))?;
        let pixels = image_struct
            .field("image")
            .and_then(|value| value.try_downcast_ref::<Vec<u8>>())?;

        if width == 0 || frame_size == 0 || frame_size % width != 0 {
            return None;
        }

        let height = frame_size / width;
        if height == 0 {
            return None;
        }

        let frame_count = pixels.len() / frame_size;
        if frame_count == 0 {
            return None;
        }

        let segments_per_row = (width + 63) / 64;
        let mut row_masks = vec![0u64; frame_count * height * segments_per_row];
        for frame in 0..frame_count {
            for row in 0..height {
                for x in 0..width {
                    let index = (frame * height + row) * width + x;
                    if pixels[index] == 0 {
                        continue;
                    }
                    let segment = x / 64;
                    let bit = x % 64;
                    let offset = (frame * height + row) * segments_per_row + segment;
                    row_masks[offset] |= 1u64 << bit;
                }
            }
        }

        Some(Self {
            width: width as u32,
            height: height as u32,
            frame_count,
            pixels: pixels.clone(),
            segments_per_row,
            row_masks,
        })
    }

    fn frame_size(&self) -> UVec2 {
        UVec2::new(self.width, self.height)
    }

    fn row_mask(&self, frame: usize, row: u32) -> &[u64] {
        let row = row as usize;
        let offset = (frame * self.height as usize + row) * self.segments_per_row;
        &self.row_masks[offset..offset + self.segments_per_row]
    }
}

/**
 * Could split between box and circle collider
 */
pub fn check_got_hit(
    camera: Res<PxCamera>,
    sprite_assets: Res<Assets<PxSpriteAsset>>,
    mut asset_events: MessageReader<AssetEvent<PxSpriteAsset>>,
    mut event_writer: MessageWriter<DamageMessage>,
    mut attack_query: Query<(
        &PlayerAttack,
        &PxPosition,
        &PxAnchor,
        &PxCanvas,
        Option<&PxFrameView>,
        &PxSprite,
        &mut UnhittableList,
    )>,
    // mut attack_query: Query<(&PlayerAttack, &mut UnhittableList, Option<&Reach>)>,
    mut hittable_query: Query<
        (
            Entity,
            &PxPosition,
            &PxSubPosition,
            &PxAnchor,
            &PxCanvas,
            Option<&PxFrameView>,
            Option<&PxSprite>,
            Option<&ColliderData>,
            Option<&Enemy>,
            Option<&crate::stage::destructible::components::Destructible>,
        ),
        With<Hittable>,
    >,
    mut score: ResMut<Score>,
    mut cache: Local<PixelCollisionCache>,
) {
    if asset_events.read().next().is_some() {
        cache.sprites.clear();
    }

    for (
        attack,
        attack_position,
        attack_anchor,
        attack_canvas,
        attack_frame,
        attack_sprite,
        mut hit_list,
    ) in attack_query.iter_mut()
    {
        let attack_data = sprite_data(&mut cache, &sprite_assets, &**attack_sprite);
        let attack_rect = attack_data.as_deref().map(|data| {
            sprite_rect(
                data.frame_size(),
                *attack_position,
                *attack_anchor,
                *attack_canvas,
                **camera,
            )
        });

        let attack_world = match *attack_canvas {
            PxCanvas::World => attack_position.0,
            PxCanvas::Camera => attack_position.0 + **camera,
        };
        let attack_world = attack_world.as_vec2();

        for (
            entity,
            entity_position,
            entity_sub_position,
            entity_anchor,
            entity_canvas,
            entity_frame,
            entity_sprite,
            collider_data,
            enemy,
            destructible,
        ) in hittable_query.iter_mut()
        {
            if hit_list.0.contains(&entity) {
                continue;
            }

            let mut hit = None;
            let mut evaluated = false;
            let wants_pixel = destructible.is_none() && entity_sprite.is_some();
            if wants_pixel {
                // TODO: allow opting into a dedicated collision sprite/mask component.
                if let (Some(attack_data), Some(attack_rect), Some(entity_sprite)) =
                    (attack_data.as_deref(), attack_rect, entity_sprite)
                {
                    if let Some(entity_data) =
                        sprite_data(&mut cache, &sprite_assets, &**entity_sprite)
                    {
                        evaluated = true;
                        let entity_rect = sprite_rect(
                            entity_data.frame_size(),
                            *entity_position,
                            *entity_anchor,
                            *entity_canvas,
                            **camera,
                        );

                        hit = pixel_overlap(
                            attack_data,
                            attack_frame.copied(),
                            attack_rect,
                            entity_data.as_ref(),
                            entity_frame.copied(),
                            entity_rect,
                        )
                        .map(|screen_pos| match *entity_canvas {
                            PxCanvas::World => (screen_pos + **camera).as_vec2(),
                            PxCanvas::Camera => screen_pos.as_vec2(),
                        });
                    }
                }
            }

            if hit.is_none() {
                if let Some(collider_data) = collider_data {
                    if collider_data
                        .point_collides(entity_sub_position.0, attack_world)
                        .is_some()
                    {
                        hit = Some(attack_world);
                    }
                    evaluated = true;
                }
            }

            if !evaluated && enemy.is_some() {
                // If we couldn't evaluate pixel data yet, fall back to collider checks for enemies.
                if let Some(collider_data) = collider_data {
                    if collider_data
                        .point_collides(entity_sub_position.0, attack_world)
                        .is_some()
                    {
                        hit = Some(attack_world);
                    }
                    evaluated = true;
                }
            }

            if !evaluated {
                continue;
            }

            let Some(hit_position) = hit else {
                hit_list.0.insert(entity);
                continue;
            };

            let defense = collider_data
                .and_then(|data| data.point_collides(entity_sub_position.0, hit_position))
                .map(|value| value.defense)
                .unwrap_or(1.0);

            hit_list.0.insert(entity);
            match attack.weapon {
                Weapon::Pincer => {
                    event_writer.write(DamageMessage::new(
                        entity,
                        (ATTACK_PINCER_DAMAGE as f32 / defense) as u32,
                    ));
                    if defense <= CRITICAL_THRESHOLD {
                        score.add_u(SCORE_MELEE_CRITICAL_HIT);

                        #[cfg(debug_assertions)]
                        println!("{} Pincer ***CRITICAL***", "HIT".yellow());
                    } else {
                        score.add_u(SCORE_MELEE_REGULAR_HIT);

                        #[cfg(debug_assertions)]
                        println!("{} Pincer", "HIT".yellow());
                    }
                }
                Weapon::Gun => {
                    event_writer.write(DamageMessage::new(
                        entity,
                        (ATTACK_GUN_DAMAGE as f32 / defense) as u32,
                    ));
                    if defense <= CRITICAL_THRESHOLD {
                        score.add_u(SCORE_RANGED_CRITICAL_HIT);

                        #[cfg(debug_assertions)]
                        println!("{} Gun ***CRITICAL***", "HIT".yellow());
                    } else {
                        score.add_u(SCORE_RANGED_REGULAR_HIT);

                        #[cfg(debug_assertions)]
                        println!("{} Gun", "HIT".yellow());
                    }
                }
            }
        }
    }
}

fn sprite_data(
    cache: &mut PixelCollisionCache,
    assets: &Assets<PxSpriteAsset>,
    handle: &Handle<PxSpriteAsset>,
) -> Option<Arc<SpritePixelData>> {
    let id = handle.id();
    if !cache.sprites.contains_key(&id) {
        let asset = assets.get(handle)?;
        let data = SpritePixelData::from_asset(asset)?;
        cache.sprites.insert(id, Arc::new(data));
    }
    cache.sprites.get(&id).cloned()
}

fn sprite_rect(
    size: UVec2,
    position: PxPosition,
    anchor: PxAnchor,
    canvas: PxCanvas,
    camera: IVec2,
) -> IRect {
    let position = *position - anchor_offset(anchor, size).as_ivec2();
    let position = match canvas {
        PxCanvas::World => position - camera,
        PxCanvas::Camera => position,
    };

    IRect {
        min: position,
        max: position.saturating_add(size.as_ivec2()),
    }
}

fn pixel_overlap(
    attack_data: &SpritePixelData,
    attack_frame: Option<PxFrameView>,
    attack_rect: IRect,
    enemy_data: &SpritePixelData,
    enemy_frame: Option<PxFrameView>,
    enemy_rect: IRect,
) -> Option<IVec2> {
    let attack_dither = attack_frame
        .as_ref()
        .is_some_and(|frame| matches!(frame.transition, PxFrameTransition::Dither));
    let enemy_dither = enemy_frame
        .as_ref()
        .is_some_and(|frame| matches!(frame.transition, PxFrameTransition::Dither));
    if !attack_dither && !enemy_dither {
        if let (Some(attack_index), Some(enemy_index)) = (
            frame_index_for_static(attack_frame, attack_data.frame_count),
            frame_index_for_static(enemy_frame, enemy_data.frame_count),
        ) {
            return pixel_overlap_fast(
                attack_data,
                attack_index,
                attack_rect,
                enemy_data,
                enemy_index,
                enemy_rect,
            );
        }
    }

    pixel_overlap_slow(
        attack_data,
        attack_frame,
        attack_rect,
        enemy_data,
        enemy_frame,
        enemy_rect,
    )
}

fn pixel_overlap_fast(
    attack_data: &SpritePixelData,
    attack_frame: usize,
    attack_rect: IRect,
    enemy_data: &SpritePixelData,
    enemy_frame: usize,
    enemy_rect: IRect,
) -> Option<IVec2> {
    let min = IVec2::new(
        attack_rect.min.x.max(enemy_rect.min.x),
        attack_rect.min.y.max(enemy_rect.min.y),
    );
    let max = IVec2::new(
        attack_rect.max.x.min(enemy_rect.max.x),
        attack_rect.max.y.min(enemy_rect.max.y),
    );

    if min.x >= max.x || min.y >= max.y {
        return None;
    }

    let delta_x = enemy_rect.min.x - attack_rect.min.x;
    let overlap_min_x = (min.x - attack_rect.min.x) as u32;
    let overlap_max_x = (max.x - attack_rect.min.x) as u32;
    let start_word = (overlap_min_x / 64) as usize;
    let end_word = ((overlap_max_x - 1) / 64) as usize;

    for y in min.y..max.y {
        let attack_local_y = (y - attack_rect.min.y) as u32;
        let enemy_local_y = (y - enemy_rect.min.y) as u32;
        let attack_y = attack_data
            .height
            .saturating_sub(1)
            .saturating_sub(attack_local_y);
        let enemy_y = enemy_data
            .height
            .saturating_sub(1)
            .saturating_sub(enemy_local_y);

        let attack_row = attack_data.row_mask(attack_frame, attack_y);
        let enemy_row = enemy_data.row_mask(enemy_frame, enemy_y);

        for word in start_word..=end_word {
            let mut mask = !0u64;
            if word == start_word {
                let start_bit = overlap_min_x % 64;
                mask &= !0u64 << start_bit;
            }
            if word == end_word {
                let end_bit = overlap_max_x % 64;
                if end_bit != 0 {
                    mask &= (1u64 << end_bit) - 1;
                }
            }

            let attack_word = attack_row.get(word).copied().unwrap_or(0) & mask;
            if attack_word == 0 {
                continue;
            }

            let enemy_word = shifted_row_word(enemy_row, delta_x, word) & mask;
            let overlap = attack_word & enemy_word;
            if overlap != 0 {
                let bit = overlap.trailing_zeros() as i32;
                let screen_x = attack_rect.min.x + (word as i32 * 64) + bit;
                return Some(IVec2::new(screen_x, y));
            }
        }
    }

    None
}

fn shifted_row_word(row: &[u64], shift: i32, word_index: usize) -> u64 {
    if shift == 0 {
        return row.get(word_index).copied().unwrap_or(0);
    }

    if shift > 0 {
        let shift = shift as u32;
        let word_shift = (shift / 64) as usize;
        let bit_shift = shift % 64;
        let src = match word_index.checked_sub(word_shift) {
            Some(index) => index,
            None => return 0,
        };
        let low = row.get(src).copied().unwrap_or(0);
        if bit_shift == 0 {
            return low;
        }
        let high = if src == 0 {
            0
        } else {
            row.get(src - 1).copied().unwrap_or(0)
        };
        (low << bit_shift) | (high >> (64 - bit_shift))
    } else {
        let shift = (-shift) as u32;
        let word_shift = (shift / 64) as usize;
        let bit_shift = shift % 64;
        let src = word_index + word_shift;
        let low = row.get(src).copied().unwrap_or(0);
        if bit_shift == 0 {
            return low;
        }
        let high = row.get(src + 1).copied().unwrap_or(0);
        (low >> bit_shift) | (high << (64 - bit_shift))
    }
}

fn pixel_overlap_slow(
    attack_data: &SpritePixelData,
    attack_frame: Option<PxFrameView>,
    attack_rect: IRect,
    enemy_data: &SpritePixelData,
    enemy_frame: Option<PxFrameView>,
    enemy_rect: IRect,
) -> Option<IVec2> {
    let min = IVec2::new(
        attack_rect.min.x.max(enemy_rect.min.x),
        attack_rect.min.y.max(enemy_rect.min.y),
    );
    let max = IVec2::new(
        attack_rect.max.x.min(enemy_rect.max.x),
        attack_rect.max.y.min(enemy_rect.max.y),
    );

    if min.x >= max.x || min.y >= max.y {
        return None;
    }

    for y in min.y..max.y {
        let attack_local_y = (y - attack_rect.min.y) as u32;
        let enemy_local_y = (y - enemy_rect.min.y) as u32;
        let attack_y = attack_data
            .height
            .saturating_sub(1)
            .saturating_sub(attack_local_y);
        let enemy_y = enemy_data
            .height
            .saturating_sub(1)
            .saturating_sub(enemy_local_y);

        for x in min.x..max.x {
            let attack_local_x = (x - attack_rect.min.x) as u32;
            let enemy_local_x = (x - enemy_rect.min.x) as u32;
            let attack_pos = UVec2::new(attack_local_x, attack_y);
            let enemy_pos = UVec2::new(enemy_local_x, enemy_y);

            if sprite_pixel_visible(attack_data, attack_frame, attack_pos)
                && sprite_pixel_visible(enemy_data, enemy_frame, enemy_pos)
            {
                return Some(IVec2::new(x, y));
            }
        }
    }

    None
}

fn sprite_pixel_visible(
    sprite: &SpritePixelData,
    frame: Option<PxFrameView>,
    local_pos: UVec2,
) -> bool {
    if sprite.width == 0 || sprite.height == 0 {
        return false;
    }
    if local_pos.x >= sprite.width || local_pos.y >= sprite.height {
        return false;
    }

    let frame_count = sprite.frame_count;
    if frame_count == 0 {
        return false;
    }

    let frame_index = frame_index_for_pos(frame, frame_count, local_pos);
    let pixel_y = frame_index as u32 * sprite.height + local_pos.y;
    let index = pixel_y as usize * sprite.width as usize + local_pos.x as usize;
    sprite.pixels.get(index).is_some_and(|pixel| *pixel != 0)
}

fn frame_index_for_static(frame: Option<PxFrameView>, frame_count: usize) -> Option<usize> {
    if frame_count == 0 {
        return None;
    }

    let Some(frame) = frame else {
        return Some(0);
    };

    let index = match frame.selector {
        PxFrameSelector::Normalized(value) => {
            if frame_count <= 1 {
                0.
            } else {
                value * (frame_count - 1) as f32
            }
        }
        PxFrameSelector::Index(value) => value,
    };

    Some(index.floor() as usize % frame_count)
}

fn frame_index_for_pos(frame: Option<PxFrameView>, frame_count: usize, pos: UVec2) -> usize {
    let Some(frame) = frame else {
        return 0;
    };

    if frame_count == 0 {
        return 0;
    }

    let index = match frame.selector {
        PxFrameSelector::Normalized(value) => {
            if frame_count <= 1 {
                0.
            } else {
                value * (frame_count - 1) as f32
            }
        }
        PxFrameSelector::Index(value) => value,
    };

    let dithering = match frame.transition {
        PxFrameTransition::Dither => DITHERING[(index.fract() * 16.) as usize % 16],
        PxFrameTransition::None => 0,
    };
    let base = index.floor() as usize;
    let bit = 0b1000_0000_0000_0000u16 >> (pos.x % 4 + pos.y % 4 * 4);
    let offset = (bit & dithering != 0) as usize;

    (base + offset) % frame_count
}

fn anchor_offset(anchor: PxAnchor, size: UVec2) -> UVec2 {
    let x = match anchor {
        PxAnchor::BottomLeft | PxAnchor::CenterLeft | PxAnchor::TopLeft => 0,
        PxAnchor::BottomCenter | PxAnchor::Center | PxAnchor::TopCenter => size.x / 2,
        PxAnchor::BottomRight | PxAnchor::CenterRight | PxAnchor::TopRight => size.x,
        PxAnchor::Custom(value) => (size.x as f32 * value.x) as u32,
    };
    let y = match anchor {
        PxAnchor::BottomLeft | PxAnchor::BottomCenter | PxAnchor::BottomRight => 0,
        PxAnchor::CenterLeft | PxAnchor::Center | PxAnchor::CenterRight => size.y / 2,
        PxAnchor::TopLeft | PxAnchor::TopCenter | PxAnchor::TopRight => size.y,
        PxAnchor::Custom(value) => (size.y as f32 * value.y) as u32,
    };
    UVec2::new(x, y)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_mask(
        width: u32,
        height: u32,
        frames: usize,
        on: &[(u32, u32, usize)],
    ) -> SpritePixelData {
        let mut pixels = vec![0u8; width as usize * height as usize * frames];
        for (x, y, frame) in on {
            let flipped_y = height.saturating_sub(1).saturating_sub(*y) as usize;
            let index = (frame * height as usize + flipped_y) * width as usize + *x as usize;
            pixels[index] = 1;
        }
        let segments_per_row = ((width + 63) / 64) as usize;
        let mut row_masks = vec![0u64; frames * height as usize * segments_per_row];
        for frame in 0..frames {
            for row in 0..height {
                for x in 0..width {
                    let index =
                        (frame * height as usize + row as usize) * width as usize + x as usize;
                    if pixels[index] == 0 {
                        continue;
                    }
                    let segment = (x / 64) as usize;
                    let bit = x % 64;
                    let offset =
                        (frame * height as usize + row as usize) * segments_per_row + segment;
                    row_masks[offset] |= 1u64 << bit;
                }
            }
        }
        SpritePixelData {
            width,
            height,
            frame_count: frames,
            pixels,
            segments_per_row,
            row_masks,
        }
    }

    fn rect_for_mask(min: IVec2, mask: &SpritePixelData) -> IRect {
        IRect {
            min,
            max: min + mask.frame_size().as_ivec2(),
        }
    }

    fn mask_overlaps_box(
        mask: &SpritePixelData,
        frame: Option<PxFrameView>,
        mask_rect: IRect,
        box_center: Vec2,
        half: Vec2,
    ) -> bool {
        let box_min = box_center - half;
        let box_max = box_center + half;
        let min = IVec2::new(
            mask_rect.min.x.max(box_min.x.floor() as i32),
            mask_rect.min.y.max(box_min.y.floor() as i32),
        );
        let max = IVec2::new(
            mask_rect.max.x.min(box_max.x.ceil() as i32),
            mask_rect.max.y.min(box_max.y.ceil() as i32),
        );

        for y in min.y..max.y {
            let local_y = (y - mask_rect.min.y) as u32;
            let sprite_y = mask.height.saturating_sub(1).saturating_sub(local_y);
            for x in min.x..max.x {
                let local_x = (x - mask_rect.min.x) as u32;
                let local = UVec2::new(local_x, sprite_y);
                if !sprite_pixel_visible(mask, frame, local) {
                    continue;
                }

                let point = Vec2::new(x as f32, y as f32);
                let delta = (point - box_center).abs();
                if delta.x <= half.x && delta.y <= half.y {
                    return true;
                }
            }
        }

        false
    }

    fn mask_overlaps_circle(
        mask: &SpritePixelData,
        frame: Option<PxFrameView>,
        mask_rect: IRect,
        center: Vec2,
        radius: f32,
    ) -> bool {
        let min = IVec2::new(
            mask_rect.min.x.max((center.x - radius).floor() as i32),
            mask_rect.min.y.max((center.y - radius).floor() as i32),
        );
        let max = IVec2::new(
            mask_rect.max.x.min((center.x + radius).ceil() as i32),
            mask_rect.max.y.min((center.y + radius).ceil() as i32),
        );
        let radius_sq = radius * radius;

        for y in min.y..max.y {
            let local_y = (y - mask_rect.min.y) as u32;
            let sprite_y = mask.height.saturating_sub(1).saturating_sub(local_y);
            for x in min.x..max.x {
                let local_x = (x - mask_rect.min.x) as u32;
                let local = UVec2::new(local_x, sprite_y);
                if !sprite_pixel_visible(mask, frame, local) {
                    continue;
                }

                let point = Vec2::new(x as f32, y as f32);
                if point.distance_squared(center) <= radius_sq {
                    return true;
                }
            }
        }

        false
    }

    #[test]
    fn pixel_mask_overlaps_pixel_mask() {
        let attack = make_mask(3, 3, 1, &[(2, 1, 0)]);
        let enemy = make_mask(3, 3, 1, &[(0, 1, 0)]);
        let attack_rect = rect_for_mask(IVec2::new(0, 0), &attack);
        let enemy_rect = rect_for_mask(IVec2::new(2, 0), &enemy);

        let hit = pixel_overlap(&attack, None, attack_rect, &enemy, None, enemy_rect);
        assert_eq!(hit, Some(IVec2::new(2, 1)));
    }

    #[test]
    fn pixel_mask_does_not_overlap_pixel_mask() {
        let attack = make_mask(2, 2, 1, &[(0, 0, 0)]);
        let enemy = make_mask(2, 2, 1, &[(1, 1, 0)]);
        let attack_rect = rect_for_mask(IVec2::new(0, 0), &attack);
        let enemy_rect = rect_for_mask(IVec2::new(3, 0), &enemy);

        let hit = pixel_overlap(&attack, None, attack_rect, &enemy, None, enemy_rect);
        assert!(hit.is_none());
    }

    #[test]
    fn box_overlaps_pixel_mask() {
        let mask = make_mask(3, 3, 1, &[(1, 1, 0)]);
        let rect = rect_for_mask(IVec2::new(0, 0), &mask);

        assert!(mask_overlaps_box(
            &mask,
            None,
            rect,
            Vec2::new(1.0, 1.0),
            Vec2::new(0.6, 0.6)
        ));
        assert!(!mask_overlaps_box(
            &mask,
            None,
            rect,
            Vec2::new(4.0, 4.0),
            Vec2::new(0.6, 0.6)
        ));
    }

    #[test]
    fn circle_overlaps_pixel_mask() {
        let mask = make_mask(3, 3, 1, &[(0, 0, 0)]);
        let rect = rect_for_mask(IVec2::new(0, 0), &mask);

        assert!(mask_overlaps_circle(
            &mask,
            None,
            rect,
            Vec2::new(0.0, 0.0),
            0.5
        ));
        assert!(!mask_overlaps_circle(
            &mask,
            None,
            rect,
            Vec2::new(2.0, 2.0),
            0.5
        ));
    }
}
