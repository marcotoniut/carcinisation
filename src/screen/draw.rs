use std::collections::BTreeMap;

#[cfg(feature = "headed")]
use bevy_ecs::{query::QueryState, world::World};
#[cfg(feature = "headed")]
use bevy_render::render_asset::RenderAssets;

#[cfg(feature = "line")]
use crate::line::draw_line;
use crate::{
    animation::draw_spatial,
    cursor::{CursorState, PxCursorPosition},
    filter::{PxFilterAsset, draw_filter},
    image::{PxImage, PxImageSliceMut},
    map::{PxTile, PxTileset},
    prelude::*,
    sprite::PxSpriteAsset,
    text::PxTypeface,
};

use super::pipeline::PxRenderBuffer;
use crate::map::TileComponents;

pub(crate) type MapEntry<'a> = (
    &'a PxMap,
    PxPosition,
    PxCanvas,
    Option<&'a PxFrame>,
    Option<&'a PxFilter>,
);

pub(crate) type SpriteEntry<'a> = (
    &'a PxSprite,
    PxPosition,
    PxAnchor,
    PxCanvas,
    Option<&'a PxFrame>,
    Option<&'a PxFilter>,
);

pub(crate) type TextEntry<'a> = (
    &'a PxText,
    PxPosition,
    PxAnchor,
    PxCanvas,
    Option<&'a PxFrame>,
    Option<&'a PxFilter>,
);

pub(crate) type RectEntry<'a> = (
    PxRect,
    &'a PxFilter,
    PxPosition,
    PxAnchor,
    PxCanvas,
    Option<&'a PxFrame>,
    bool,
);

#[cfg(feature = "line")]
pub(crate) type LineEntry<'a> = (
    &'a PxLine,
    &'a PxFilter,
    PxCanvas,
    Option<&'a PxFrame>,
    bool,
);

pub(crate) type FilterEntry<'a> = (&'a PxFilter, Option<&'a PxFrame>);

#[cfg(feature = "line")]
pub(crate) type LayerContents<'a> = (
    Vec<MapEntry<'a>>,
    Vec<SpriteEntry<'a>>,
    Vec<TextEntry<'a>>,
    Vec<RectEntry<'a>>,
    Vec<LineEntry<'a>>,
    Vec<FilterEntry<'a>>,
    Vec<RectEntry<'a>>,
    Vec<LineEntry<'a>>,
    Vec<FilterEntry<'a>>,
);

#[cfg(not(feature = "line"))]
pub(crate) type LayerContents<'a> = (
    Vec<MapEntry<'a>>,
    Vec<SpriteEntry<'a>>,
    Vec<TextEntry<'a>>,
    Vec<RectEntry<'a>>,
    (),
    Vec<FilterEntry<'a>>,
    Vec<RectEntry<'a>>,
    (),
    Vec<FilterEntry<'a>>,
);

pub(crate) type LayerContentsMap<'a, L> = BTreeMap<L, LayerContents<'a>>;

#[cfg(feature = "headed")]
pub(crate) fn draw_layers<'w, L: PxLayer>(
    world: &'w World,
    render_buffer: &PxRenderBuffer,
    camera: PxCamera,
    layer_contents: LayerContentsMap<'w, L>,
    tiles: &QueryState<TileComponents>,
) {
    let tilesets = world.resource::<RenderAssets<PxTileset>>();
    let sprite_assets = world.resource::<RenderAssets<PxSpriteAsset>>();
    let typefaces = world.resource::<RenderAssets<PxTypeface>>();
    let filters = world.resource::<RenderAssets<PxFilterAsset>>();

    {
        let mut inner = render_buffer.write_inner();
        let image = inner.image.as_mut().unwrap();
        let mut layer_image = PxImage::empty_from_image(image);
        let mut image_slice = PxImageSliceMut::from_image_mut(image).unwrap();

        #[allow(unused_variables)]
        for (
            _,
            (
                maps,
                sprites,
                texts,
                clip_rects,
                clip_lines,
                clip_filters,
                over_rects,
                over_lines,
                over_filters,
            ),
        ) in layer_contents.into_iter()
        {
            layer_image.clear();
            let mut layer_slice = layer_image.slice_all_mut();

            for (map, position, canvas, frame, map_filter) in maps {
                let Some(tileset) = tilesets.get(&map.tileset) else {
                    continue;
                };

                let map_filter = map_filter.and_then(|map_filter| filters.get(&**map_filter));
                let size = map.tiles.size();

                for x in 0..size.x {
                    for y in 0..size.y {
                        let pos = UVec2::new(x, y);

                        let Some(tile) = map.tiles.get(pos) else {
                            continue;
                        };

                        let Ok((&PxTile { texture }, tile_filter)) = tiles.get_manual(world, tile)
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
                            PxAnchor::BottomLeft,
                            canvas,
                            frame.copied(),
                            [
                                tile_filter.and_then(|tile_filter| filters.get(&**tile_filter)),
                                map_filter,
                            ]
                            .into_iter()
                            .flatten(),
                            camera,
                        );
                    }
                }
            }

            for (sprite, position, anchor, canvas, frame, filter) in sprites {
                let Some(sprite) = sprite_assets.get(&**sprite) else {
                    continue;
                };

                draw_spatial(
                    sprite,
                    (),
                    &mut layer_slice,
                    position,
                    anchor,
                    canvas,
                    frame.copied(),
                    filter.and_then(|filter| filters.get(&**filter)),
                    camera,
                );
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
                        PxPosition(top_left + ivec2(x as i32, -(y as i32))),
                        PxAnchor::TopLeft,
                        canvas,
                        frame.copied(),
                        filter.and_then(|filter| filters.get(&**filter)),
                        camera,
                    );
                }
            }

            for (rect, filter, pos, anchor, canvas, frame, invert) in clip_rects {
                if let Some(filter) = filters.get(&**filter) {
                    draw_spatial(
                        &(rect, filter),
                        invert,
                        &mut layer_slice,
                        pos,
                        anchor,
                        canvas,
                        frame.copied(),
                        std::iter::empty(),
                        camera,
                    );
                }
            }

            #[cfg(feature = "line")]
            for (line, filter, canvas, frame, invert) in clip_lines {
                if let Some(filter) = filters.get(&**filter) {
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
                if let Some(filter) = filters.get(&**filter) {
                    draw_filter(filter, frame.copied(), &mut layer_slice);
                }
            }

            image_slice.draw(&layer_image);

            for (rect, filter, pos, anchor, canvas, frame, invert) in over_rects {
                if let Some(filter) = filters.get(&**filter) {
                    draw_spatial(
                        &(rect, filter),
                        invert,
                        &mut image_slice,
                        pos,
                        anchor,
                        canvas,
                        frame.copied(),
                        std::iter::empty(),
                        camera,
                    );
                }
            }

            #[cfg(feature = "line")]
            for (line, filter, canvas, frame, invert) in over_lines {
                if let Some(filter) = filters.get(&**filter) {
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
            }

            for (filter, frame) in over_filters {
                if let Some(filter) = filters.get(&**filter) {
                    draw_filter(filter, frame.copied(), &mut image_slice);
                }
            }
        }
    }

    let cursor = world.resource::<CursorState>();

    if let PxCursor::Filter {
        idle,
        left_click,
        right_click,
    } = world.resource()
        && let Some(cursor_pos) = **world.resource::<PxCursorPosition>()
        && let Some(PxFilterAsset(filter)) = filters.get(match cursor {
            CursorState::Idle => idle,
            CursorState::Left => left_click,
            CursorState::Right => right_click,
        })
    {
        let mut inner = render_buffer.write_inner();
        let image = inner.image.as_mut().unwrap();
        let mut cursor_image = PxImageSliceMut::from_image_mut(image).unwrap();
        if let Some(pixel) = cursor_image.get_pixel_mut(IVec2::new(
            cursor_pos.x as i32,
            cursor_image.height() as i32 - 1 - cursor_pos.y as i32,
        )) {
            if let Some(new_pixel) = filter.get_pixel(IVec2::new(*pixel as i32, 0)) {
                *pixel = new_pixel;
            } else {
                error!("`PxCursor` filter is the wrong size");
            }
        }
    }
}
