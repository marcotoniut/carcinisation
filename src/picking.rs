use std::collections::BTreeMap;

use bevy_picking::backend::prelude::*;

use crate::{
    cursor::PxCursorPosition,
    frame::{Frames, PxFrameControl, PxFrameView, animate, resolve_frame_binding},
    math::RectExt,
    position::Spatial,
    prelude::*,
    screen::Screen,
    set::PxSet,
    sprite::PxSpriteAsset,
};

/// Enable pixel-perfect picking for a sprite.
#[derive(Component, Default, Clone, Copy, Debug)]
pub struct PxPixelPick;

pub(crate) fn plug<L: PxLayer>(app: &mut App) {
    app.add_systems(PostUpdate, pick::<L>.in_set(PxSet::Picking));
}

fn layer_depth<L: PxLayer>(layer_depths: &mut BTreeMap<L, f32>, layer: &L) -> f32 {
    if let Some(&depth) = layer_depths.get(layer) {
        return depth;
    }

    let depth = match (
        layer_depths.range(..layer).last(),
        layer_depths.range(layer..).next(),
    ) {
        (Some((_, &lower)), Some((_, &upper))) => f32::midpoint(lower, upper),
        (Some((_, &lower)), None) => lower - 1.,
        (None, Some((_, &upper))) => upper + 1.,
        (None, None) => 0.,
    };

    // R-A workaround
    BTreeMap::insert(layer_depths, layer.clone(), depth);
    depth
}

fn spatial_rect(
    size: UVec2,
    position: PxPosition,
    anchor: PxAnchor,
    canvas: PxCanvas,
    cam_pos: IVec2,
) -> IRect {
    let position = *position - anchor.pos(size).as_ivec2();
    let position = match canvas {
        PxCanvas::World => position - cam_pos,
        PxCanvas::Camera => position,
    };

    IRect {
        min: position,
        max: position.saturating_add(size.as_ivec2()),
    }
}

fn sprite_pixel_visible<'a>(
    sprite: &PxSpriteAsset,
    frame: Option<PxFrameView>,
    filters: impl IntoIterator<Item = &'a PxFilterAsset>,
    local_pos: UVec2,
) -> bool {
    let size = sprite.frame_size();
    if size.x == 0 || size.y == 0 {
        return false;
    }
    if local_pos.x >= size.x || local_pos.y >= size.y {
        return false;
    }

    let frame_count = sprite.frame_count();
    if frame_count == 0 {
        return false;
    }

    let frame_index = match frame {
        Some(frame) => animate(frame, frame_count)(local_pos),
        None => 0,
    };

    let pixel_y = frame_index as i32 * size.y as i32 + local_pos.y as i32;
    let mut pixel = sprite.data.pixel(IVec2::new(local_pos.x as i32, pixel_y));
    if pixel == 0 {
        return false;
    }

    for filter in filters {
        pixel = filter.as_fn()(pixel);
        if pixel == 0 {
            return false;
        }
    }

    true
}

// TODO Pick other entities in a generic way
// TODO Other pointers support
fn pick<L: PxLayer>(
    mut hits: MessageWriter<PointerHits>,
    pointers: Query<&PointerId>,
    rects: Query<(
        &PxRect,
        &PxFilterLayers<L>,
        &PxPosition,
        &PxAnchor,
        &PxCanvas,
        &InheritedVisibility,
        Entity,
    )>,
    sprites: Query<
        (
            &PxSprite,
            &PxPosition,
            &PxAnchor,
            &L,
            &PxCanvas,
            Option<&PxFrameView>,
            Option<&PxFrameControl>,
            Option<&PxFilter>,
            &InheritedVisibility,
            Entity,
        ),
        With<PxPixelPick>,
    >,
    composites: Query<
        (
            &PxCompositeSprite,
            &PxPosition,
            &PxAnchor,
            &L,
            &PxCanvas,
            Option<&PxFrameView>,
            Option<&PxFrameControl>,
            Option<&PxFilter>,
            &InheritedVisibility,
            Entity,
        ),
        With<PxPixelPick>,
    >,
    sprite_assets: Res<Assets<PxSpriteAsset>>,
    filters: Res<Assets<PxFilterAsset>>,
    cursor: Res<PxCursorPosition>,
    px_camera: Res<PxCamera>,
    screen: Res<Screen>,
    cameras: Query<(&Camera, Entity)>,
) {
    // Note: PxPixelPick enables per-pixel picking; rects remain rectangle-based.
    let Some(cursor) = **cursor else {
        return;
    };
    let cursor = cursor.as_ivec2();

    let Ok((camera, camera_id)) = cameras.single() else {
        return;
    };

    let cam_pos = **px_camera;
    if screen.computed_size.y == 0 || screen.computed_size.x == 0 {
        return;
    }
    for &pointer in &pointers {
        let PointerId::Mouse = pointer else {
            continue;
        };

        let mut layer_depths = BTreeMap::new();
        let mut picks = Vec::new();

        for (&rect, layer, &pos, &anchor, canvas, visibility, id) in &rects {
            if !visibility.get() {
                continue;
            }

            let layer = match layer {
                PxFilterLayers::Single { layer, .. } => Some(layer),
                PxFilterLayers::Many(layers) => layers.iter().max(),
                // TODO Can't pick rects with this variant
                PxFilterLayers::Range(range) => Some(range.end()),
            };
            let Some(layer) = layer else {
                continue;
            };

            let depth = layer_depth(&mut layer_depths, layer);
            let rect = spatial_rect(*rect, pos, anchor, *canvas, cam_pos);

            if rect.contains_exclusive(cursor) {
                picks.push((
                    id,
                    HitData {
                        camera: camera_id,
                        depth,
                        position: None,
                        normal: None,
                    },
                ));
            }
        }

        for (
            sprite,
            &pos,
            &anchor,
            layer,
            &canvas,
            frame_view,
            frame_control,
            filter,
            visibility,
            id,
        ) in &sprites
        {
            if !visibility.get() {
                continue;
            }

            let Some(sprite) = sprite_assets.get(&**sprite) else {
                continue;
            };

            let size = sprite.frame_size();
            if size.x == 0 || size.y == 0 {
                continue;
            }

            let rect = spatial_rect(size, pos, anchor, canvas, cam_pos);

            if !rect.contains_exclusive(cursor) {
                continue;
            }

            let depth = layer_depth(&mut layer_depths, layer);

            let frame = frame_view
                .copied()
                .or_else(|| frame_control.copied().map(PxFrameView::from));

            let local = cursor - rect.min;
            let local_x = local.x as u32;
            let local_y = local.y as u32;
            let local_y = size.y.saturating_sub(1).saturating_sub(local_y);

            let local_pos = UVec2::new(local_x, local_y);
            let filter = filter.and_then(|filter| filters.get(&**filter));
            if !sprite_pixel_visible(sprite, frame, filter.into_iter(), local_pos) {
                continue;
            }

            picks.push((
                id,
                HitData {
                    camera: camera_id,
                    depth,
                    position: None,
                    normal: None,
                },
            ));
        }

        for (
            composite,
            &pos,
            &anchor,
            layer,
            &canvas,
            frame_view,
            frame_control,
            filter,
            visibility,
            id,
        ) in &composites
        {
            if !visibility.get() {
                continue;
            }

            let metrics = if composite.size.x == 0 || composite.size.y == 0 {
                composite.metrics_with(|handle| {
                    let sprite = sprite_assets.get(handle)?;
                    Some(crate::sprite::PxCompositePartMetrics {
                        size: sprite.frame_size(),
                        frame_count: sprite.frame_count(),
                    })
                })
            } else {
                Some(crate::sprite::PxCompositeMetrics {
                    size: composite.size,
                    origin: composite.origin,
                    frame_count: composite.frame_count,
                })
            };
            let Some(metrics) = metrics else {
                continue;
            };

            let rect = spatial_rect(metrics.size, pos, anchor, canvas, cam_pos);
            if !rect.contains_exclusive(cursor) {
                continue;
            }

            let depth = layer_depth(&mut layer_depths, layer);
            let frame = frame_view
                .copied()
                .or_else(|| frame_control.copied().map(PxFrameView::from));
            let master_count = metrics.frame_count;
            let local = cursor - rect.min;
            let entity_filter = filter.and_then(|filter| filters.get(&**filter));

            let mut hit = false;
            for part in &composite.parts {
                let Some(sprite) = sprite_assets.get(&part.sprite) else {
                    continue;
                };

                let part_size = sprite.frame_size().as_ivec2();
                if part_size.x == 0 || part_size.y == 0 {
                    continue;
                }

                let part_origin = part.offset - metrics.origin;
                let part_local = local - part_origin;
                if part_local.x < 0
                    || part_local.y < 0
                    || part_local.x >= part_size.x
                    || part_local.y >= part_size.y
                {
                    continue;
                }

                let part_frame =
                    resolve_frame_binding(frame, master_count, sprite.frame_count(), &part.frame);
                let local_x = part_local.x as u32;
                let local_y = part_local.y as u32;
                let local_y = part_size.y as u32 - 1 - local_y;
                let local_pos = UVec2::new(local_x, local_y);
                let part_filter = part.filter.as_ref().and_then(|filter| filters.get(filter));
                if sprite_pixel_visible(
                    sprite,
                    part_frame,
                    [part_filter, entity_filter].into_iter().flatten(),
                    local_pos,
                ) {
                    hit = true;
                    break;
                }
            }

            if hit {
                picks.push((
                    id,
                    HitData {
                        camera: camera_id,
                        depth,
                        position: None,
                        normal: None,
                    },
                ));
            }
        }

        hits.write(PointerHits {
            pointer,
            picks,
            order: camera.order as f32,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::{PxFrameSelector, PxFrameTransition};
    use crate::image::PxImage;

    #[test]
    fn pixel_pick_uses_local_pos_for_dither() {
        let sprite = PxSpriteAsset {
            data: PxImage::new(vec![0, 0, 0, 0, 1, 0, 0, 0], 2),
            frame_size: 4,
        };
        let frame = PxFrameView {
            selector: PxFrameSelector::Index(0.5),
            transition: PxFrameTransition::Dither,
        };

        let hit_a = sprite_pixel_visible(&sprite, Some(frame), [], UVec2::new(0, 0));
        let hit_b = sprite_pixel_visible(&sprite, Some(frame), [], UVec2::new(1, 0));

        assert!(hit_a);
        assert!(!hit_b);
    }
}
