use std::{
    collections::{BTreeMap, HashMap, HashSet},
    mem::size_of,
    ops::Range,
    sync::RwLock,
};

use bevy_ecs::query::QueryState;
use bevy_mesh::VertexBufferLayout;
use bevy_render::{
    render_asset::RenderAssets,
    render_graph::{NodeRunError, RenderGraphContext, ViewNode},
    render_resource::{
        BindGroup, BindGroupEntries, BindGroupLayout, BindGroupLayoutDescriptor,
        BindGroupLayoutEntries, BlendState, Buffer, BufferDescriptor, BufferUsages,
        CachedRenderPipelineId, ColorTargetState, ColorWrites, FragmentState, PipelineCache,
        RenderPassColorAttachment, RenderPassDescriptor, RenderPipelineDescriptor, ShaderStages,
        TextureFormat, TextureSampleType, TextureViewDescriptor, VertexAttribute, VertexFormat,
        VertexState, VertexStepMode,
        binding_types::{texture_2d, uniform_buffer},
    },
    renderer::{RenderContext, RenderDevice, RenderQueue},
    view::ViewTarget,
};
use bytemuck::{Pod, Zeroable};

use crate::{
    frame::{PxFrameView, resolve_frame_binding},
    prelude::*,
    profiling::{px_end_span, px_trace, px_trace_span},
    sprite::{
        CompositeSpriteComponents, PxGpuComposite, PxGpuSprite, PxSpriteAsset, PxSpriteGpu,
        SpriteComponents,
    },
};

use super::{
    GPU_SPRITE_SHADER_HANDLE, PxLayerOrder, Screen, gpu_composite_supported, gpu_sprite_supported,
    pipeline::{PxRenderBuffer, PxUniform, PxUniformBuffer},
};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct SpriteVertex {
    position: [f32; 2],
    uv: [f32; 2],
    layer: u32,
}

impl SpriteVertex {
    fn layout() -> VertexBufferLayout {
        const ATTRIBUTES: [VertexAttribute; 3] = [
            VertexAttribute {
                format: VertexFormat::Float32x2,
                offset: 0,
                shader_location: 0,
            },
            VertexAttribute {
                format: VertexFormat::Float32x2,
                offset: 8,
                shader_location: 1,
            },
            VertexAttribute {
                format: VertexFormat::Uint32,
                offset: 16,
                shader_location: 2,
            },
        ];

        VertexBufferLayout {
            array_stride: size_of::<SpriteVertex>() as u64,
            step_mode: VertexStepMode::Vertex,
            attributes: Vec::from(ATTRIBUTES),
        }
    }
}

pub(crate) struct PxGpuSpriteNode<L: PxLayer> {
    sprites: QueryState<SpriteComponents<L>, With<PxGpuSprite>>,
    composites: QueryState<CompositeSpriteComponents<L>, With<PxGpuComposite>>,
}

impl<L: PxLayer> FromWorld for PxGpuSpriteNode<L> {
    fn from_world(world: &mut World) -> Self {
        Self {
            sprites: world.query_filtered(),
            composites: world.query_filtered(),
        }
    }
}

#[derive(Resource)]
pub(crate) struct PxGpuSpritePipeline {
    pub(crate) layout: BindGroupLayout,
    pub(crate) id: CachedRenderPipelineId,
}

impl FromWorld for PxGpuSpritePipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<bevy_render::renderer::RenderDevice>();

        let layout_descriptor = BindGroupLayoutDescriptor::new(
            "px_gpu_sprite_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    texture_2d(TextureSampleType::Uint),
                    texture_2d(TextureSampleType::Uint),
                    uniform_buffer::<PxUniform>(false).visibility(ShaderStages::VERTEX_FRAGMENT),
                ),
            ),
        );
        let layout = render_device
            .create_bind_group_layout(layout_descriptor.label.as_ref(), &layout_descriptor.entries);

        let id =
            world
                .resource_mut::<PipelineCache>()
                .queue_render_pipeline(RenderPipelineDescriptor {
                    label: Some("px_gpu_sprite_pipeline".into()),
                    layout: vec![layout_descriptor],
                    vertex: VertexState {
                        shader: GPU_SPRITE_SHADER_HANDLE,
                        shader_defs: Vec::new(),
                        entry_point: Some("vertex".into()),
                        buffers: vec![SpriteVertex::layout()],
                    },
                    fragment: Some(FragmentState {
                        shader: GPU_SPRITE_SHADER_HANDLE,
                        shader_defs: Vec::new(),
                        entry_point: Some("fragment".into()),
                        targets: vec![Some(ColorTargetState {
                            format: TextureFormat::bevy_default(),
                            blend: Some(BlendState::ALPHA_BLENDING),
                            write_mask: ColorWrites::ALL,
                        })],
                    }),
                    primitive: default(),
                    depth_stencil: None,
                    multisample: default(),
                    push_constant_ranges: Vec::new(),
                    zero_initialize_workgroup_memory: true,
                });

        Self { layout, id }
    }
}

#[derive(Resource)]
pub(crate) struct PxGpuSpriteBuffer {
    inner: RwLock<PxGpuSpriteBufferInner>,
}

struct PxGpuSpriteBufferInner {
    buffer: Option<Buffer>,
    capacity: usize,
}

impl Default for PxGpuSpriteBuffer {
    fn default() -> Self {
        Self {
            inner: RwLock::new(PxGpuSpriteBufferInner {
                buffer: None,
                capacity: 0,
            }),
        }
    }
}

impl PxGpuSpriteBuffer {
    fn write(
        &self,
        device: &RenderDevice,
        queue: &RenderQueue,
        data: &[SpriteVertex],
    ) -> Option<Buffer> {
        if data.is_empty() {
            return None;
        }

        let bytes = bytemuck::cast_slice(data);
        let mut inner = self.inner.write().unwrap();
        let required = bytes.len();

        if inner.buffer.is_none() || required > inner.capacity {
            let capacity = required.next_power_of_two().max(256);
            inner.buffer = Some(device.create_buffer(&BufferDescriptor {
                label: Some("px_gpu_sprite_vertices"),
                size: capacity as u64,
                usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
            inner.capacity = capacity;
        }

        let buffer = inner.buffer.as_ref().unwrap();
        queue.write_buffer(buffer, 0, bytes);
        Some(buffer.clone())
    }
}

struct SpriteItem {
    sprite: Handle<PxSpriteAsset>,
    position: PxPosition,
    anchor: PxAnchor,
    canvas: PxCanvas,
    frame: Option<PxFrameView>,
}

struct SpriteDraw<'a> {
    range: Range<u32>,
    sprite: &'a PxSpriteGpu,
    handle: Handle<PxSpriteAsset>,
}

impl<L: PxLayer> ViewNode for PxGpuSpriteNode<L> {
    type ViewQuery = &'static ViewTarget;

    fn update(&mut self, world: &mut World) {
        self.sprites.update_archetypes(world);
        self.composites.update_archetypes(world);
    }

    fn run<'w>(
        &self,
        _: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        target: &ViewTarget,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let Some(uniform_binding) = world.resource::<PxUniformBuffer>().binding() else {
            return Ok(());
        };

        let screen = world.resource::<Screen>();
        let _run_span = px_trace_span!(
            "carapace::gpu_sprite_node::run",
            width = screen.computed_size.x,
            height = screen.computed_size.y
        );
        if screen.computed_size.x == 0 || screen.computed_size.y == 0 {
            return Ok(());
        }

        let layer_order = world.resource::<PxLayerOrder<L>>();
        let layer_order = layer_order.read();

        let fit_factor = fit_factor(screen);
        let screen_size = Vec2::new(screen.computed_size.x as f32, screen.computed_size.y as f32);
        let screen_height = screen.computed_size.y as i32;
        let camera = *world.resource::<PxCamera>();

        let depth_view = {
            let render_buffer = world.resource::<PxRenderBuffer>();
            let inner = render_buffer.read_inner();
            let Some(depth_texture) = inner.depth_texture.as_ref() else {
                return Ok(());
            };
            depth_texture.create_view(&TextureViewDescriptor::default())
        };

        let sprite_assets = world.resource::<RenderAssets<PxSpriteGpu>>();
        let mut sprites_by_layer: BTreeMap<L, Vec<SpriteItem>> = BTreeMap::new();
        let _collect_span = px_trace_span!("carapace::gpu_sprite_node::collect");

        for (sprite, &position, &anchor, layer, &canvas, frame, filter) in
            self.sprites.iter_manual(world)
        {
            if !gpu_sprite_supported(frame.copied(), filter) {
                continue;
            }

            sprites_by_layer
                .entry(layer.clone())
                .or_default()
                .push(SpriteItem {
                    sprite: sprite.clone().0,
                    position,
                    anchor,
                    canvas,
                    frame: frame.copied(),
                });
        }

        for (composite, &position, &anchor, layer, &canvas, frame, filter) in
            self.composites.iter_manual(world)
        {
            if !gpu_composite_supported(composite, frame.copied(), filter) {
                continue;
            }

            let metrics = if composite.size.x == 0 || composite.size.y == 0 {
                composite.metrics_with(|handle| {
                    let sprite = sprite_assets.get(handle)?;
                    let part_count = frame_count(sprite);
                    let part_size = match frame_height(sprite) {
                        Some(height) => UVec2::new(sprite.size.x, height),
                        None => return None,
                    };
                    Some(crate::sprite::PxCompositePartMetrics {
                        size: part_size,
                        frame_count: part_count,
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

            let base_pos = *position - anchor.pos(metrics.size).as_ivec2();
            let master = frame.copied();
            let master_count = metrics.frame_count;

            for part in &composite.parts {
                let Some(sprite_gpu) = sprite_assets.get(&part.sprite) else {
                    continue;
                };

                let part_count = frame_count(sprite_gpu);
                let part_frame =
                    resolve_frame_binding(master, master_count, part_count, &part.frame);
                let part_pos = base_pos + (part.offset - metrics.origin);

                sprites_by_layer
                    .entry(layer.clone())
                    .or_default()
                    .push(SpriteItem {
                        sprite: part.sprite.clone(),
                        position: PxPosition(part_pos),
                        anchor: PxAnchor::BottomLeft,
                        canvas,
                        frame: part_frame,
                    });
            }
        }
        px_end_span!(_collect_span);

        if sprites_by_layer.is_empty() {
            return Ok(());
        }
        let estimated_items: usize = sprites_by_layer.values().map(Vec::len).sum();
        px_trace!(
            layer_count = sprites_by_layer.len(),
            estimated_items,
            "carapace::gpu_sprite_node::collected"
        );

        let _build_span = px_trace_span!("carapace::gpu_sprite_node::build");
        let mut vertices: Vec<SpriteVertex> = Vec::with_capacity(estimated_items.saturating_mul(6));
        let mut draws: Vec<SpriteDraw<'_>> = Vec::with_capacity(estimated_items);

        for (layer, items) in sprites_by_layer {
            let Some(layer_index) = layer_index_for(&layer_order, &layer) else {
                continue;
            };

            for item in items {
                let handle = item.sprite;
                let Some(sprite_gpu) = sprite_assets.get(&handle) else {
                    continue;
                };

                let Some(frame_height) = frame_height(sprite_gpu) else {
                    continue;
                };

                let frame_count = frame_count(sprite_gpu);
                if frame_count == 0 {
                    continue;
                }

                let frame_index = frame_index(item.frame, frame_count) as u32;
                let frame_top = frame_index.saturating_mul(frame_height);
                let frame_bottom = frame_top.saturating_add(frame_height);
                if frame_bottom > sprite_gpu.size.y {
                    continue;
                }

                let size = UVec2::new(sprite_gpu.size.x, frame_height);
                let mut position = *item.position - item.anchor.pos(size).as_ivec2();
                if matches!(item.canvas, PxCanvas::World) {
                    position -= *camera;
                }

                let image_pos = IVec2::new(position.x, screen_height - position.y);
                let min = image_pos - IVec2::new(0, size.y as i32);
                let max = image_pos + IVec2::new(size.x as i32, 0);

                let top_left = Vec2::new(min.x as f32, min.y as f32);
                let bottom_right = Vec2::new(max.x as f32, max.y as f32);
                let bottom_left = Vec2::new(min.x as f32, max.y as f32);
                let top_right = Vec2::new(max.x as f32, min.y as f32);

                let ndc_top_left = pixel_to_ndc(top_left, screen_size, fit_factor);
                let ndc_bottom_left = pixel_to_ndc(bottom_left, screen_size, fit_factor);
                let ndc_bottom_right = pixel_to_ndc(bottom_right, screen_size, fit_factor);
                let ndc_top_right = pixel_to_ndc(top_right, screen_size, fit_factor);

                let v_min = frame_top as f32 / sprite_gpu.size.y as f32;
                let v_max = frame_bottom as f32 / sprite_gpu.size.y as f32;

                let start = vertices.len() as u32;
                vertices.extend_from_slice(&[
                    SpriteVertex {
                        position: ndc_top_left,
                        uv: [0.0, v_min],
                        layer: layer_index,
                    },
                    SpriteVertex {
                        position: ndc_bottom_left,
                        uv: [0.0, v_max],
                        layer: layer_index,
                    },
                    SpriteVertex {
                        position: ndc_bottom_right,
                        uv: [1.0, v_max],
                        layer: layer_index,
                    },
                    SpriteVertex {
                        position: ndc_top_left,
                        uv: [0.0, v_min],
                        layer: layer_index,
                    },
                    SpriteVertex {
                        position: ndc_bottom_right,
                        uv: [1.0, v_max],
                        layer: layer_index,
                    },
                    SpriteVertex {
                        position: ndc_top_right,
                        uv: [1.0, v_min],
                        layer: layer_index,
                    },
                ]);

                draws.push(SpriteDraw {
                    range: start..start + 6,
                    sprite: sprite_gpu,
                    handle,
                });
            }
        }
        px_end_span!(_build_span);

        if vertices.is_empty() {
            return Ok(());
        }
        px_trace!(
            vertex_count = vertices.len(),
            draw_count = draws.len(),
            "carapace::gpu_sprite_node::prepared"
        );

        let _upload_span = px_trace_span!("carapace::gpu_sprite_node::upload");
        let render_device = render_context.render_device();
        let render_queue = world.resource::<RenderQueue>();
        let Some(vertex_buffer) =
            world
                .resource::<PxGpuSpriteBuffer>()
                .write(render_device, render_queue, &vertices)
        else {
            return Ok(());
        };

        let px_pipeline = world.resource::<PxGpuSpritePipeline>();
        let Some(pipeline) = world
            .resource::<PipelineCache>()
            .get_render_pipeline(px_pipeline.id)
        else {
            return Ok(());
        };

        let post_process = target.post_process_write();

        let mut bind_groups: HashMap<Handle<PxSpriteAsset>, BindGroup> =
            HashMap::with_capacity(draws.len());
        // Build one bind group per distinct sprite texture, preserving first-seen draw order.
        for draw_index in first_seen_indices_by_asset_id(draws.iter().map(|draw| draw.handle.id()))
        {
            let draw = &draws[draw_index];
            let texture_view = draw
                .sprite
                .texture
                .create_view(&TextureViewDescriptor::default());
            let bind_group = render_device.create_bind_group(
                "px_gpu_sprite_bind_group",
                &px_pipeline.layout,
                &BindGroupEntries::sequential((
                    &texture_view,
                    &depth_view,
                    uniform_binding.clone(),
                )),
            );
            bind_groups.insert(draw.handle.clone(), bind_group);
        }
        px_end_span!(_upload_span);

        let _present_span = px_trace_span!("carapace::gpu_sprite_node::present");
        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("px_gpu_sprite_pass"),
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
        render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));

        for draw in &draws {
            let Some(bind_group) = bind_groups.get(&draw.handle) else {
                continue;
            };
            render_pass.set_bind_group(0, bind_group, &[]);
            render_pass.draw(draw.range.clone(), 0..1);
        }
        px_end_span!(_present_span);

        Ok(())
    }
}

fn frame_height(sprite: &PxSpriteGpu) -> Option<u32> {
    let width = sprite.size.x as usize;
    if width == 0 || sprite.frame_size == 0 || !sprite.frame_size.is_multiple_of(width) {
        return None;
    }

    Some((sprite.frame_size / width) as u32)
}

/// Returns indices of the first occurrence of each distinct sprite asset id.
///
/// Preserves first-seen order from `asset_ids`.
/// Used to build one GPU bind group per unique sprite texture.
///
/// Complexity:
/// - Time: `O(n)` expected (hash set inserts/lookups)
/// - Space: `O(u)` where `u` is number of unique asset ids
fn first_seen_indices_by_asset_id<I>(asset_ids: I) -> Vec<usize>
where
    I: IntoIterator<Item = bevy_asset::AssetId<PxSpriteAsset>>,
{
    let asset_ids = asset_ids.into_iter();
    let (lower_bound, _) = asset_ids.size_hint();
    let mut seen = HashSet::with_capacity(lower_bound);
    let mut first_seen_indices = Vec::with_capacity(lower_bound);

    for (index, asset_id) in asset_ids.enumerate() {
        if seen.insert(asset_id) {
            first_seen_indices.push(index);
        }
    }

    first_seen_indices
}

fn frame_count(sprite: &PxSpriteGpu) -> usize {
    let area = sprite.size.x as usize * sprite.size.y as usize;
    if sprite.frame_size == 0 {
        return 0;
    }

    area / sprite.frame_size
}

fn layer_index_for<L: PxLayer>(layers: &[L], layer: &L) -> Option<u32> {
    let index = layers.binary_search(layer).ok()?;
    u32::try_from((index + 1) * 2).ok()
}

fn frame_index(frame: Option<PxFrameView>, frame_count: usize) -> usize {
    if frame_count == 0 {
        return 0;
    }

    let Some(frame) = frame else {
        return 0;
    };

    let index = match frame.selector {
        PxFrameSelector::Normalized(value) => value * (frame_count.saturating_sub(1)) as f32,
        PxFrameSelector::Index(value) => value,
    };

    let index = index.floor() as i32;
    index.rem_euclid(frame_count as i32) as usize
}

fn fit_factor(screen: &Screen) -> Vec2 {
    let aspect_ratio_ratio =
        screen.computed_size.x as f32 / screen.computed_size.y as f32 / screen.window_aspect_ratio;
    if aspect_ratio_ratio > 1.0 {
        Vec2::new(1.0, 1.0 / aspect_ratio_ratio)
    } else {
        Vec2::new(aspect_ratio_ratio, 1.0)
    }
}

fn pixel_to_ndc(pos: Vec2, screen_size: Vec2, fit_factor: Vec2) -> [f32; 2] {
    let ndc = Vec2::new(
        (pos.x / screen_size.x) * 2.0 - 1.0,
        1.0 - (pos.y / screen_size.y) * 2.0,
    );
    (ndc * fit_factor).to_array()
}

#[cfg(all(test, feature = "gpu_palette"))]
mod tests {
    use std::fmt::Write as _;

    use super::*;
    use bevy_asset::uuid::Uuid;
    use insta::assert_snapshot;

    #[test]
    fn frame_index_wraps() {
        let frame_count = 4;
        let cases = [
            ("index 0.0", PxFrameSelector::Index(0.0)),
            ("index 5.2", PxFrameSelector::Index(5.2)),
            ("index -1.0", PxFrameSelector::Index(-1.0)),
            ("normalized 1.8", PxFrameSelector::Normalized(1.8)),
            ("normalized -0.2", PxFrameSelector::Normalized(-0.2)),
        ];
        let mut out = String::from("frame_count=4\n");
        for (label, selector) in cases {
            let frame = PxFrameView {
                selector,
                transition: PxFrameTransition::None,
            };
            let index = frame_index(Some(frame), frame_count);
            let _ = writeln!(&mut out, "{label} -> {index}");
        }

        assert_snapshot!(&out, @r###"
frame_count=4
index 0.0 -> 0
index 5.2 -> 1
index -1.0 -> 3
normalized 1.8 -> 1
normalized -0.2 -> 3
"###);
    }

    #[test]
    fn bind_group_selection_uses_first_seen_sprite_handles() {
        fn sprite_handle(id: u128) -> Handle<PxSpriteAsset> {
            Handle::Uuid(Uuid::from_u128(id), std::marker::PhantomData)
        }

        let handles = [
            sprite_handle(3),
            sprite_handle(1),
            sprite_handle(3),
            sprite_handle(2),
            sprite_handle(1),
        ];

        let indices = first_seen_indices_by_asset_id(handles.iter().map(|handle| handle.id()));

        assert_eq!(
            indices,
            vec![0, 1, 3],
            "bind-group creation should keep first-seen draw indices for each sprite handle"
        );
        assert_eq!(
            indices
                .iter()
                .map(|&index| handles[index].id())
                .collect::<Vec<_>>(),
            vec![handles[0].id(), handles[1].id(), handles[3].id()],
            "selected draw indices should map to unique sprite handles in first-seen order"
        );
    }
}
