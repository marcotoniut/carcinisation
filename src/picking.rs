use bevy_picking::backend::prelude::*;

use crate::{
    cursor::PxCursorPosition,
    frame::{Frames, PxFrameControl, PxFrameView, animate, resolve_frame_binding},
    math::RectExt,
    position::Spatial,
    prelude::*,
    profiling::{px_end_span, px_profile, px_trace_span},
    screen::Screen,
    set::PxSet,
    sprite::PxSpriteAsset,
};

/// Enable pixel-perfect picking for a sprite.
#[derive(Component, Default, Clone, Copy, Debug)]
pub struct PxPixelPick;

pub(crate) fn plug<L: PxLayer>(app: &mut App) {
    app.add_systems(
        PostUpdate,
        pick::<L>.in_set(PxSet::Picking).run_if(pick_needs_run),
    );
}

fn pick_needs_run(
    cursor: Res<PxCursorPosition>,
    screen: Res<Screen>,
    cameras: Query<(), With<Camera>>,
    pointers: Query<&PointerId>,
) -> bool {
    cursor.is_some()
        && screen.computed_size.x > 0
        && screen.computed_size.y > 0
        && !cameras.is_empty()
        && pointers
            .iter()
            .any(|pointer| matches!(pointer, PointerId::Mouse))
}

fn layer_depth<L: PxLayer>(layer_depths: &mut Vec<(L, f32)>, layer: &L) -> f32 {
    match layer_depths.binary_search_by(|(existing, _)| existing.cmp(layer)) {
        Ok(index) => layer_depths[index].1,
        Err(index) => {
            let depth = match (
                index.checked_sub(1).map(|i| layer_depths[i].1),
                layer_depths.get(index).map(|(_, upper)| *upper),
            ) {
                (Some(lower), Some(upper)) => f32::midpoint(lower, upper),
                (Some(lower), None) => lower - 1.,
                (None, Some(upper)) => upper + 1.,
                (None, None) => 0.,
            };
            layer_depths.insert(index, (layer.clone(), depth));
            depth
        }
    }
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
    mut depth_cache: Local<Vec<(L, f32)>>,
) {
    let _pick_span = px_trace_span!(
        "carapace::picking::pick",
        width = screen.computed_size.x,
        height = screen.computed_size.y
    );
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
    px_profile!(let mut mouse_pointer_count = 0usize);
    px_profile!(let mut hit_count = 0usize);
    for &pointer in &pointers {
        let PointerId::Mouse = pointer else {
            continue;
        };
        px_profile!(mouse_pointer_count += 1);

        depth_cache.clear();
        let mut picks: Option<Vec<(Entity, HitData)>> = None;
        let mut push_pick = |id, depth| {
            picks.get_or_insert_with(Vec::new).push((
                id,
                HitData {
                    camera: camera_id,
                    depth,
                    position: None,
                    normal: None,
                },
            ));
        };

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

            let depth = layer_depth(&mut depth_cache, layer);
            let rect = spatial_rect(*rect, pos, anchor, *canvas, cam_pos);

            if rect.contains_exclusive(cursor) {
                push_pick(id, depth);
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

            let depth = layer_depth(&mut depth_cache, layer);

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

            push_pick(id, depth);
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

            let depth = layer_depth(&mut depth_cache, layer);
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
                push_pick(id, depth);
            }
        }

        let picks = picks.unwrap_or_default();
        px_profile!(hit_count += picks.len());
        hits.write(PointerHits {
            pointer,
            picks,
            order: camera.order as f32,
        });
    }
    px_profile!(emit mouse_pointer_count, hit_count, "carapace::picking::hits");
    px_end_span!(_pick_span);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    use crate::frame::{PxFrameSelector, PxFrameTransition};
    use crate::image::PxImage;
    use crate::screen::Screen;
    use bevy_ecs::{message::Messages, schedule::Schedule, system::RunSystemOnce};

    #[derive(
        bevy_render::extract_component::ExtractComponent,
        Component,
        next::Next,
        Ord,
        PartialOrd,
        Eq,
        PartialEq,
        Clone,
        Default,
        Debug,
    )]
    #[next(path = next::Next)]
    enum TestLayer {
        Back,
        #[default]
        Mid,
        Front,
    }

    fn layer_depth_reference(
        layer_depths: &mut BTreeMap<TestLayer, f32>,
        layer: &TestLayer,
    ) -> f32 {
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

        BTreeMap::insert(layer_depths, layer.clone(), depth);
        depth
    }

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

    #[test]
    fn layer_depth_matches_previous_btreemap_behavior() {
        let sequence = [
            TestLayer::Mid,
            TestLayer::Back,
            TestLayer::Front,
            TestLayer::Mid,
            TestLayer::Front,
        ];

        let mut vec_cache = Vec::new();
        let mut map_cache = BTreeMap::new();
        for layer in sequence {
            let vec_depth = layer_depth(&mut vec_cache, &layer);
            let map_depth = layer_depth_reference(&mut map_cache, &layer);
            assert_eq!(vec_depth, map_depth);
        }
    }

    #[test]
    fn layer_depth_cache_clear_resets_anchor_depth() {
        let mut cache = Vec::new();
        let _ = layer_depth(&mut cache, &TestLayer::Back);
        let _ = layer_depth(&mut cache, &TestLayer::Mid);
        cache.clear();
        assert_eq!(layer_depth(&mut cache, &TestLayer::Front), 0.0);
    }

    #[derive(Resource, Default)]
    struct PickSummary {
        messages: usize,
        picks: usize,
    }

    fn count_pointer_hits(mut hits: MessageReader<PointerHits>, mut summary: ResMut<PickSummary>) {
        for hit in hits.read() {
            summary.messages += 1;
            summary.picks += hit.picks.len();
        }
    }

    #[test]
    fn pick_emits_empty_hits_when_no_entities_match() {
        let mut world = World::new();
        world.init_resource::<Messages<PointerHits>>();
        world.insert_resource(PickSummary::default());
        world.insert_resource(Assets::<PxSpriteAsset>::default());
        world.insert_resource(Assets::<PxFilterAsset>::default());
        world.insert_resource(PxCursorPosition(Some(UVec2::new(1, 1))));
        world.insert_resource(PxCamera::default());
        world.insert_resource(Screen::test_resource(UVec2::new(16, 16)));

        world.spawn(PointerId::Mouse);
        world.spawn(Camera::default());

        let mut schedule = Schedule::default();
        schedule.add_systems((pick::<TestLayer>, count_pointer_hits).chain());
        schedule.run(&mut world);

        let summary = world.resource::<PickSummary>();
        assert_eq!(summary.messages, 1);
        assert_eq!(summary.picks, 0);
    }

    #[test]
    fn pick_run_condition_requires_visible_cursor_and_mouse_pointer() {
        let mut world = World::new();
        world.insert_resource(PxCursorPosition(None));
        world.insert_resource(Screen::test_resource(UVec2::new(16, 16)));
        world.spawn(Camera::default());
        world.spawn(PointerId::Mouse);

        let runs = world.run_system_once(pick_needs_run).unwrap();
        assert!(!runs, "pick should skip when cursor is off-screen");

        *world.resource_mut::<PxCursorPosition>() = PxCursorPosition(Some(UVec2::new(1, 1)));
        let runs = world.run_system_once(pick_needs_run).unwrap();
        assert!(
            runs,
            "pick should run when cursor and mouse pointer are available"
        );
    }

    #[test]
    fn pick_run_condition_skips_without_camera() {
        let mut world = World::new();
        world.insert_resource(PxCursorPosition(Some(UVec2::new(1, 1))));
        world.insert_resource(Screen::test_resource(UVec2::new(16, 16)));
        world.spawn(PointerId::Mouse);

        let runs = world.run_system_once(pick_needs_run).unwrap();
        assert!(!runs, "pick should skip until a camera exists");

        world.spawn(Camera::default());
        let runs = world.run_system_once(pick_needs_run).unwrap();
        assert!(runs, "pick should run once a camera exists");
    }
}
