use crate::{
    position::Spatial,
    prelude::*,
    profiling::{px_end_span, px_trace, px_trace_span},
    screen::Screen,
};

use super::widgets::{PxGrid, PxMargin, PxMinSize, PxRow, PxRowSlot, PxScroll, PxStack};

// If layouting ends up being too slow, make a tree of min sizes up front and lookup in that
fn calc_min_size<L: PxLayer>(
    ui: Entity,
    uis: Query<(
        AnyOf<(
            (&PxMinSize, Option<&Children>),
            (&PxMargin, Option<&Children>),
            (&PxRow, Option<&Children>),
            (&PxGrid, Option<&Children>),
            (&PxStack, Option<&Children>),
            (Option<(&PxScroll, &Children)>, &PxRect, &PxFilterLayers<L>),
            &PxSprite,
            &PxText,
        )>,
        Option<&L>,
        Option<(&PxPosition, &PxCanvas)>,
    )>,
    typefaces: &Assets<PxTypeface>,
    sprites: &Assets<PxSpriteAsset>,
) -> UVec2 {
    fn dim(vec: UVec2, y: bool) -> u32 {
        if y { vec.y } else { vec.x }
    }

    fn dim_mut(vec: &mut UVec2, y: bool) -> &mut u32 {
        if y { &mut vec.y } else { &mut vec.x }
    }

    let Ok(((min_size, margin, row, grid, stack, rect, sprite, text), _, _)) = uis.get(ui) else {
        // This includes `PxSpace`. Surprise, the `PxSpace` component doesn't do anything at all.
        // It's just easier to spawn in UI.
        return UVec2::ZERO;
    };

    if let Some((min_size, children)) = min_size {
        return match children.map(|children| &**children) {
            None | Some([]) => **min_size,
            Some(&[content]) => {
                calc_min_size(content, uis.as_readonly(), typefaces, sprites).max(**min_size)
            }
            Some([_, _, ..]) => {
                warn!("`PxMinSize` has multiple children");
                **min_size
            }
        };
    }

    if let Some((margin, children)) = margin {
        let margin = 2 * UVec2::splat(**margin);

        return match children.map(|children| &**children) {
            None | Some([]) => margin,
            Some(&[content]) => {
                calc_min_size(content, uis.as_readonly(), typefaces, sprites) + margin
            }
            Some([_, _, ..]) => {
                warn!("`PxMargin` has multiple children");
                margin
            }
        };
    }

    if let Some((row, children)) = row {
        let vert = row.vertical;
        let mut size = UVec2::ZERO;

        let children = if let Some(children) = children {
            &**children
        } else {
            &[]
        };

        *dim_mut(&mut size, vert) += children.len().saturating_sub(1) as u32 * row.space_between;

        for &entry in children {
            let min_size = calc_min_size(entry, uis.as_readonly(), typefaces, sprites);

            *dim_mut(&mut size, vert) += dim(min_size, vert);

            let cross_size = dim(min_size, !vert);
            if cross_size > dim(size, !vert) {
                *dim_mut(&mut size, !vert) = cross_size;
            }
        }

        return size;
    }

    if let Some((grid, children)) = grid {
        let mut column_widths = vec![0; grid.width as usize];
        let children = if let Some(children) = children {
            &**children
        } else {
            &[]
        };
        let mut height = (children.len() as u32)
            .div_ceil(grid.width)
            .saturating_sub(1)
            * grid.rows.space_between;

        for row in children.chunks(grid.width as usize) {
            let mut row_height = 0;

            for (column, &entry) in row.iter().enumerate() {
                let size = calc_min_size(entry, uis.as_readonly(), typefaces, sprites);

                if size.x > column_widths[column] {
                    column_widths[column] = size.x;
                }

                if size.y > row_height {
                    row_height = size.y;
                }
            }

            height += row_height;
        }

        return uvec2(
            column_widths.into_iter().sum::<u32>()
                + grid.width.saturating_sub(1) * grid.columns.space_between,
            height,
        );
    }

    if let Some((_, children)) = stack {
        let mut size = UVec2::ZERO;

        for &entry in children.iter().flat_map(|children| &***children) {
            size = size.max(calc_min_size(entry, uis.as_readonly(), typefaces, sprites));
        }

        return size;
    }

    if let Some((scroll, _, _)) = rect {
        let Some((scroll, children)) = scroll else {
            return UVec2::ZERO;
        };

        let mut children = children.iter();

        let (mut size, bar_size) = if let Some(content) = children.next() {
            (
                calc_min_size(content, uis.as_readonly(), typefaces, sprites),
                if let Some(bar) = children.next() {
                    calc_min_size(bar, uis.as_readonly(), typefaces, sprites).max(
                        if let Some(bar_bg) = children.next() {
                            calc_min_size(bar_bg, uis.as_readonly(), typefaces, sprites)
                        } else {
                            UVec2::ZERO
                        },
                    )
                } else {
                    UVec2::ZERO
                },
            )
        } else {
            default()
        };

        if children.next().is_some() {
            warn!("`PxScroll` has more than 3 children");
        }

        let horz = scroll.horizontal;

        *dim_mut(&mut size, horz) += dim(bar_size, horz);
        let bar_main = dim(bar_size, !horz);
        if bar_main > dim(size, !horz) {
            *dim_mut(&mut size, !horz) = bar_main;
        }

        return size;
    }

    if let Some(sprite) = sprite {
        return if let Some(sprite) = sprites.get(&**sprite) {
            sprite.frame_size()
        } else {
            UVec2::ZERO
        };
    }

    if let Some(text) = text {
        let Some(typeface) = typefaces.get(&text.typeface) else {
            return UVec2::ZERO;
        };

        return uvec2(
            text.value
                .chars()
                .map(|char| {
                    if let Some(char) = typeface.characters.get(&char) {
                        char.frame_size().x + 1
                    } else if let Some(separator) = typeface.separators.get(&char) {
                        separator.width
                    } else {
                        error!(r#"character "{char}" in text isn't in typeface"#);
                        0
                    }
                })
                .sum::<u32>()
                .saturating_sub(1),
            typeface.height,
        );
    }

    unreachable!()
}

fn layout_inner<L: PxLayer>(
    target_rect: IRect,
    target_layer: &L,
    target_canvas: PxCanvas,
    ui: Entity,
    mut uis: Query<(
        AnyOf<(
            (&PxMinSize, Option<&Children>),
            (&PxMargin, Option<&Children>),
            (&PxRow, Option<&Children>),
            (&PxGrid, Option<&Children>),
            (&PxStack, Option<&Children>),
            (
                Option<(&mut PxScroll, &Children)>,
                &mut PxRect,
                &mut PxFilterLayers<L>,
            ),
            &PxSprite,
            &mut PxText,
        )>,
        Option<&mut L>,
        Option<(&mut PxPosition, &mut PxCanvas)>,
    )>,
    row_slots: Query<&PxRowSlot>,
    typefaces: &Assets<PxTypeface>,
    sprites: &Assets<PxSpriteAsset>,
) -> Result<Option<L>> {
    fn dim(vec: IVec2, y: bool) -> i32 {
        if y { vec.y } else { vec.x }
    }

    // Adds to x, subtracts from y
    fn add(augend: i32, addend: i32, y: bool) -> i32 {
        if y { augend - addend } else { augend + addend }
    }

    fn rect_size(rect: IRect, y: bool) -> i32 {
        if y { rect.height() } else { rect.width() }
    }

    let Ok(((min_size, margin, row, grid, stack, rect, sprite, text), _, _)) = uis.get(ui) else {
        return Ok(None);
    };

    if let Some((_, children)) = min_size {
        return match children.map(|children| &**children) {
            None | Some([]) => Ok(None),
            Some(&[content]) => layout_inner(
                target_rect,
                target_layer,
                target_canvas,
                content,
                uis,
                row_slots,
                typefaces,
                sprites,
            ),
            Some([_, _, ..]) => {
                warn!("`PxMinSize` has multiple children");
                Ok(None)
            }
        };
    }

    if let Some((margin, children)) = margin {
        return match children.map(|children| &**children) {
            None | Some([]) => Ok(None),
            Some(&[content]) => layout_inner(
                IRect {
                    min: target_rect.min + **margin as i32,
                    max: target_rect.max - **margin as i32,
                },
                target_layer,
                target_canvas,
                content,
                uis,
                row_slots,
                typefaces,
                sprites,
            ),
            Some([_, _, ..]) => {
                warn!("`PxMargin` has multiple children");
                Ok(None)
            }
        };
    }

    if let Some((row, children)) = row {
        fn dim_mut(vec: &mut IVec2, y: bool) -> &mut i32 {
            if y { &mut vec.y } else { &mut vec.x }
        }

        let row = row.clone();
        let children = children
            .iter()
            .flat_map(|children| &**children)
            .copied()
            .collect::<Vec<_>>();

        let vert = row.vertical;
        let mut pos = ivec2(target_rect.min.x, target_rect.max.y);
        let mut remaining_stretchers = children
            .iter()
            .map(|&entry| row_slots.get(entry).cloned().unwrap_or_default())
            .filter(|slot| slot.stretch)
            .count() as i32;
        let mut stretch_budget = rect_size(target_rect, vert)
            - dim(
                calc_min_size(ui, uis.as_readonly(), typefaces, sprites).as_ivec2(),
                vert,
            );
        let fill_size = rect_size(target_rect, !vert);

        let mut layer = None::<L>;

        for &child in &children {
            let slot = row_slots.get(child).cloned().unwrap_or_default();
            let mut size = calc_min_size(child, uis.as_readonly(), typefaces, sprites).as_ivec2();
            if slot.stretch {
                // For simplicity, we just split the extra size among the stretched entries evenly
                // instead of prioritizing the smallest. I might change this in the future.
                let extra_size = stretch_budget / remaining_stretchers;
                *dim_mut(&mut size, vert) += extra_size;
                stretch_budget -= extra_size;
                remaining_stretchers -= 1;
            }

            // if entry.fill {
            *dim_mut(&mut size, !vert) = fill_size;
            // }

            let entry_layer = if let Some(ref layer) = layer {
                layer.clone().next().unwrap_or(layer.clone())
            } else {
                target_layer.clone()
            };

            // Improvements to the layouting could make it so that most things can share layers
            if let Some(last_layer) = layout_inner(
                IRect {
                    min: ivec2(pos.x, pos.y - size.y),
                    max: ivec2(pos.x + size.x, pos.y),
                },
                &entry_layer,
                target_canvas,
                child,
                uis.reborrow(),
                row_slots.as_readonly(),
                typefaces,
                sprites,
            )? {
                layer = Some(last_layer);
            }

            *dim_mut(&mut pos, vert) = add(
                dim(pos, vert),
                dim(size, vert) + row.space_between as i32,
                vert,
            );
        }

        return Ok(layer);
    }

    if let Some((grid, children)) = grid {
        let grid = grid.clone();
        let children = children
            .iter()
            .flat_map(|children| &**children)
            .copied()
            .collect::<Vec<_>>();

        let mut column_widths = vec![0; grid.width as usize];
        let mut row_heights = vec![0; children.len().div_ceil(grid.width as usize)];

        for (row_index, row) in children.chunks(grid.width as usize).enumerate() {
            for (column, &entry) in row.iter().enumerate() {
                let size = calc_min_size(entry, uis.as_readonly(), typefaces, sprites).as_ivec2();

                if size.x > column_widths[column] {
                    column_widths[column] = size.x;
                }

                if size.y > row_heights[row_index] {
                    row_heights[row_index] = size.y;
                }
            }
        }

        let min_size = calc_min_size(ui, uis.as_readonly(), typefaces, sprites).as_ivec2();

        let mut remaining_stretching_rows =
            grid.rows.rows.iter().filter(|row| row.stretch).count() as i32;
        let mut row_stretch_budget = target_rect.height() - min_size.y;

        for (index, row) in grid.rows.rows.iter().enumerate() {
            if index >= row_heights.len() {
                continue;
            }

            if row.stretch {
                let extra_size = row_stretch_budget / remaining_stretching_rows;
                row_heights[index] += extra_size;
                row_stretch_budget -= extra_size;
                remaining_stretching_rows -= 1;
            }
        }

        let mut remaining_stretching_columns = grid
            .columns
            .rows
            .iter()
            .filter(|column| column.stretch)
            .count() as i32;
        let mut column_stretch_budget = target_rect.width() - min_size.x;

        for (index, column) in grid.columns.rows.iter().enumerate() {
            if index >= column_widths.len() {
                continue;
            }

            if column.stretch {
                let extra_size = column_stretch_budget / remaining_stretching_columns;
                column_widths[index] += extra_size;
                column_stretch_budget -= extra_size;
                remaining_stretching_columns -= 1;
            }
        }

        let mut y_pos = target_rect.max.y;

        let mut layer = None::<L>;

        for (row_index, row) in children.chunks(grid.width as usize).enumerate() {
            let mut x_pos = target_rect.min.x;
            let height = row_heights[row_index];

            for (column, &entry) in row.iter().enumerate() {
                let width = column_widths[column];

                let entry_layer = if let Some(ref layer) = layer {
                    layer.clone().next().unwrap_or(layer.clone())
                } else {
                    target_layer.clone()
                };

                if let Some(last_layer) = layout_inner(
                    IRect {
                        min: ivec2(x_pos, y_pos - height),
                        max: ivec2(x_pos + width, y_pos),
                    },
                    &entry_layer,
                    target_canvas,
                    entry,
                    uis.reborrow(),
                    row_slots.as_readonly(),
                    typefaces,
                    sprites,
                )? {
                    layer = Some(last_layer);
                }

                x_pos += width + grid.columns.space_between as i32;
            }

            y_pos -= height + grid.columns.space_between as i32;
        }

        return Ok(layer);
    }

    if let Some((_, children)) = stack {
        let children = children
            .iter()
            .flat_map(|children| &**children)
            .copied()
            .collect::<Vec<_>>();

        let mut layer = None::<L>;

        for &entry in &children {
            let entry_layer = if let Some(ref layer) = layer {
                layer.clone().next().unwrap_or(layer.clone())
            } else {
                target_layer.clone()
            };

            if let Some(last_layer) = layout_inner(
                target_rect,
                &entry_layer,
                target_canvas,
                entry,
                uis.reborrow(),
                row_slots.as_readonly(),
                typefaces,
                sprites,
            )? {
                layer = Some(last_layer);
            }
        }

        return Ok(layer);
    }

    if rect.is_some() {
        let ((_, _, _, _, _, rect, _, _), _, mut pos) = uis.get_mut(ui).unwrap();

        if let Some((_, ref mut canvas)) = pos {
            **canvas = target_canvas;
        }

        let (scroll, mut rect, mut layers) = rect.unwrap();

        if let Some((scroll, children)) = scroll {
            fn rect_start(rect: IRect, y: bool) -> i32 {
                if y { rect.max.y } else { rect.min.x }
            }

            fn rect_start_mut(rect: &mut IRect, y: bool) -> &mut i32 {
                if y { &mut rect.max.y } else { &mut rect.min.x }
            }

            fn rect_end(rect: IRect, y: bool) -> i32 {
                if y { rect.min.y } else { rect.max.x }
            }

            fn rect_end_mut(rect: &mut IRect, y: bool) -> &mut i32 {
                if y { &mut rect.min.y } else { &mut rect.max.x }
            }

            let scroll = *scroll;
            let content = children[0];
            let bar = children.get(1).copied();
            let bg = children.get(2).copied();
            if children.get(3).is_some() {
                warn!("`PxScroll` has more than 3 children");
                return Ok(None);
            }
            let horz = scroll.horizontal;

            let content_min_size =
                calc_min_size(content, uis.as_readonly(), typefaces, sprites).as_ivec2();

            let bar_min_size = if let Some(bar) = bar {
                calc_min_size(bar, uis.as_readonly(), typefaces, sprites).max(
                    if let Some(bg) = bg {
                        calc_min_size(bg, uis.as_readonly(), typefaces, sprites)
                    } else {
                        UVec2::ZERO
                    },
                )
            } else {
                UVec2::ZERO
            }
            .as_ivec2();

            let mut view_rect = target_rect;
            *rect_end_mut(&mut view_rect, horz) =
                add(rect_end(view_rect, horz), -dim(bar_min_size, horz), horz);

            let ((_, _, _, _, _, rect, _, _), _, pos) = uis.get_mut(ui).unwrap();
            let (_, mut rect, _) = rect.unwrap();
            **rect = view_rect.size().as_uvec2();
            if let Some((mut pos, _)) = pos {
                **pos = view_rect.center();
            }

            let mut content_rect = view_rect;
            *rect_start_mut(&mut content_rect, !horz) = add(
                rect_start(content_rect, !horz),
                -(scroll.scroll as i32),
                !horz,
            );
            *rect_end_mut(&mut content_rect, !horz) = add(
                rect_start(content_rect, !horz),
                dim(content_min_size, !horz),
                !horz,
            );

            let mut layer = None;

            // TODO Need to make containers with multiple entries put entries beyond the first on
            // different layers
            let last_content_layer = layout_inner(
                content_rect,
                target_layer,
                target_canvas,
                content,
                uis.reborrow(),
                row_slots.as_readonly(),
                typefaces,
                sprites,
            )?;

            let ((_, _, _, _, _, rect, _, _), _, _) = uis.get_mut(ui).unwrap();
            let (_, _, mut layers) = rect.unwrap();

            let bg_layer;
            (*layers, bg_layer) = if let Some(last_content_layer) = last_content_layer {
                layer = Some(last_content_layer.clone());

                (
                    PxFilterLayers::Range(target_layer.clone()..=last_content_layer.clone()),
                    last_content_layer
                        .clone()
                        .next()
                        .unwrap_or(last_content_layer),
                )
            } else {
                (PxFilterLayers::Many(Vec::new()), target_layer.clone())
            };

            let mut bar_rect = target_rect;
            *rect_start_mut(&mut bar_rect, horz) = rect_end(view_rect, horz);

            let last_bg_layer = bg
                .map(|bg| {
                    layout_inner(
                        bar_rect,
                        &bg_layer,
                        target_canvas,
                        bg,
                        uis.reborrow(),
                        row_slots.as_readonly(),
                        typefaces,
                        sprites,
                    )
                })
                .transpose()?
                .flatten();
            let bar_layer = if let Some(last_bg_layer) = last_bg_layer {
                layer = Some(last_bg_layer.clone());
                last_bg_layer.clone().next().unwrap_or(last_bg_layer)
            } else {
                bg_layer.clone()
            };

            let content_size = rect_size(content_rect, !horz);
            let view_size = rect_size(view_rect, !horz);
            let ratio = if content_size == 0 {
                0.
            } else {
                view_size as f32 / content_size as f32
            };
            *rect_start_mut(&mut bar_rect, !horz) = add(
                rect_start(view_rect, !horz),
                (scroll.scroll as f32 * ratio) as i32,
                !horz,
            );
            *rect_end_mut(&mut bar_rect, !horz) = add(
                rect_start(view_rect, !horz),
                ((view_size + scroll.scroll as i32) as f32 * ratio) as i32,
                !horz,
            );

            let ((_, _, _, _, _, rect, _, _), _, _) = uis.get_mut(ui).unwrap();
            let (scroll, _, _) = rect.unwrap();
            let (mut scroll, _) = scroll.unwrap();

            let new_max_scroll = (view_size as f32 * (1. / ratio - 1.)).ceil() as u32;
            if scroll.max_scroll != new_max_scroll {
                scroll.max_scroll = new_max_scroll;
            }

            if let Some(last_bar_layer) = bar
                .map(|bar| {
                    layout_inner(
                        bar_rect,
                        &bar_layer,
                        target_canvas,
                        bar,
                        uis.reborrow(),
                        row_slots.as_readonly(),
                        typefaces,
                        sprites,
                    )
                })
                .transpose()?
                .flatten()
            {
                layer = Some(last_bar_layer);
            }

            return Ok(layer);
        }

        if let Some((mut pos, _)) = pos {
            **pos = target_rect.center();
        }

        let rect_layer = target_layer.clone();
        match *layers {
            PxFilterLayers::Single { ref mut layer, .. } => *layer = rect_layer,
            PxFilterLayers::Range(ref mut layers) => {
                *layers = layers.start().clone()..=rect_layer;
            }
            ref mut layers @ PxFilterLayers::Many(_) => {
                *layers = PxFilterLayers::single_over(rect_layer);
            }
        }

        **rect = target_rect.size().as_uvec2();

        return Ok(Some(target_layer.clone()));
    }

    if sprite.is_some() {
        let (_, layer, pos) = uis.get_mut(ui).unwrap();

        if let Some((mut pos, mut canvas)) = pos {
            **pos = target_rect.center();
            *canvas = target_canvas;
        }

        if let Some(mut layer) = layer {
            *layer = target_layer.clone();
        }

        return Ok(Some(target_layer.clone()));
    }

    if text.is_some() {
        let ((_, _, _, _, _, _, _, text), layer, pos) = uis.get_mut(ui).unwrap();

        if let Some(mut layer) = layer {
            *layer = target_layer.clone();
        }

        let Some((mut pos, mut canvas)) = pos else {
            return Ok(Some(target_layer.clone()));
        };

        *canvas = target_canvas;

        let mut text = text.unwrap();
        let PxText {
            ref mut value,
            ref typeface,
            ref mut line_breaks,
        } = *text;

        let Some(typeface) = typefaces.get(typeface) else {
            return Ok(Some(target_layer.clone()));
        };

        let mut new_line_breaks = Vec::new();

        let max_width = target_rect.width();
        let mut x = 0;
        let mut max_x = 0;
        let mut last_separator = None;

        for (index, char) in value.chars().enumerate() {
            let index = index as u32;

            if let Some(char) = typeface.characters.get(&char) {
                let split = x > max_width;
                if split {
                    x = 0;
                    new_line_breaks.push(last_separator.unwrap_or(index.saturating_sub(1)));
                    last_separator = None;
                }

                let width = char.frame_size().x as i32;

                if x != 0 {
                    x += 1;
                }
                x += width;

                if x > max_width && !split {
                    x = width;
                    new_line_breaks.push(last_separator.unwrap_or(index.saturating_sub(1)));
                    last_separator = None;
                }

                if x > max_x {
                    max_x = x;
                }
            } else if let Some(separator) = typeface.separators.get(&char) {
                x += separator.width as i32;
                last_separator = Some(index);
            } else {
                error!(r#"character "{char}" in text isn't in typeface"#);
            }
        }

        if *line_breaks != new_line_breaks {
            *line_breaks = new_line_breaks;
        }

        let line_break_count = line_breaks.len() as i32;
        **pos = ivec2(target_rect.min.x, target_rect.max.y)
            + ivec2(
                max_x,
                -((line_break_count + 1) * typeface.height as i32 + line_break_count),
            ) / 2;

        return Ok(Some(target_layer.clone()));
    }

    unreachable!();
}

pub(crate) fn layout<L: PxLayer>(
    mut uis: ParamSet<(
        Query<(&L, &PxCanvas, Entity), With<PxUiRoot>>,
        Query<(
            AnyOf<(
                (&PxMinSize, Option<&Children>),
                (&PxMargin, Option<&Children>),
                (&PxRow, Option<&Children>),
                (&PxGrid, Option<&Children>),
                (&PxStack, Option<&Children>),
                (
                    Option<(&mut PxScroll, &Children)>,
                    &mut PxRect,
                    &mut PxFilterLayers<L>,
                ),
                &PxSprite,
                &mut PxText,
            )>,
            Option<&mut L>,
            Option<(&mut PxPosition, &mut PxCanvas)>,
        )>,
    )>,
    row_slots: Query<&PxRowSlot>,
    typefaces: Res<Assets<PxTypeface>>,
    sprites: Res<Assets<PxSpriteAsset>>,
    screen: Res<Screen>,
) -> Result {
    let _layout_span = px_trace_span!(
        "carapace::ui::layout",
        width = screen.computed_size.x,
        height = screen.computed_size.y
    );
    let roots = uis
        .p0()
        .iter()
        .map(|(layer, &canvas, entity)| (layer.clone(), canvas, entity))
        .collect::<Vec<_>>();
    px_trace!(root_count = roots.len(), "carapace::ui::layout_roots");

    for (layer, canvas, root) in roots {
        layout_inner(
            IRect {
                min: IVec2::ZERO,
                max: screen.computed_size.as_ivec2(),
            },
            &layer,
            canvas,
            root,
            uis.p1(),
            row_slots.as_readonly(),
            &typefaces,
            &sprites,
        )?;
    }
    px_end_span!(_layout_span);

    OK
}

pub(crate) fn layout_needs_recompute(
    roots: Query<(), With<PxUiRoot>>,
    changed_structure: Query<
        (),
        Or<(
            Changed<PxUiRoot>,
            Changed<Children>,
            Changed<PxMinSize>,
            Changed<PxMargin>,
            Changed<PxRow>,
            Changed<PxGrid>,
            Changed<PxStack>,
            Changed<PxRowSlot>,
        )>,
    >,
    changed_content: Query<(), Or<(Changed<PxScroll>, Changed<PxSprite>, Changed<PxText>)>>,
    typefaces: Option<Res<Assets<PxTypeface>>>,
    sprites: Option<Res<Assets<PxSpriteAsset>>>,
    screen: Option<Res<Screen>>,
) -> bool {
    if roots.is_empty() {
        return false;
    }

    changed_structure.iter().next().is_some()
        || changed_content.iter().next().is_some()
        || typefaces.is_some_and(|assets| assets.is_changed())
        || sprites.is_some_and(|assets| assets.is_changed())
        || screen.is_some_and(|screen| screen.is_changed())
}
