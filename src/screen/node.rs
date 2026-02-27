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

            if let Some((maps, _, _, _, _, _, _, _, _, _, _)) = layer_contents.get_mut(layer) {
                maps.push(map);
            } else {
                BTreeMap::insert(
                    &mut layer_contents,
                    layer.clone(),
                    (
                        vec![map],
                        Vec::new(),
                        Vec::new(),
                        Vec::new(),
                        Vec::new(),
                        default(),
                        default(),
                        Vec::new(),
                        default(),
                        default(),
                        Vec::new(),
                    ),
                );
            }
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

            if let Some((_, sprites, _, _, _, _, _, _, _, _, _)) = layer_contents.get_mut(layer) {
                sprites.push(sprite);
            } else {
                BTreeMap::insert(
                    &mut layer_contents,
                    layer.clone(),
                    (
                        Vec::new(),
                        vec![sprite],
                        Vec::new(),
                        Vec::new(),
                        Vec::new(),
                        default(),
                        default(),
                        Vec::new(),
                        default(),
                        default(),
                        Vec::new(),
                    ),
                );
            }
        }

        #[cfg(not(feature = "gpu_palette"))]
        for (sprite, &position, &anchor, layer, &canvas, animation, filter) in
            self.sprites.iter_manual(world)
        {
            let sprite = (sprite, position, anchor, canvas, animation, filter);

            if let Some((_, sprites, _, _, _, _, _, _, _, _, _)) = layer_contents.get_mut(layer) {
                sprites.push(sprite);
            } else {
                BTreeMap::insert(
                    &mut layer_contents,
                    layer.clone(),
                    (
                        Vec::new(),
                        vec![sprite],
                        Vec::new(),
                        Vec::new(),
                        Vec::new(),
                        default(),
                        default(),
                        Vec::new(),
                        default(),
                        default(),
                        Vec::new(),
                    ),
                );
            }
        }

        for (sprite, &position, &anchor, layer, &canvas, animation, filter) in
            self.atlas_sprites.iter_manual(world)
        {
            let sprite = (sprite, position, anchor, canvas, animation, filter);
            #[cfg(feature = "gpu_palette")]
            {
                layer_set.insert(layer.clone());
            }

            if let Some((_, _, atlas_sprites, _, _, _, _, _, _, _, _)) =
                layer_contents.get_mut(layer)
            {
                atlas_sprites.push(sprite);
            } else {
                BTreeMap::insert(
                    &mut layer_contents,
                    layer.clone(),
                    (
                        Vec::new(),
                        Vec::new(),
                        vec![sprite],
                        Vec::new(),
                        Vec::new(),
                        default(),
                        default(),
                        Vec::new(),
                        default(),
                        default(),
                        Vec::new(),
                    ),
                );
            }
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

            if let Some((_, _, _, composites, _, _, _, _, _, _, _)) = layer_contents.get_mut(layer)
            {
                composites.push(composite);
            } else {
                BTreeMap::insert(
                    &mut layer_contents,
                    layer.clone(),
                    (
                        Vec::new(),
                        Vec::new(),
                        Vec::new(),
                        vec![composite],
                        Vec::new(),
                        default(),
                        default(),
                        Vec::new(),
                        default(),
                        default(),
                        Vec::new(),
                    ),
                );
            }
        }

        #[cfg(not(feature = "gpu_palette"))]
        for (composite, &position, &anchor, layer, &canvas, animation, filter) in
            self.composites.iter_manual(world)
        {
            let composite = (composite, position, anchor, canvas, animation, filter);

            if let Some((_, _, _, composites, _, _, _, _, _, _, _)) = layer_contents.get_mut(layer)
            {
                composites.push(composite);
            } else {
                BTreeMap::insert(
                    &mut layer_contents,
                    layer.clone(),
                    (
                        Vec::new(),
                        Vec::new(),
                        Vec::new(),
                        vec![composite],
                        Vec::new(),
                        default(),
                        default(),
                        Vec::new(),
                        default(),
                        default(),
                        Vec::new(),
                    ),
                );
            }
        }

        for (text, &pos, &alignment, layer, &canvas, animation, filter) in
            self.texts.iter_manual(world)
        {
            let text = (text, pos, alignment, canvas, animation, filter);
            #[cfg(feature = "gpu_palette")]
            {
                layer_set.insert(layer.clone());
            }

            if let Some((_, _, _, _, texts, _, _, _, _, _, _)) = layer_contents.get_mut(layer) {
                texts.push(text);
            } else {
                BTreeMap::insert(
                    &mut layer_contents,
                    layer.clone(),
                    (
                        Vec::new(),
                        Vec::new(),
                        Vec::new(),
                        Vec::new(),
                        vec![text],
                        default(),
                        default(),
                        Vec::new(),
                        default(),
                        default(),
                        Vec::new(),
                    ),
                );
            }
        }

        for (&rect, filter, layers, &pos, &anchor, &canvas, animation, invert) in
            self.rects.iter_manual(world)
        {
            for (layer, clip) in match layers {
                PxFilterLayers::Single { layer, clip } => vec![(layer.clone(), *clip)],
                // TODO Need to do this after all layers have been extracted
                PxFilterLayers::Range(range) => layer_contents
                    .keys()
                    .filter(|layer| range.contains(layer))
                    .map(|layer| (layer.clone(), true))
                    .collect(),
                PxFilterLayers::Many(layers) => {
                    layers.iter().map(|layer| (layer.clone(), true)).collect()
                }
            } {
                let rect = (rect, filter, pos, anchor, canvas, animation, invert);
                #[cfg(feature = "gpu_palette")]
                {
                    layer_set.insert(layer.clone());
                }

                if let Some((_, _, _, _, _, clip_rects, _, _, over_rects, _, _)) =
                    layer_contents.get_mut(&layer)
                {
                    if clip { clip_rects } else { over_rects }.push(rect);
                } else {
                    let rects = vec![rect];

                    BTreeMap::insert(
                        &mut layer_contents,
                        layer,
                        if clip {
                            (
                                default(),
                                Vec::new(),
                                Vec::new(),
                                Vec::new(),
                                Vec::new(),
                                rects,
                                default(),
                                Vec::new(),
                                default(),
                                default(),
                                Vec::new(),
                            )
                        } else {
                            (
                                Vec::new(),
                                Vec::new(),
                                Vec::new(),
                                Vec::new(),
                                Vec::new(),
                                default(),
                                default(),
                                default(),
                                rects,
                                default(),
                                Vec::new(),
                            )
                        },
                    );
                }
            }
        }

        #[cfg(feature = "line")]
        for (line, filter, layers, &canvas, animation, invert) in self.lines.iter_manual(world) {
            let line = (line, filter, canvas, animation, invert);

            for (layer, clip) in match layers {
                PxFilterLayers::Single { layer, clip } => vec![(layer.clone(), *clip)],
                PxFilterLayers::Range(range) => layer_contents
                    .keys()
                    .filter(|layer| range.contains(layer))
                    .map(|layer| (layer.clone(), true))
                    .collect(),
                PxFilterLayers::Many(layers) => {
                    layers.iter().map(|layer| (layer.clone(), true)).collect()
                }
            } {
                #[cfg(feature = "gpu_palette")]
                {
                    layer_set.insert(layer.clone());
                }
                if let Some((_, _, _, _, _, _, clip_lines, _, _, over_lines, _)) =
                    layer_contents.get_mut(&layer)
                {
                    if clip { clip_lines } else { over_lines }.push(line);
                } else {
                    let lines = vec![line];

                    BTreeMap::insert(
                        &mut layer_contents,
                        layer,
                        if clip {
                            (
                                Vec::new(),
                                Vec::new(),
                                Vec::new(),
                                Vec::new(),
                                Vec::new(),
                                default(),
                                lines,
                                Vec::new(),
                                default(),
                                default(),
                                Vec::new(),
                            )
                        } else {
                            (
                                Vec::new(),
                                Vec::new(),
                                Vec::new(),
                                Vec::new(),
                                Vec::new(),
                                default(),
                                default(),
                                Vec::new(),
                                default(),
                                lines,
                                Vec::new(),
                            )
                        },
                    );
                }
            }
        }

        for (filter, layers, animation) in self.filters.iter_manual(world) {
            let filter = (filter, animation);

            for (layer, clip) in match layers {
                PxFilterLayers::Single { layer, clip } => vec![(layer.clone(), *clip)],
                PxFilterLayers::Range(range) => layer_contents
                    .keys()
                    .filter(|layer| range.contains(layer))
                    .map(|layer| (layer.clone(), true))
                    .collect(),
                PxFilterLayers::Many(layers) => {
                    layers.iter().map(|layer| (layer.clone(), true)).collect()
                }
            } {
                #[cfg(feature = "gpu_palette")]
                {
                    layer_set.insert(layer.clone());
                }
                if let Some((_, _, _, _, _, _, _, clip_filters, _, _, over_filters)) =
                    layer_contents.get_mut(&layer)
                {
                    if clip { clip_filters } else { over_filters }.push(filter);
                } else {
                    let filters = vec![filter];

                    BTreeMap::insert(
                        &mut layer_contents,
                        layer,
                        if clip {
                            (
                                Vec::new(),
                                Vec::new(),
                                Vec::new(),
                                Vec::new(),
                                Vec::new(),
                                default(),
                                default(),
                                filters,
                                default(),
                                default(),
                                Vec::new(),
                            )
                        } else {
                            (
                                Vec::new(),
                                Vec::new(),
                                Vec::new(),
                                Vec::new(),
                                Vec::new(),
                                default(),
                                default(),
                                Vec::new(),
                                default(),
                                default(),
                                filters,
                            )
                        },
                    );
                }
            }
        }
        px_end_span!(_collect_span);

        #[cfg(feature = "gpu_palette")]
        let layer_order: Vec<L> = layer_set.into_iter().collect();
        #[cfg(feature = "gpu_palette")]
        world.resource::<PxLayerOrder<L>>().set(layer_order.clone());
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
