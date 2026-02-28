use std::collections::BTreeMap;
#[cfg(feature = "gpu_palette")]
use std::collections::BTreeSet;

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
use crate::sprite::{PxGpuComposite, PxGpuSprite};
use crate::{
    atlas::AtlasSpriteComponents,
    filter::FilterComponents,
    map::{MapComponents, TileComponents},
    position::PxLayer,
    prelude::*,
    profiling::{px_end_span, px_trace, px_trace_span},
    rect::RectComponents,
    sprite::{CompositeSpriteComponents, SpriteComponents},
    text::TextComponents,
};

#[cfg(feature = "gpu_palette")]
use super::{PxLayerOrder, gpu_composite_supported, gpu_sprite_supported};
use super::{
    Screen,
    draw::{self, LayerContentsMap},
    pipeline::{PxPipeline, PxRenderBuffer, PxUniformBuffer},
};

fn resolve_filter_layers<L: PxLayer>(
    out: &mut Vec<(L, bool)>,
    layers: &PxFilterLayers<L>,
    layer_contents: &LayerContentsMap<'_, L>,
) {
    out.clear();
    match layers {
        PxFilterLayers::Single { layer, clip } => out.push((layer.clone(), *clip)),
        // TODO: Revisit range resolution so it can target layers not yet extracted.
        // Current behavior only targets layers already extracted into `layer_contents`.
        PxFilterLayers::Range(range) => out.extend(
            layer_contents
                .keys()
                .filter(|layer| range.contains(layer))
                .map(|layer| (layer.clone(), true)),
        ),
        PxFilterLayers::Many(layers) => {
            out.extend(layers.iter().map(|layer| (layer.clone(), true)));
        }
    }
}

pub(crate) struct PxRenderNode<L: PxLayer> {
    maps: QueryState<MapComponents<L>>,
    tiles: QueryState<TileComponents>,
    // image_to_sprites: QueryState<ImageToSpriteComponents<L>>,
    #[cfg(feature = "gpu_palette")]
    sprites: QueryState<(SpriteComponents<L>, Has<PxGpuSprite>)>,
    #[cfg(not(feature = "gpu_palette"))]
    sprites: QueryState<SpriteComponents<L>>,
    atlas_sprites: QueryState<AtlasSpriteComponents<L>>,
    #[cfg(feature = "gpu_palette")]
    composites: QueryState<(CompositeSpriteComponents<L>, Has<PxGpuComposite>)>,
    #[cfg(not(feature = "gpu_palette"))]
    composites: QueryState<CompositeSpriteComponents<L>>,
    texts: QueryState<TextComponents<L>>,
    rects: QueryState<RectComponents<L>>,
    #[cfg(feature = "line")]
    lines: QueryState<LineComponents<L>>,
    filters: QueryState<FilterComponents<L>, Without<PxCanvas>>,
}

impl<L: PxLayer> FromWorld for PxRenderNode<L> {
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
            rects: world.query(),
            #[cfg(feature = "line")]
            lines: world.query(),
            filters: world.query_filtered(),
        }
    }
}

#[cfg(feature = "headed")]
impl<L: PxLayer> ViewNode for PxRenderNode<L> {
    type ViewQuery = &'static ViewTarget;

    fn update(&mut self, world: &mut World) {
        self.maps.update_archetypes(world);
        self.tiles.update_archetypes(world);
        // self.image_to_sprites.update_archetypes(world);
        self.sprites.update_archetypes(world);
        self.atlas_sprites.update_archetypes(world);
        self.composites.update_archetypes(world);
        self.texts.update_archetypes(world);
        self.rects.update_archetypes(world);
        #[cfg(feature = "line")]
        self.lines.update_archetypes(world);
        self.filters.update_archetypes(world);
    }

    fn run<'w>(
        &self,
        _: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        target: &ViewTarget,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        // Compose each layer into a CPU buffer, then blit to the GPU texture once per frame.
        let &camera = world.resource::<PxCamera>();
        let screen = world.resource::<Screen>();
        let _run_span = px_trace_span!(
            "carapace::screen_node::run",
            width = screen.computed_size.x,
            height = screen.computed_size.y
        );

        let device = world.resource::<RenderDevice>();
        let render_buffer = world.resource::<PxRenderBuffer>();
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

        #[cfg(feature = "gpu_palette")]
        for ((sprite, &position, &anchor, layer, &canvas, animation, filter), has_gpu) in
            self.sprites.iter_manual(world)
        {
            layer_set.insert(layer.clone());
            let gpu_eligible = has_gpu && gpu_sprite_supported(animation.copied(), filter);
            if gpu_eligible {
                continue;
            }

            let sprite = (sprite, position, anchor, canvas, animation, filter);
            layer_contents
                .entry(layer.clone())
                .or_default()
                .sprites
                .push(sprite);
        }

        #[cfg(not(feature = "gpu_palette"))]
        for (sprite, &position, &anchor, layer, &canvas, animation, filter) in
            self.sprites.iter_manual(world)
        {
            let sprite = (sprite, position, anchor, canvas, animation, filter);
            layer_contents
                .entry(layer.clone())
                .or_default()
                .sprites
                .push(sprite);
        }

        for (sprite, &position, &anchor, layer, &canvas, animation, filter) in
            self.atlas_sprites.iter_manual(world)
        {
            let sprite = (sprite, position, anchor, canvas, animation, filter);
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
        for ((composite, &position, &anchor, layer, &canvas, animation, filter), has_gpu) in
            self.composites.iter_manual(world)
        {
            layer_set.insert(layer.clone());
            let gpu_eligible =
                has_gpu && gpu_composite_supported(composite, animation.copied(), filter);
            if gpu_eligible {
                continue;
            }

            let composite = (composite, position, anchor, canvas, animation, filter);
            layer_contents
                .entry(layer.clone())
                .or_default()
                .composites
                .push(composite);
        }

        #[cfg(not(feature = "gpu_palette"))]
        for (composite, &position, &anchor, layer, &canvas, animation, filter) in
            self.composites.iter_manual(world)
        {
            let composite = (composite, position, anchor, canvas, animation, filter);
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

        let mut resolved_filter_layers = Vec::with_capacity(8);
        for (&rect, filter, layers, &pos, &anchor, &canvas, animation, invert) in
            self.rects.iter_manual(world)
        {
            resolve_filter_layers(&mut resolved_filter_layers, layers, &layer_contents);

            for (layer, clip) in &resolved_filter_layers {
                let rect = (rect, filter, pos, anchor, canvas, animation, invert);
                #[cfg(feature = "gpu_palette")]
                {
                    layer_set.insert(layer.clone());
                }
                layer_contents
                    .entry(layer.clone())
                    .or_default()
                    .push_rect(rect, *clip);
            }
        }

        #[cfg(feature = "line")]
        for (line, filter, layers, &canvas, animation, invert) in self.lines.iter_manual(world) {
            let line = (line, filter, canvas, animation, invert);

            resolve_filter_layers(&mut resolved_filter_layers, layers, &layer_contents);

            for (layer, clip) in &resolved_filter_layers {
                #[cfg(feature = "gpu_palette")]
                {
                    layer_set.insert(layer.clone());
                }
                layer_contents
                    .entry(layer.clone())
                    .or_default()
                    .push_line(line, *clip);
            }
        }

        for (filter, layers, animation) in self.filters.iter_manual(world) {
            let filter = (filter, animation);

            resolve_filter_layers(&mut resolved_filter_layers, layers, &layer_contents);

            for (layer, clip) in &resolved_filter_layers {
                #[cfg(feature = "gpu_palette")]
                {
                    layer_set.insert(layer.clone());
                }
                layer_contents
                    .entry(layer.clone())
                    .or_default()
                    .push_filter(filter, *clip);
            }
        }
        px_end_span!(_collect_span);

        #[cfg(feature = "gpu_palette")]
        let layer_order: Vec<L> = layer_set.into_iter().collect();
        #[cfg(feature = "gpu_palette")]
        let layer_order_res = world.resource::<PxLayerOrder<L>>();
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

        let Some(uniform_binding) = world.resource::<PxUniformBuffer>().binding() else {
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

        let px_pipeline = world.resource::<PxPipeline>();
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

    #[test]
    fn resolve_filter_layers_single_preserves_clip_flag() {
        let mut out = Vec::new();
        let layer_contents: LayerContentsMap<'static, TestLayer> = BTreeMap::new();

        resolve_filter_layers(
            &mut out,
            &PxFilterLayers::Single {
                layer: TestLayer::Mid,
                clip: false,
            },
            &layer_contents,
        );

        assert_eq!(out, vec![(TestLayer::Mid, false)]);
    }

    #[test]
    fn resolve_filter_layers_range_uses_existing_layer_keys() {
        let mut out = Vec::new();
        let mut layer_contents: LayerContentsMap<'static, TestLayer> = BTreeMap::new();
        layer_contents.insert(TestLayer::Back, draw::LayerContents::default());
        layer_contents.insert(TestLayer::Front, draw::LayerContents::default());

        resolve_filter_layers(
            &mut out,
            &PxFilterLayers::Range(TestLayer::Back..=TestLayer::Front),
            &layer_contents,
        );

        assert_eq!(out, vec![(TestLayer::Back, true), (TestLayer::Front, true)]);
    }

    #[test]
    fn resolve_filter_layers_many_keeps_declared_order() {
        let mut out = Vec::new();
        let layer_contents: LayerContentsMap<'static, TestLayer> = BTreeMap::new();

        resolve_filter_layers(
            &mut out,
            &PxFilterLayers::Many(vec![TestLayer::Front, TestLayer::Back, TestLayer::Mid]),
            &layer_contents,
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
}
