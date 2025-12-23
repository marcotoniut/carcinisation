use std::sync::RwLock;

use bevy_derive::{Deref, DerefMut};
#[cfg(feature = "headed")]
use bevy_render::{
    render_resource::{
        BindGroupLayout, BindGroupLayoutEntries, CachedRenderPipelineId, ColorTargetState,
        ColorWrites, DynamicUniformBuffer, Extent3d, FragmentState, PipelineCache,
        RenderPipelineDescriptor, ShaderStages, ShaderType, Texture, TextureDescriptor,
        TextureDimension, TextureFormat, TextureSampleType, TextureUsages, VertexState,
        binding_types::{texture_2d, uniform_buffer},
    },
    renderer::{RenderDevice, RenderQueue},
};

use crate::prelude::*;

use super::{SCREEN_SHADER_HANDLE, Screen};

#[cfg(feature = "headed")]
#[derive(ShaderType)]
pub(crate) struct PxUniform {
    pub(crate) palette: [Vec3; 256],
    pub(crate) fit_factor: Vec2,
}

#[cfg(feature = "headed")]
#[derive(Resource, Deref, DerefMut, Default)]
pub(crate) struct PxUniformBuffer(DynamicUniformBuffer<PxUniform>);

#[cfg(feature = "headed")]
pub(crate) fn prepare_uniform(
    mut buffer: ResMut<PxUniformBuffer>,
    screen: Res<Screen>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
) {
    let Some(mut writer) = buffer.get_writer(1, &device, &queue) else {
        return;
    };

    let aspect_ratio_ratio =
        screen.computed_size.x as f32 / screen.computed_size.y as f32 / screen.window_aspect_ratio;
    writer.write(&PxUniform {
        palette: screen.palette,
        fit_factor: if aspect_ratio_ratio > 1. {
            Vec2::new(1., 1. / aspect_ratio_ratio)
        } else {
            Vec2::new(aspect_ratio_ratio, 1.)
        },
    });
}

#[cfg(feature = "headed")]
#[derive(Resource)]
pub(crate) struct PxPipeline {
    pub(crate) layout: BindGroupLayout,
    pub(crate) id: CachedRenderPipelineId,
}

#[cfg(feature = "headed")]
impl FromWorld for PxPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let layout = render_device.create_bind_group_layout(
            "px_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    texture_2d(TextureSampleType::Uint),
                    uniform_buffer::<PxUniform>(false).visibility(ShaderStages::VERTEX_FRAGMENT),
                ),
            ),
        );

        Self {
            id: world.resource_mut::<PipelineCache>().queue_render_pipeline(
                RenderPipelineDescriptor {
                    label: Some("px_pipeline".into()),
                    layout: vec![layout.clone()],
                    vertex: VertexState {
                        shader: SCREEN_SHADER_HANDLE,
                        shader_defs: Vec::new(),
                        entry_point: Some("vertex".into()),
                        buffers: Vec::new(),
                    },
                    fragment: Some(FragmentState {
                        shader: SCREEN_SHADER_HANDLE,
                        shader_defs: Vec::new(),
                        entry_point: Some("fragment".into()),
                        targets: vec![Some(ColorTargetState {
                            format: TextureFormat::bevy_default(),
                            blend: None,
                            write_mask: ColorWrites::ALL,
                        })],
                    }),
                    primitive: default(),
                    depth_stencil: None,
                    multisample: default(),
                    push_constant_ranges: Vec::new(),
                    zero_initialize_workgroup_memory: true,
                },
            ),
            layout,
        }
    }
}

#[cfg(feature = "headed")]
#[derive(Resource)]
pub(crate) struct PxRenderBuffer {
    inner: RwLock<PxRenderBufferInner>,
}

#[cfg(feature = "headed")]
pub(crate) struct PxRenderBufferInner {
    pub(crate) size: UVec2,
    pub(crate) image: Option<Image>,
    pub(crate) texture: Option<Texture>,
}

#[cfg(feature = "headed")]
impl Default for PxRenderBuffer {
    fn default() -> Self {
        Self {
            inner: RwLock::new(PxRenderBufferInner {
                size: UVec2::ZERO,
                image: None,
                texture: None,
            }),
        }
    }
}

#[cfg(feature = "headed")]
impl PxRenderBuffer {
    pub(crate) fn ensure_size(&self, device: &RenderDevice, size: UVec2) {
        let mut inner = self.inner.write().unwrap();
        if size == inner.size && inner.image.is_some() && inner.texture.is_some() {
            return;
        }

        inner.size = size;

        let descriptor = TextureDescriptor {
            label: Some("px_present_texture"),
            size: Extent3d {
                width: size.x,
                height: size.y,
                depth_or_array_layers: 1,
            },
            dimension: TextureDimension::D2,
            format: TextureFormat::R8Uint,
            sample_count: 1,
            mip_level_count: 1,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        };

        inner.texture = Some(device.create_texture(&descriptor));
        inner.image = Some(Image::new_fill(
            descriptor.size,
            descriptor.dimension,
            &[0],
            descriptor.format,
            default(),
        ));
    }

    pub(crate) fn clear(&self) {
        let mut inner = self.inner.write().unwrap();
        if let Some(image) = inner.image.as_mut()
            && let Some(data) = image.data.as_mut()
        {
            data.fill(0);
        }
    }

    pub(crate) fn read_inner(&self) -> std::sync::RwLockReadGuard<'_, PxRenderBufferInner> {
        self.inner.read().unwrap()
    }

    pub(crate) fn write_inner(&self) -> std::sync::RwLockWriteGuard<'_, PxRenderBufferInner> {
        self.inner.write().unwrap()
    }
}
