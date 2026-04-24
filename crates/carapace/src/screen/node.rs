#[cfg(feature = "gpu_palette")]
use std::collections::BTreeSet;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

#[cfg(feature = "gpu_palette")]
use bevy_ecs::query::Has;
use bevy_ecs::query::QueryState;
use bevy_image::TextureFormatPixelInfo;
#[cfg(feature = "headed")]
use bevy_render::{
    render_graph::{NodeRunError, RenderGraphContext, ViewNode},
    render_resource::{
        BindGroupEntries, PipelineCache, RenderPassColorAttachment, RenderPassDescriptor,
        TexelCopyBufferLayout, TextureViewDescriptor, TextureViewDimension,
    },
    renderer::{RenderContext, RenderDevice, RenderQueue},
    view::ViewTarget,
};

#[cfg(feature = "line")]
use crate::line::LineComponents;
#[cfg(feature = "gpu_palette")]
use crate::sprite::{CxGpuComposite, CxGpuSprite};
use crate::{
    atlas::AtlasSpriteComponents,
    filter::FilterComponents,
    position::CxLayer,
    prelude::*,
    profiling::{px_end_span, px_trace, px_trace_span},
    rect::RectComponents,
    sprite::{CompositeSpriteComponents, SpriteComponents},
    text::TextComponents,
    tilemap::{MapComponents, TileComponents},
};

#[cfg(feature = "gpu_palette")]
use super::{CxLayerOrder, gpu_composite_supported, gpu_sprite_supported};
use super::{
    CxScreen,
    draw::{self, LayerContentsMap},
    pipeline::{CxPipeline, CxRenderBuffer, CxUniformBuffer},
};

static EXACT_SCREENSHOT_WRITTEN: AtomicBool = AtomicBool::new(false);
const EXACT_SCREENSHOT_ENV: &str = "CARAPACE_EXACT_SCREENSHOT_PATH";

/// Resolves filter layer targets against a pre-built ordered layer index.
///
/// Uses binary search (`partition_point`) for `Range` variants, giving `O(log L + K)` cost
/// instead of the previous `O(L)` full-scan approach. The `ordered_layers` slice must be
/// sorted (which it is, since it comes from `BTreeMap::keys`).
fn resolve_filter_layers<L: CxLayer>(
    out: &mut Vec<(L, bool)>,
    layers: &CxFilterLayers<L>,
    ordered_layers: &[L],
) {
    out.clear();
    match layers {
        CxFilterLayers::Single { layer, clip } => out.push((layer.clone(), *clip)),
        CxFilterLayers::Range(range) => {
            let start = ordered_layers.partition_point(|l| l < range.start());
            let end = start + ordered_layers[start..].partition_point(|l| l <= range.end());
            out.extend(
                ordered_layers[start..end]
                    .iter()
                    .map(|layer| (layer.clone(), true)),
            );
        }
        CxFilterLayers::Many(layers) => {
            out.extend(layers.iter().map(|layer| (layer.clone(), true)));
        }
    }
}

/// Pre-registers layers referenced by `Single` and `Many` filter targets so they are present
/// in the layer map before the ordered index is built. `Range` targets are not pre-registered
/// because they resolve against the discovered set.
fn preregister_filter_layer<L: CxLayer>(
    layers: &CxFilterLayers<L>,
    layer_contents: &mut LayerContentsMap<'_, L>,
) {
    match layers {
        CxFilterLayers::Single { layer, .. } => {
            layer_contents.entry(layer.clone()).or_default();
        }
        CxFilterLayers::Many(ls) => {
            for l in ls {
                layer_contents.entry(l.clone()).or_default();
            }
        }
        CxFilterLayers::Range(_) => {}
    }
}

pub(crate) struct CxRenderNode<L: CxLayer> {
    maps: QueryState<MapComponents<L>>,
    tiles: QueryState<TileComponents>,
    // image_to_sprites: QueryState<ImageToSpriteComponents<L>>,
    #[cfg(feature = "gpu_palette")]
    sprites: QueryState<(SpriteComponents<L>, Has<CxGpuSprite>)>,
    #[cfg(not(feature = "gpu_palette"))]
    sprites: QueryState<SpriteComponents<L>>,
    atlas_sprites: QueryState<AtlasSpriteComponents<L>>,
    #[cfg(feature = "gpu_palette")]
    composites: QueryState<(CompositeSpriteComponents<L>, Has<CxGpuComposite>)>,
    #[cfg(not(feature = "gpu_palette"))]
    composites: QueryState<CompositeSpriteComponents<L>>,
    texts: QueryState<TextComponents<L>>,
    primitives: QueryState<crate::primitive::PrimitiveComponents<L>>,
    rects: QueryState<RectComponents<L>>,
    #[cfg(feature = "line")]
    lines: QueryState<LineComponents<L>>,
    filters: QueryState<FilterComponents<L>, Without<CxRenderSpace>>,
}

impl<L: CxLayer> FromWorld for CxRenderNode<L> {
    fn from_world(world: &mut World) -> Self {
        Self {
            maps: world.query(),
            tiles: world.query(),
            // image_to_sprites: world.query(),
            #[cfg(feature = "gpu_palette")]
            sprites: world.query(),
            #[cfg(not(feature = "gpu_palette"))]
            sprites: world.query(),
            atlas_sprites: world.query(),
            #[cfg(feature = "gpu_palette")]
            composites: world.query(),
            #[cfg(not(feature = "gpu_palette"))]
            composites: world.query(),
            texts: world.query(),
            primitives: world.query(),
            rects: world.query(),
            #[cfg(feature = "line")]
            lines: world.query(),
            filters: world.query_filtered(),
        }
    }
}

#[cfg(feature = "headed")]
impl<L: CxLayer> ViewNode for CxRenderNode<L> {
    type ViewQuery = (&'static ViewTarget, Has<crate::screen::CxOverlayCamera>);

    fn update(&mut self, world: &mut World) {
        self.maps.update_archetypes(world);
        self.tiles.update_archetypes(world);
        // self.image_to_sprites.update_archetypes(world);
        self.sprites.update_archetypes(world);
        self.atlas_sprites.update_archetypes(world);
        self.composites.update_archetypes(world);
        self.texts.update_archetypes(world);
        self.primitives.update_archetypes(world);
        self.rects.update_archetypes(world);
        #[cfg(feature = "line")]
        self.lines.update_archetypes(world);
        self.filters.update_archetypes(world);
    }

    fn run<'w>(
        &self,
        _: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        (target, is_overlay): (&ViewTarget, bool),
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        // Skip rendering on overlay cameras so gizmos drawn on them remain visible.
        if is_overlay {
            return Ok(());
        }
        // Compose each layer into a CPU buffer, then blit to the GPU texture once per frame.
        let &camera = world.resource::<CxCamera>();
        let screen = world.resource::<CxScreen>();
        let _run_span = px_trace_span!(
            "carapace::screen_node::run",
            width = screen.computed_size.x,
            height = screen.computed_size.y
        );

        let device = world.resource::<RenderDevice>();
        let render_buffer = world.resource::<CxRenderBuffer>();
        render_buffer.ensure_size(device, screen.computed_size);
        render_buffer.clear();

        let mut layer_contents: LayerContentsMap<'w, L> = BTreeMap::default();
        #[cfg(feature = "gpu_palette")]
        let mut layer_set: BTreeSet<L> = BTreeSet::new();
        let _collect_span = px_trace_span!("carapace::screen_node::collect");

        for (map, &pos, layer, &canvas, animation, filter) in self.maps.iter_manual(world) {
            let map = (map, pos, canvas, animation, filter);
            #[cfg(feature = "gpu_palette")]
            {
                layer_set.insert(layer.clone());
            }
            layer_contents
                .entry(layer.clone())
                .or_default()
                .maps
                .push(map);
        }

        for (prim, &position, &anchor, layer, &canvas, presentation) in
            self.primitives.iter_manual(world)
        {
            #[cfg(feature = "gpu_palette")]
            {
                layer_set.insert(layer.clone());
            }
            layer_contents
                .entry(layer.clone())
                .or_default()
                .primitives
                .push((prim, position, anchor, canvas, presentation.copied()));
        }

        #[cfg(feature = "gpu_palette")]
        for (
            (sprite, &position, &anchor, layer, &canvas, animation, filter, presentation),
            has_gpu,
        ) in self.sprites.iter_manual(world)
        {
            layer_set.insert(layer.clone());
            let gpu_eligible = has_gpu && gpu_sprite_supported(animation.copied(), filter);
            if gpu_eligible {
                continue;
            }

            let sprite = (
                sprite,
                position,
                anchor,
                canvas,
                animation,
                filter,
                presentation.copied(),
            );
            layer_contents
                .entry(layer.clone())
                .or_default()
                .sprites
                .push(sprite);
        }

        #[cfg(not(feature = "gpu_palette"))]
        for (sprite, &position, &anchor, layer, &canvas, animation, filter, presentation) in
            self.sprites.iter_manual(world)
        {
            let sprite = (
                sprite,
                position,
                anchor,
                canvas,
                animation,
                filter,
                presentation.copied(),
            );
            layer_contents
                .entry(layer.clone())
                .or_default()
                .sprites
                .push(sprite);
        }

        for (sprite, &position, &anchor, layer, &canvas, animation, filter, presentation) in
            self.atlas_sprites.iter_manual(world)
        {
            let sprite = (
                sprite,
                position,
                anchor,
                canvas,
                animation,
                filter,
                presentation.copied(),
            );
            #[cfg(feature = "gpu_palette")]
            {
                layer_set.insert(layer.clone());
            }
            layer_contents
                .entry(layer.clone())
                .or_default()
                .atlas_sprites
                .push(sprite);
        }

        #[cfg(feature = "gpu_palette")]
        for (
            (composite, &position, &anchor, layer, &canvas, animation, filter, presentation),
            has_gpu,
        ) in self.composites.iter_manual(world)
        {
            layer_set.insert(layer.clone());
            let gpu_eligible =
                has_gpu && gpu_composite_supported(composite, animation.copied(), filter);
            if gpu_eligible {
                continue;
            }

            let composite = (
                composite,
                position,
                anchor,
                canvas,
                animation,
                filter,
                presentation.copied(),
            );
            layer_contents
                .entry(layer.clone())
                .or_default()
                .composites
                .push(composite);
        }

        #[cfg(not(feature = "gpu_palette"))]
        for (composite, &position, &anchor, layer, &canvas, animation, filter, presentation) in
            self.composites.iter_manual(world)
        {
            let composite = (
                composite,
                position,
                anchor,
                canvas,
                animation,
                filter,
                presentation.copied(),
            );
            layer_contents
                .entry(layer.clone())
                .or_default()
                .composites
                .push(composite);
        }

        for (text, &pos, &alignment, layer, &canvas, animation, filter) in
            self.texts.iter_manual(world)
        {
            let text = (text, pos, alignment, canvas, animation, filter);
            #[cfg(feature = "gpu_palette")]
            {
                layer_set.insert(layer.clone());
            }
            layer_contents
                .entry(layer.clone())
                .or_default()
                .texts
                .push(text);
        }

        // === Two-phase filter extraction ===
        // Phase 1: buffer rect/line/filter entities and pre-register their explicit layer targets.
        // This ensures Range resolution in phase 2 sees the complete set of layers.
        let pending_rects: Vec<_> = self
            .rects
            .iter_manual(world)
            .map(
                |(&rect, filter, layers, &pos, &anchor, &canvas, animation, invert)| {
                    preregister_filter_layer(layers, &mut layer_contents);
                    (
                        (rect, filter, pos, anchor, canvas, animation, invert),
                        layers,
                    )
                },
            )
            .collect();

        #[cfg(feature = "line")]
        let pending_lines: Vec<_> = self
            .lines
            .iter_manual(world)
            .map(|(line, filter, layers, &canvas, animation, invert)| {
                preregister_filter_layer(layers, &mut layer_contents);
                ((line, filter, canvas, animation, invert), layers)
            })
            .collect();

        let pending_filters: Vec<_> = self
            .filters
            .iter_manual(world)
            .map(|(filter, layers, animation)| {
                preregister_filter_layer(layers, &mut layer_contents);
                ((filter, animation), layers)
            })
            .collect();

        // Phase 2: build ordered layer index and resolve pending filters.
        let ordered_layers: Vec<L> = layer_contents.keys().cloned().collect();
        let mut resolved_filter_layers = Vec::with_capacity(8);

        for (rect, layers) in pending_rects {
            resolve_filter_layers(&mut resolved_filter_layers, layers, &ordered_layers);

            for (layer, clip) in &resolved_filter_layers {
                #[cfg(feature = "gpu_palette")]
                {
                    layer_set.insert(layer.clone());
                }
                layer_contents
                    .get_mut(layer)
                    .unwrap()
                    .push_rect(rect, *clip);
            }
        }

        #[cfg(feature = "line")]
        for (line, layers) in pending_lines {
            resolve_filter_layers(&mut resolved_filter_layers, layers, &ordered_layers);

            for (layer, clip) in &resolved_filter_layers {
                #[cfg(feature = "gpu_palette")]
                {
                    layer_set.insert(layer.clone());
                }
                layer_contents
                    .get_mut(layer)
                    .unwrap()
                    .push_line(line, *clip);
            }
        }

        for (filter, layers) in pending_filters {
            resolve_filter_layers(&mut resolved_filter_layers, layers, &ordered_layers);

            for (layer, clip) in &resolved_filter_layers {
                #[cfg(feature = "gpu_palette")]
                {
                    layer_set.insert(layer.clone());
                }
                layer_contents
                    .get_mut(layer)
                    .unwrap()
                    .push_filter(filter, *clip);
            }
        }
        px_end_span!(_collect_span);

        #[cfg(feature = "gpu_palette")]
        let layer_order: Vec<L> = layer_set.into_iter().collect();
        #[cfg(feature = "gpu_palette")]
        let layer_order_res = world.resource::<CxLayerOrder<L>>();
        #[cfg(feature = "gpu_palette")]
        layer_order_res.set(layer_order);
        #[cfg(feature = "gpu_palette")]
        let layer_order = layer_order_res.read();
        px_trace!(
            layer_count = layer_contents.len(),
            "carapace::screen_node::draw"
        );

        {
            let _draw_span = px_trace_span!("carapace::screen_node::draw_layers");
            #[cfg(feature = "gpu_palette")]
            draw::draw_layers(
                world,
                render_buffer,
                camera,
                layer_contents,
                &self.tiles,
                &layer_order,
            );
            #[cfg(not(feature = "gpu_palette"))]
            draw::draw_layers(world, render_buffer, camera, layer_contents, &self.tiles);
        }

        let Some(uniform_binding) = world.resource::<CxUniformBuffer>().binding() else {
            return Ok(());
        };

        let _upload_span = px_trace_span!("carapace::screen_node::upload");
        let inner = render_buffer.read_inner();
        let texture = inner.texture.as_ref().unwrap();
        let image = inner.image.as_ref().unwrap();
        let image_descriptor = image.texture_descriptor.clone();

        let Ok(pixel_size) = image_descriptor.format.pixel_size() else {
            return Ok(());
        };

        world.resource::<RenderQueue>().write_texture(
            texture.as_image_copy(),
            image.data.as_ref().unwrap(),
            TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(image.width() * pixel_size as u32),
                rows_per_image: None,
            },
            image_descriptor.size,
        );
        write_exact_screenshot_if_requested(image, world.resource::<CxScreen>());

        #[cfg(feature = "gpu_palette")]
        if let Some(depth_image) = inner.depth_image.as_ref()
            && let Some(depth_data) = depth_image.data.as_ref()
            && let Some(depth_texture) = inner.depth_texture.as_ref()
        {
            let depth_descriptor = depth_image.texture_descriptor.clone();
            if let Ok(depth_pixel_size) = depth_descriptor.format.pixel_size() {
                world.resource::<RenderQueue>().write_texture(
                    depth_texture.as_image_copy(),
                    depth_data,
                    TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(depth_image.width() * depth_pixel_size as u32),
                        rows_per_image: None,
                    },
                    depth_descriptor.size,
                );
            }
        }
        px_end_span!(_upload_span);

        let _present_span = px_trace_span!("carapace::screen_node::present");
        let texture_view = texture.create_view(&TextureViewDescriptor {
            label: Some("px_texture_view"),
            format: Some(image_descriptor.format),
            dimension: Some(TextureViewDimension::D2),
            ..default()
        });

        let px_pipeline = world.resource::<CxPipeline>();
        let Some(pipeline) = world
            .resource::<PipelineCache>()
            .get_render_pipeline(px_pipeline.id)
        else {
            return Ok(());
        };

        let post_process = target.post_process_write();

        let bind_group = render_context.render_device().create_bind_group(
            "px_bind_group",
            &px_pipeline.layout,
            &BindGroupEntries::sequential((&texture_view, uniform_binding.clone())),
        );

        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("px_pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: post_process.destination,
                depth_slice: None,
                resolve_target: None,
                ops: default(),
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_render_pipeline(pipeline);
        render_pass.set_bind_group(0, &bind_group, &[]);
        render_pass.draw(0..6, 0..1);
        px_end_span!(_present_span);

        Ok(())
    }
}

fn write_exact_screenshot_if_requested(image: &Image, screen: &CxScreen) {
    let Some(path) = std::env::var_os(EXACT_SCREENSHOT_ENV).map(PathBuf::from) else {
        return;
    };

    if EXACT_SCREENSHOT_WRITTEN.swap(true, Ordering::SeqCst) {
        return;
    }

    if let Err(err) = write_exact_screenshot(&path, screen.computed_size, image, &screen.palette) {
        warn!(
            "failed to write exact screenshot to {}: {err}",
            path.display()
        );
        EXACT_SCREENSHOT_WRITTEN.store(false, Ordering::SeqCst);
    }
}

fn write_exact_screenshot(
    path: &Path,
    size: UVec2,
    image: &Image,
    palette: &[Vec3; 256],
) -> Result<(), String> {
    let data = image
        .data
        .as_ref()
        .ok_or_else(|| "render image has no CPU buffer".to_string())?;

    let expected_len = size.x as usize * size.y as usize;
    if data.len() != expected_len {
        return Err(format!(
            "render image size mismatch: expected {expected_len} bytes, got {}",
            data.len()
        ));
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }

    let mut rgba = Vec::with_capacity(expected_len * 4);
    for &index in data {
        let [r, g, b] = linear_palette_to_srgb_u8(palette[index as usize]);
        rgba.extend_from_slice(&[r, g, b, 255]);
    }

    let png = ::image::RgbaImage::from_raw(size.x, size.y, rgba)
        .ok_or_else(|| "failed to assemble RGBA image".to_string())?;
    png.save(path).map_err(|err| err.to_string())
}

fn linear_palette_to_srgb_u8(color: Vec3) -> [u8; 3] {
    [
        linear_channel_to_srgb_u8(color.x),
        linear_channel_to_srgb_u8(color.y),
        linear_channel_to_srgb_u8(color.z),
    ]
}

fn linear_channel_to_srgb_u8(channel: f32) -> u8 {
    let channel = channel.clamp(0.0, 1.0);
    let srgb = if channel <= 0.003_130_8 {
        channel * 12.92
    } else {
        1.055 * channel.powf(1.0 / 2.4) - 0.055
    };

    (srgb * 255.0).round().clamp(0.0, 255.0) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg_attr(
        feature = "headed",
        derive(bevy_render::extract_component::ExtractComponent)
    )]
    #[derive(Component, next::Next, Ord, PartialOrd, Eq, PartialEq, Clone, Default, Debug)]
    #[next(path = next::Next)]
    enum TestLayer {
        Back,
        #[default]
        Mid,
        Front,
    }

    /// Richer layer enum for testing binary search with gaps.
    #[cfg_attr(
        feature = "headed",
        derive(bevy_render::extract_component::ExtractComponent)
    )]
    #[derive(Component, next::Next, Ord, PartialOrd, Eq, PartialEq, Clone, Default, Debug)]
    #[next(path = next::Next)]
    enum WideLayer {
        #[default]
        L0,
        L1,
        L2,
        L3,
        L4,
        L5,
        L6,
        L7,
    }

    #[test]
    fn resolve_single_preserves_clip() {
        let mut out = Vec::new();
        resolve_filter_layers(
            &mut out,
            &CxFilterLayers::Single {
                layer: TestLayer::Mid,
                clip: false,
            },
            &[],
        );
        assert_eq!(out, vec![(TestLayer::Mid, false)]);
    }

    #[test]
    fn resolve_range_subset() {
        let mut out = Vec::new();
        resolve_filter_layers(
            &mut out,
            &CxFilterLayers::Range(TestLayer::Mid..=TestLayer::Front),
            &[TestLayer::Back, TestLayer::Mid, TestLayer::Front],
        );
        assert_eq!(out, vec![(TestLayer::Mid, true), (TestLayer::Front, true)]);
    }

    #[test]
    fn resolve_range_empty_when_no_layers_in_range() {
        let mut out = Vec::new();
        resolve_filter_layers(
            &mut out,
            &CxFilterLayers::Range(TestLayer::Mid..=TestLayer::Front),
            &[TestLayer::Back],
        );
        assert!(out.is_empty());
    }

    #[test]
    fn resolve_range_sparse() {
        // Range boundaries (L2, L6) don't exist in the ordered set — tests that
        // partition_point correctly handles gaps without leaking adjacent layers.
        let mut out = Vec::new();
        resolve_filter_layers(
            &mut out,
            &CxFilterLayers::Range(WideLayer::L2..=WideLayer::L6),
            &[WideLayer::L1, WideLayer::L3, WideLayer::L5, WideLayer::L7],
        );
        assert_eq!(out, vec![(WideLayer::L3, true), (WideLayer::L5, true)]);
    }

    #[test]
    fn resolve_many_keeps_declared_order() {
        let mut out = Vec::new();
        resolve_filter_layers(
            &mut out,
            &CxFilterLayers::Many(vec![TestLayer::Front, TestLayer::Back, TestLayer::Mid]),
            &[],
        );
        assert_eq!(
            out,
            vec![
                (TestLayer::Front, true),
                (TestLayer::Back, true),
                (TestLayer::Mid, true)
            ]
        );
    }

    #[test]
    fn resolve_clears_previous_results() {
        let mut out = Vec::new();
        resolve_filter_layers(
            &mut out,
            &CxFilterLayers::Range(TestLayer::Back..=TestLayer::Front),
            &[TestLayer::Back, TestLayer::Mid, TestLayer::Front],
        );
        assert_eq!(out.len(), 3);

        // Second call must not retain stale data from the first.
        resolve_filter_layers(
            &mut out,
            &CxFilterLayers::Single {
                layer: TestLayer::Mid,
                clip: false,
            },
            &[],
        );
        assert_eq!(out, vec![(TestLayer::Mid, false)]);
    }

    #[test]
    fn two_phase_range_sees_preregistered_layers() {
        // The core correctness test: a Range must include layers introduced by
        // Single and Many pre-registration, not just base content.
        let mut lc: LayerContentsMap<'static, WideLayer> = BTreeMap::new();

        // Base content on L0 and L7 only.
        lc.insert(WideLayer::L0, draw::LayerContents::default());
        lc.insert(WideLayer::L7, draw::LayerContents::default());

        // A Single target introduces L2.
        preregister_filter_layer(
            &CxFilterLayers::Single {
                layer: WideLayer::L2,
                clip: true,
            },
            &mut lc,
        );

        // A Many target introduces L4 and L6.
        preregister_filter_layer(
            &CxFilterLayers::Many(vec![WideLayer::L4, WideLayer::L6]),
            &mut lc,
        );

        // A Range pre-registers nothing (by design).
        let range = CxFilterLayers::Range(WideLayer::L1..=WideLayer::L5);
        preregister_filter_layer(&range, &mut lc);

        // Build ordered index after all pre-registration.
        let ordered: Vec<WideLayer> = lc.keys().cloned().collect();
        let mut out = Vec::new();
        resolve_filter_layers(&mut out, &range, &ordered);

        // Range(L1..=L5) should find L2 (from Single) and L4 (from Many),
        // but not L0, L6, or L7.
        assert_eq!(out, vec![(WideLayer::L2, true), (WideLayer::L4, true)]);
    }
}
