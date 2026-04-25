use std::collections::BTreeMap;

#[cfg(feature = "headed")]
use bevy_ecs::{query::QueryState, world::World};
#[cfg(feature = "headed")]
use bevy_render::render_asset::RenderAssets;
#[cfg(feature = "gpu_palette")]
use bytemuck::cast_slice_mut;

#[cfg(feature = "line")]
use crate::line::draw_line;
use crate::{
    atlas::CxSpriteAtlasAsset,
    cursor::{CursorState, CxCursorPosition},
    filter::{CxFilterAsset, draw_filter, resolve_filter},
    frame::{
        Frames, blit_transformed, draw_spatial, draw_spatial_transformed, resolve_frame_binding,
    },
    image::{CxImage, CxImageSliceMut},
    math::RectExt,
    position::Spatial,
    prelude::*,
    sprite::{
        CxCompositeMetrics, CxCompositePartDrawable, CxCompositePartMetrics, CxCompositeSprite,
        CxSpriteAsset, log_composite_part_resolve_error,
    },
    text::CxTypeface,
    tilemap::{CxTile, CxTileset},
};

#[cfg(feature = "headed")]
use super::pipeline::{CxRenderBuffer, CxRenderBufferInner};
use crate::tilemap::TileComponents;

pub(crate) type MapEntry<'a> = (
    &'a CxTilemap,
    CxPosition,
    CxRenderSpace,
    Option<&'a CxFrameView>,
    Option<&'a CxFilter>,
);

pub(crate) type SpriteEntry<'a> = (
    &'a CxSprite,
    CxPosition,
    CxAnchor,
    CxRenderSpace,
    Option<&'a CxFrameView>,
    Option<&'a CxFilter>,
    Option<crate::presentation::CxPresentationTransform>,
);

pub(crate) type AtlasSpriteEntry<'a> = (
    &'a CxAtlasSprite,
    CxPosition,
    CxAnchor,
    CxRenderSpace,
    Option<&'a CxFrameView>,
    Option<&'a CxFilter>,
    Option<crate::presentation::CxPresentationTransform>,
);

pub(crate) type CompositeSpriteEntry<'a> = (
    &'a CxCompositeSprite,
    CxPosition,
    CxAnchor,
    CxRenderSpace,
    Option<&'a CxFrameView>,
    Option<&'a CxFilter>,
    Option<crate::presentation::CxPresentationTransform>,
);

pub(crate) type TextEntry<'a> = (
    &'a CxText,
    CxPosition,
    CxAnchor,
    CxRenderSpace,
    Option<&'a CxFrameView>,
    Option<&'a CxFilter>,
);

#[cfg(feature = "line")]
pub(crate) type LineEntry<'a> = (
    &'a CxLine,
    &'a CxFilter,
    CxRenderSpace,
    Option<&'a CxFrameView>,
    bool,
);

pub(crate) type PrimitiveEntry<'a> = (
    &'a crate::primitive::CxPrimitive,
    CxPosition,
    CxAnchor,
    CxRenderSpace,
    Option<crate::presentation::CxPresentationTransform>,
);

pub(crate) type FilterEntry<'a> = (&'a CxFilter, Option<&'a CxFrameView>);

#[cfg(feature = "line")]
#[derive(Default)]
pub(crate) struct LayerContents<'a> {
    pub(crate) maps: Vec<MapEntry<'a>>,
    pub(crate) primitives: Vec<PrimitiveEntry<'a>>,
    pub(crate) sprites: Vec<SpriteEntry<'a>>,
    pub(crate) atlas_sprites: Vec<AtlasSpriteEntry<'a>>,
    pub(crate) composites: Vec<CompositeSpriteEntry<'a>>,
    pub(crate) texts: Vec<TextEntry<'a>>,
    pub(crate) clip_lines: Vec<LineEntry<'a>>,
    pub(crate) clip_filters: Vec<FilterEntry<'a>>,
    pub(crate) over_lines: Vec<LineEntry<'a>>,
    pub(crate) over_filters: Vec<FilterEntry<'a>>,
}

#[cfg(not(feature = "line"))]
#[derive(Default)]
pub(crate) struct LayerContents<'a> {
    pub(crate) maps: Vec<MapEntry<'a>>,
    pub(crate) primitives: Vec<PrimitiveEntry<'a>>,
    pub(crate) sprites: Vec<SpriteEntry<'a>>,
    pub(crate) atlas_sprites: Vec<AtlasSpriteEntry<'a>>,
    pub(crate) composites: Vec<CompositeSpriteEntry<'a>>,
    pub(crate) texts: Vec<TextEntry<'a>>,
    pub(crate) clip_filters: Vec<FilterEntry<'a>>,
    pub(crate) over_filters: Vec<FilterEntry<'a>>,
}

impl<'a> LayerContents<'a> {
    pub(crate) fn push_filter(&mut self, filter: FilterEntry<'a>, clip: bool) {
        if clip {
            self.clip_filters.push(filter);
        } else {
            self.over_filters.push(filter);
        }
    }
}

#[cfg(feature = "line")]
impl<'a> LayerContents<'a> {
    pub(crate) fn push_line(&mut self, line: LineEntry<'a>, clip: bool) {
        if clip {
            self.clip_lines.push(line);
        } else {
            self.over_lines.push(line);
        }
    }
}

pub(crate) type LayerContentsMap<'a, L> = BTreeMap<L, LayerContents<'a>>;

#[cfg(feature = "headed")]
pub(crate) fn draw_layers<'w, L: CxLayer>(
    world: &'w World,
    render_buffer: &CxRenderBuffer,
    camera: CxCamera,
    layer_contents: LayerContentsMap<'w, L>,
    tiles: &QueryState<TileComponents>,
    #[cfg(feature = "gpu_palette")] layer_order: &[L],
) {
    let tilesets = world.resource::<RenderAssets<CxTileset>>();
    let atlas_assets = world.resource::<RenderAssets<CxSpriteAtlasAsset>>();
    let sprite_assets = world.resource::<RenderAssets<CxSpriteAsset>>();
    let typefaces = world.resource::<RenderAssets<CxTypeface>>();
    let filters = world.resource::<RenderAssets<CxFilterAsset>>();

    {
        let mut inner = render_buffer.write_inner();
        let CxRenderBufferInner {
            image,
            #[cfg(feature = "gpu_palette")]
            depth_image,
            ..
        } = &mut *inner;
        let image = image.as_mut().unwrap();
        #[cfg(feature = "gpu_palette")]
        let mut depth_data = depth_image
            .as_mut()
            .and_then(|depth| depth.data.as_mut())
            .map(|data| cast_slice_mut::<u8, u16>(data.as_mut_slice()));
        #[cfg(feature = "gpu_palette")]
        let (image_width, image_height) = {
            let height = image.height() as usize;
            (image.width() as usize, height)
        };
        let mut layer_image = CxImage::empty_from_image(image);
        let mut image_slice = CxImageSliceMut::from_image_mut(image).unwrap();

        #[allow(unused_variables)]
        for (layer, contents) in layer_contents {
            let LayerContents {
                primitives,
                maps,
                sprites,
                atlas_sprites,
                composites,
                texts,
                #[cfg(feature = "line")]
                clip_lines,
                clip_filters,
                #[cfg(feature = "line")]
                over_lines,
                over_filters,
            } = contents;
            #[cfg(feature = "gpu_palette")]
            let base_depth = layer_index_for(layer_order, &layer);
            #[cfg(feature = "gpu_palette")]
            let over_depth = base_depth.map(|depth| depth.saturating_add(1));
            layer_image.clear();
            let mut layer_slice = layer_image.slice_all_mut();

            for (map, position, canvas, frame, map_filter) in maps {
                let Some(tileset) = tilesets.get(&map.tileset) else {
                    continue;
                };

                let map_filter = resolve_filter(map_filter, filters);
                let size = map.tiles.size();

                for x in 0..size.x {
                    for y in 0..size.y {
                        let pos = UVec2::new(x, y);

                        let Some(tile) = map.tiles.get(pos) else {
                            continue;
                        };

                        let Ok((&CxTile { texture }, tile_filter)) = tiles.get_manual(world, tile)
                        else {
                            continue;
                        };

                        let Some(tile) = tileset.tileset.get(texture as usize) else {
                            error!(
                                "tile texture index out of bounds: the len is {}, but the index is {texture}",
                                tileset.tileset.len()
                            );
                            continue;
                        };

                        draw_spatial(
                            tile,
                            (),
                            &mut layer_slice,
                            (*position + pos.as_ivec2() * tileset.tile_size().as_ivec2()).into(),
                            CxAnchor::BottomLeft,
                            canvas,
                            frame.copied(),
                            [resolve_filter(tile_filter, filters), map_filter]
                                .into_iter()
                                .flatten(),
                            camera,
                        );
                    }
                }
            }

            // Primitives draw after maps but before sprites, so sprites
            // always paint on top and map tiles form the backdrop.
            for (prim, position, anchor, canvas, presentation) in primitives {
                let size = prim.shape.frame_size();
                let visual_offset = presentation.map_or(Vec2::ZERO, |pt| pt.visual_offset);
                let position = *position + visual_offset.round().as_ivec2();
                let position = position - anchor.ipos(size);
                let world_origin = position;
                let position = match canvas {
                    CxRenderSpace::World => position - *camera,
                    CxRenderSpace::Camera => position,
                };
                // Image space: Y flipped (top-left origin).
                let image_pos =
                    IVec2::new(position.x, layer_slice.flip_y(position.y) - size.y as i32);
                let mut prim_slice = layer_slice.slice_mut(IRect::from_pos_size(image_pos, size));
                crate::primitive::draw_primitive(prim, &mut prim_slice, world_origin);
            }

            for (sprite, position, anchor, canvas, frame, filter, presentation) in sprites {
                let Some(sprite) = sprite_assets.get(&**sprite) else {
                    continue;
                };

                let resolved_filters = resolve_filter(filter, filters);

                if let Some(pt) = presentation
                    && pt.needs_transformed_blit()
                {
                    // Transformed path: scratch buffer + nearest-neighbour blit.
                    draw_spatial_transformed(
                        sprite,
                        (),
                        &mut layer_slice,
                        position,
                        anchor,
                        canvas,
                        frame.copied(),
                        resolved_filters,
                        camera,
                        pt.clamped_scale(),
                        pt.sanitised_rotation(),
                        pt.visual_offset,
                    );
                } else {
                    // Unscaled path: offset-only adjustment (if any), no scratch buffer.
                    let adjusted_pos = if let Some(pt) = presentation
                        && pt.has_offset()
                    {
                        CxPosition(*position + pt.visual_offset.round().as_ivec2())
                    } else {
                        position
                    };
                    draw_spatial(
                        sprite,
                        (),
                        &mut layer_slice,
                        adjusted_pos,
                        anchor,
                        canvas,
                        frame.copied(),
                        resolved_filters,
                        camera,
                    );
                }
            }

            for (sprite, position, anchor, canvas, frame, filter, presentation) in atlas_sprites {
                let Some(atlas) = atlas_assets.get(&sprite.atlas) else {
                    continue;
                };
                let Some(region) = atlas.region(sprite.region) else {
                    continue;
                };

                let resolved_filters = resolve_filter(filter, filters);

                if let Some(pt) = presentation
                    && pt.needs_transformed_blit()
                {
                    draw_spatial_transformed(
                        &(atlas, region),
                        (),
                        &mut layer_slice,
                        position,
                        anchor,
                        canvas,
                        frame.copied(),
                        resolved_filters,
                        camera,
                        pt.clamped_scale(),
                        pt.sanitised_rotation(),
                        pt.visual_offset,
                    );
                } else {
                    let adjusted_pos = if let Some(pt) = presentation
                        && pt.has_offset()
                    {
                        CxPosition(*position + pt.visual_offset.round().as_ivec2())
                    } else {
                        position
                    };
                    draw_spatial(
                        &(atlas, region),
                        (),
                        &mut layer_slice,
                        adjusted_pos,
                        anchor,
                        canvas,
                        frame.copied(),
                        resolved_filters,
                        camera,
                    );
                }
            }

            for (composite, position, anchor, canvas, frame, filter, presentation) in composites {
                let metrics = if composite.size.x == 0 || composite.size.y == 0 {
                    composite.metrics_with(|source| {
                        source
                            .resolve(
                                |handle| sprite_assets.get(handle),
                                |handle| atlas_assets.get(handle),
                            )
                            .ok()
                            .map(|resolved| CxCompositePartMetrics {
                                size: resolved.frame_size(),
                                frame_count: resolved.frame_count(),
                            })
                    })
                } else {
                    Some(CxCompositeMetrics {
                        size: composite.size,
                        origin: composite.origin,
                        render_size: composite.render_size,
                        render_origin: composite.render_origin,
                        frame_count: composite.frame_count,
                    })
                };
                let Some(metrics) = metrics else {
                    continue;
                };

                // Nothing to render for zero-area composites (metrics not yet
                // populated, e.g. assets still loading).
                if metrics.render_size.x == 0 || metrics.render_size.y == 0 {
                    continue;
                }

                let needs_scaled = presentation.is_some_and(|pt| pt.needs_transformed_blit());

                if needs_scaled {
                    let pt = presentation.unwrap();
                    let scale = pt.clamped_scale();

                    // Compose all parts into a scratch buffer sized to the render
                    // envelope (may be larger than base size for transformed parts).
                    let mut scratch = crate::image::CxImage::empty(metrics.render_size);
                    let mut scratch_slice = scratch.slice_all_mut();
                    let master = frame.copied();
                    let master_count = metrics.frame_count;

                    for (part_index, part) in composite.parts.iter().enumerate() {
                        let resolved = match part.source.resolve(
                            |handle| sprite_assets.get(handle),
                            |handle| atlas_assets.get(handle),
                        ) {
                            Ok(resolved) => resolved,
                            Err(error) => {
                                log_composite_part_resolve_error(part_index, &error);
                                continue;
                            }
                        };

                        let part_frame = resolve_frame_binding(
                            master,
                            master_count,
                            resolved.frame_count(),
                            &part.frame,
                        );
                        let part_filter =
                            part.filter.as_ref().and_then(|handle| filters.get(handle));
                        let entity_filter = resolve_filter(filter, filters);
                        let drawable = CxCompositePartDrawable {
                            resolved,
                            flip_x: part.flip_x,
                            flip_y: part.flip_y,
                        };

                        let needs_part_transform =
                            part.transform.as_ref().is_some_and(|t| !t.is_identity());

                        if needs_part_transform {
                            let t = part.transform.as_ref().unwrap();
                            let part_size = drawable.frame_size();
                            if part_size.x == 0 || part_size.y == 0 {
                                continue;
                            }

                            // Render part into a mini scratch at native size.
                            let mut mini = CxImage::empty(part_size);
                            let mut mini_slice = mini.slice_all_mut();
                            crate::frame::draw_frame(
                                &drawable,
                                (),
                                &mut mini_slice,
                                part_frame,
                                [part_filter, entity_filter].into_iter().flatten(),
                            );

                            // Pivot position in render-scratch engine-space.
                            let part_bl = (part.offset - metrics.render_origin).as_vec2();
                            let ps = part_size.as_vec2();
                            let pivot_pos =
                                part_bl + Vec2::new(t.pivot.x * ps.x, (1.0 - t.pivot.y) * ps.y);

                            blit_transformed(
                                &mini,
                                part_size,
                                &mut scratch_slice,
                                CxPosition(pivot_pos.round().as_ivec2()),
                                t.anchor(),
                                CxRenderSpace::Camera,
                                CxCamera(IVec2::ZERO),
                                t.clamped_scale(),
                                t.sanitised_rotation(),
                                Vec2::ZERO,
                            );
                        } else {
                            let part_pos = part.offset - metrics.render_origin;
                            draw_spatial(
                                &drawable,
                                (),
                                &mut scratch_slice,
                                part_pos.into(),
                                CxAnchor::BottomLeft,
                                CxRenderSpace::Camera,
                                part_frame,
                                [part_filter, entity_filter].into_iter().flatten(),
                                CxCamera(IVec2::ZERO),
                            );
                        }
                    }

                    // Blit the composed scratch buffer transformed to the layer.
                    // The scratch is render_size (may be larger than base size).
                    // Compute a custom anchor that maps the entity's anchor to the
                    // correct position within the enlarged render scratch: the base
                    // frame sits at offset (origin - render_origin) inside the scratch.
                    let base_in_scratch = (metrics.origin - metrics.render_origin).as_vec2();
                    let base_anchor = anchor.pos(metrics.size).as_vec2();
                    let render_anchor_px = base_in_scratch + base_anchor;
                    // render_size is guaranteed non-zero here (zero-area
                    // composites are skipped above).
                    let render_anchor = CxAnchor::Custom(Vec2::new(
                        render_anchor_px.x / metrics.render_size.x as f32,
                        render_anchor_px.y / metrics.render_size.y as f32,
                    ));
                    blit_transformed(
                        &scratch,
                        metrics.render_size,
                        &mut layer_slice,
                        position,
                        render_anchor,
                        canvas,
                        camera,
                        scale,
                        pt.sanitised_rotation(),
                        pt.visual_offset,
                    );
                } else {
                    // Unscaled path: draw parts directly to layer. Apply offset if present.
                    let offset_adjust = presentation
                        .filter(super::super::presentation::CxPresentationTransform::has_offset)
                        .map_or(IVec2::ZERO, |pt| pt.visual_offset.round().as_ivec2());
                    let base_pos = *position + offset_adjust - anchor.pos(metrics.size).as_ivec2();
                    let master = frame.copied();
                    let master_count = metrics.frame_count;

                    for (part_index, part) in composite.parts.iter().enumerate() {
                        let resolved = match part.source.resolve(
                            |handle| sprite_assets.get(handle),
                            |handle| atlas_assets.get(handle),
                        ) {
                            Ok(resolved) => resolved,
                            Err(error) => {
                                log_composite_part_resolve_error(part_index, &error);
                                continue;
                            }
                        };

                        let part_frame = resolve_frame_binding(
                            master,
                            master_count,
                            resolved.frame_count(),
                            &part.frame,
                        );
                        let part_filter =
                            part.filter.as_ref().and_then(|handle| filters.get(handle));
                        let entity_filter = resolve_filter(filter, filters);
                        let drawable = CxCompositePartDrawable {
                            resolved,
                            flip_x: part.flip_x,
                            flip_y: part.flip_y,
                        };

                        let needs_part_transform =
                            part.transform.as_ref().is_some_and(|t| !t.is_identity());

                        if needs_part_transform {
                            let t = part.transform.as_ref().unwrap();
                            let part_size = drawable.frame_size();
                            if part_size.x == 0 || part_size.y == 0 {
                                continue;
                            }

                            // Render part into a mini scratch at native size.
                            let mut mini = CxImage::empty(part_size);
                            let mut mini_slice = mini.slice_all_mut();
                            crate::frame::draw_frame(
                                &drawable,
                                (),
                                &mut mini_slice,
                                part_frame,
                                [part_filter, entity_filter].into_iter().flatten(),
                            );

                            // Pivot position in world engine-space.
                            let part_bl = (base_pos + (part.offset - metrics.origin)).as_vec2();
                            let ps = part_size.as_vec2();
                            let pivot_pos =
                                part_bl + Vec2::new(t.pivot.x * ps.x, (1.0 - t.pivot.y) * ps.y);

                            blit_transformed(
                                &mini,
                                part_size,
                                &mut layer_slice,
                                CxPosition(pivot_pos.round().as_ivec2()),
                                t.anchor(),
                                canvas,
                                camera,
                                t.clamped_scale(),
                                t.sanitised_rotation(),
                                Vec2::ZERO,
                            );
                        } else {
                            let part_pos = base_pos + (part.offset - metrics.origin);
                            draw_spatial(
                                &drawable,
                                (),
                                &mut layer_slice,
                                part_pos.into(),
                                CxAnchor::BottomLeft,
                                canvas,
                                part_frame,
                                [part_filter, entity_filter].into_iter().flatten(),
                                camera,
                            );
                        }
                    }
                }
            }

            for (text, pos, alignment, canvas, frame, filter) in texts {
                let Some(typeface) = typefaces.get(&text.typeface) else {
                    continue;
                };

                let line_break_count = text.line_breaks.len() as u32;
                let mut size = uvec2(
                    0,
                    (line_break_count + 1) * typeface.height + line_break_count,
                );
                let mut x = 0;
                let mut y = 0;
                let mut chars = Vec::new();
                let mut line_break_index = 0;

                for (index, char) in text.value.chars().enumerate() {
                    if let Some(char) = typeface.characters.get(&char) {
                        if x != 0 {
                            x += 1;
                        }

                        chars.push((x, y, char));
                        x += char.data.size().x;

                        if x > size.x {
                            size.x = x;
                        }
                    } else if let Some(separator) = typeface.separators.get(&char) {
                        x += separator.width;
                    } else {
                        error!(r#"character "{char}" in text isn't in typeface"#);
                    }

                    if text.line_breaks.get(line_break_index).copied() == Some(index as u32) {
                        line_break_index += 1;
                        y += typeface.height + 1;
                        x = 0;
                    }
                }

                let top_left = *pos - alignment.pos(size).as_ivec2() + ivec2(0, size.y as i32 - 1);

                for (x, y, char) in chars {
                    draw_spatial(
                        char,
                        (),
                        &mut layer_slice,
                        CxPosition(top_left + ivec2(x as i32, -(y as i32))),
                        CxAnchor::TopLeft,
                        canvas,
                        frame.copied(),
                        resolve_filter(filter, filters),
                        camera,
                    );
                }
            }

            #[cfg(feature = "line")]
            for (line, filter, canvas, frame, invert) in clip_lines {
                if let Some(filter) = filter.resolve(filters) {
                    draw_line(
                        line,
                        filter,
                        invert,
                        &mut layer_slice,
                        canvas,
                        frame.copied(),
                        camera,
                    );
                }
            }

            for (filter, frame) in clip_filters {
                if let Some(filter) = filter.resolve(filters) {
                    draw_filter(filter, frame.copied(), &mut layer_slice);
                }
            }

            image_slice.draw(&layer_image);
            #[cfg(feature = "gpu_palette")]
            if let (Some(depth), Some(base_depth)) = (depth_data.as_mut(), base_depth) {
                update_depth_from_layer(depth, layer_image.data(), base_depth);
            }

            #[cfg(feature = "line")]
            for (line, filter, canvas, frame, invert) in over_lines {
                if let Some(filter) = filter.resolve(filters) {
                    draw_line(
                        line,
                        filter,
                        invert,
                        &mut image_slice,
                        canvas,
                        frame.copied(),
                        camera,
                    );
                }
                #[cfg(all(feature = "gpu_palette", feature = "line"))]
                if let (Some(depth), Some(over_depth)) = (depth_data.as_mut(), over_depth) {
                    update_depth_line(
                        depth,
                        image_width,
                        image_height,
                        line,
                        canvas,
                        camera,
                        invert,
                        over_depth,
                    );
                }
            }

            for (filter, frame) in over_filters {
                if let Some(filter) = filter.resolve(filters) {
                    draw_filter(filter, frame.copied(), &mut image_slice);
                }
                #[cfg(feature = "gpu_palette")]
                if let (Some(depth), Some(over_depth)) = (depth_data.as_mut(), over_depth) {
                    depth.fill(over_depth);
                }
            }
        }
    }

    let cursor = world.resource::<CursorState>();

    if let CxCursor::Filter {
        idle,
        left_click,
        right_click,
    } = world.resource()
        && let Some(cursor_pos) = **world.resource::<CxCursorPosition>()
        && let Some(CxFilterAsset(filter)) = filters.get(match cursor {
            CursorState::Idle => idle,
            CursorState::Left => left_click,
            CursorState::Right => right_click,
        })
    {
        let mut inner = render_buffer.write_inner();
        let CxRenderBufferInner {
            image,
            #[cfg(feature = "gpu_palette")]
            depth_image,
            ..
        } = &mut *inner;
        let image = image.as_mut().unwrap();
        let mut cursor_image = CxImageSliceMut::from_image_mut(image).unwrap();
        let cursor_pos = IVec2::new(
            cursor_pos.x as i32,
            cursor_image.flip_y(cursor_pos.y as i32) - 1,
        );
        if let Some(pixel) = cursor_image.get_pixel_mut(cursor_pos) {
            if let Some(new_pixel) = filter.get_pixel(IVec2::new(i32::from(*pixel), 0)) {
                *pixel = new_pixel;
            } else {
                error!("`CxCursor` filter is the wrong size");
            }
        }
        #[cfg(feature = "gpu_palette")]
        if let Some(depth) = depth_image.as_mut().and_then(|depth| depth.data.as_mut()) {
            let width = cursor_image.width;
            if cursor_pos.x >= 0 && cursor_pos.y >= 0 {
                let x = cursor_pos.x as usize;
                let y = cursor_pos.y as usize;
                if x < width && y < cursor_image.image.len() {
                    let depth = cast_slice_mut::<u8, u16>(depth);
                    depth[y * width + x] = u16::MAX;
                }
            }
        }
    }
}

#[cfg(feature = "gpu_palette")]
fn layer_index_for<L: CxLayer>(layers: &[L], layer: &L) -> Option<u16> {
    let index = layers.binary_search(layer).ok()?;
    let base = (index + 1).checked_mul(2)?;
    u16::try_from(base).ok()
}

#[cfg(feature = "gpu_palette")]
fn update_depth_from_layer(depth: &mut [u16], layer_data: &[u8], depth_value: u16) {
    for (index, value) in layer_data.iter().enumerate() {
        if *value != crate::palette::TRANSPARENT_INDEX {
            depth[index] = depth_value;
        }
    }
}

#[cfg(feature = "gpu_palette")]
#[allow(dead_code)]
fn spatial_bounds(
    size: UVec2,
    position: CxPosition,
    anchor: CxAnchor,
    canvas: CxRenderSpace,
    camera: CxCamera,
    image_height: i32,
) -> IRect {
    let position = *position - anchor.ipos(size);
    let position = match canvas {
        CxRenderSpace::World => position - *camera,
        CxRenderSpace::Camera => position,
    };
    let position = IVec2::new(position.x, image_height - position.y);
    let size = size.as_ivec2();

    IRect {
        min: position - IVec2::new(0, size.y),
        max: position + IVec2::new(size.x, 0),
    }
}

#[cfg(feature = "gpu_palette")]
#[allow(dead_code)]
fn update_depth_rect(
    depth: &mut [u16],
    width: usize,
    height: usize,
    rect: IRect,
    invert: bool,
    depth_value: u16,
) {
    let x_min = rect.min.x.clamp(0, width as i32) as usize;
    let x_max = rect.max.x.clamp(0, width as i32) as usize;
    let y_min = rect.min.y.clamp(0, height as i32) as usize;
    let y_max = rect.max.y.clamp(0, height as i32) as usize;

    if invert {
        for y in 0..height {
            for x in 0..width {
                let inside = x >= x_min && x < x_max && y >= y_min && y < y_max;
                if !inside {
                    depth[y * width + x] = depth_value;
                }
            }
        }
    } else {
        for y in y_min..y_max {
            for x in x_min..x_max {
                depth[y * width + x] = depth_value;
            }
        }
    }
}

#[cfg(all(feature = "gpu_palette", feature = "line"))]
fn update_depth_line(
    depth: &mut [u16],
    width: usize,
    height: usize,
    line: &CxLine,
    canvas: CxRenderSpace,
    camera: CxCamera,
    invert: bool,
    depth_value: u16,
) {
    use bevy_platform::collections::HashSet;
    use line_drawing::Bresenham;

    let offset = match canvas {
        CxRenderSpace::World => -*camera,
        CxRenderSpace::Camera => IVec2::ZERO,
    };

    let mut poses = HashSet::new();
    for (segment_index, (start, end)) in line.iter().zip(line.iter().skip(1)).enumerate() {
        let start = *start + offset;
        let end = *end + offset;

        for (step, pos) in Bresenham::new(start.into(), end.into()).enumerate() {
            if segment_index > 0 && step == 0 {
                continue;
            }
            poses.insert(IVec2::from(pos));
        }
    }

    if invert {
        for y in 0..height as i32 {
            for x in 0..width as i32 {
                if !poses.contains(&IVec2::new(x, y)) {
                    depth[y as usize * width + x as usize] = depth_value;
                }
            }
        }
    } else {
        for pos in poses {
            if pos.x >= 0 && pos.y >= 0 && (pos.x as usize) < width && (pos.y as usize) < height {
                depth[pos.y as usize * width + pos.x as usize] = depth_value;
            }
        }
    }
}

#[cfg(all(test, feature = "gpu_palette"))]
mod tests {
    use super::*;
    use insta::assert_snapshot;

    fn depth_to_string(depth: &[u16], width: usize, height: usize) -> String {
        let mut out = String::new();
        for y in 0..height {
            for x in 0..width {
                if x > 0 {
                    out.push(' ');
                }
                out.push_str(&depth[y * width + x].to_string());
            }
            if y + 1 < height {
                out.push('\n');
            }
        }
        out
    }

    #[test]
    fn depth_updates_snapshot() {
        let width = 4;
        let height = 4;
        let mut depth = vec![0u16; width * height];
        let layer_image = vec![0, 0, 0, 0, 0, 1, 1, 0, 0, 1, 1, 0, 0, 0, 0, 0];

        update_depth_from_layer(&mut depth, &layer_image, 2);
        let mut out = String::new();
        out.push_str("after_layer_image:\n");
        out.push_str(&depth_to_string(&depth, width, height));
        out.push('\n');

        update_depth_rect(
            &mut depth,
            width,
            height,
            IRect {
                min: IVec2::new(0, 0),
                max: IVec2::new(4, 1),
            },
            false,
            3,
        );
        out.push_str("after_over_rect:\n");
        out.push_str(&depth_to_string(&depth, width, height));
        out.push('\n');

        update_depth_rect(
            &mut depth,
            width,
            height,
            IRect {
                min: IVec2::new(2, 2),
                max: IVec2::new(4, 4),
            },
            true,
            4,
        );
        out.push_str("after_invert_rect:\n");
        out.push_str(&depth_to_string(&depth, width, height));
        out.push('\n');

        assert_snapshot!(&out, @r###"
after_layer_image:
0 0 0 0
0 2 2 0
0 2 2 0
0 0 0 0
after_over_rect:
3 3 3 3
0 2 2 0
0 2 2 0
0 0 0 0
after_invert_rect:
4 4 4 4
4 4 4 4
4 4 2 0
4 4 0 0
"###);
    }
}
