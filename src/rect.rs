use bevy_derive::{Deref, DerefMut};
use bevy_math::{ivec2, uvec2};
#[cfg(feature = "headed")]
use bevy_render::{Extract, RenderApp, sync_world::RenderEntity};

use crate::{
    filter::DefaultPxFilterLayers, frame::Frames, image::PxImageSliceMut, position::Spatial,
    prelude::*,
};

pub(crate) fn plug<L: PxLayer>(_app: &mut App) {
    #[cfg(feature = "headed")]
    _app.sub_app_mut(RenderApp)
        .add_systems(ExtractSchedule, extract_rects::<L>);
}

/// A rectangle in which a filter is applied
#[derive(Component, Deref, DerefMut, Clone, Copy, Reflect)]
#[require(PxFilter, DefaultPxFilterLayers, PxPosition, PxAnchor, PxCanvas)]
#[cfg_attr(feature = "headed", require(Visibility))]
pub struct PxRect(pub UVec2);

impl Default for PxRect {
    fn default() -> Self {
        Self(UVec2::ONE)
    }
}

impl Frames for (PxRect, &PxFilterAsset) {
    type Param = bool;

    fn frame_count(&self) -> usize {
        self.1.frame_count()
    }

    fn draw(
        &self,
        invert: bool,
        image: &mut PxImageSliceMut,
        frame: impl Fn(UVec2) -> usize,
        filter_fn: impl Fn(u8) -> u8,
    ) {
        let (_, PxFilterAsset(filter)) = self;

        if invert {
            let image_width = image.image_width() as i32;
            let image_height = image.image_height() as i32;
            let rect_min = image.offset();
            let rect_max = rect_min + IVec2::new(image.width() as i32, image.height() as i32);
            let x_min = rect_min.x.clamp(0, image_width);
            let x_max = rect_max.x.clamp(0, image_width);
            let y_min = rect_min.y.clamp(0, image_height);
            let y_max = rect_max.y.clamp(0, image_height);

            for y in 0..y_min {
                for x in 0..image_width {
                    let pos = ivec2(x, y);
                    let pixel = image.image_pixel_mut(pos);
                    *pixel = filter_fn(filter.pixel(ivec2(
                        *pixel as i32,
                        frame(uvec2(x as u32, y as u32)) as i32,
                    )));
                }
            }

            for y in y_max..image_height {
                for x in 0..image_width {
                    let pos = ivec2(x, y);
                    let pixel = image.image_pixel_mut(pos);
                    *pixel = filter_fn(filter.pixel(ivec2(
                        *pixel as i32,
                        frame(uvec2(x as u32, y as u32)) as i32,
                    )));
                }
            }

            for y in y_min..y_max {
                for x in 0..x_min {
                    let pos = ivec2(x, y);
                    let pixel = image.image_pixel_mut(pos);
                    *pixel = filter_fn(filter.pixel(ivec2(
                        *pixel as i32,
                        frame(uvec2(x as u32, y as u32)) as i32,
                    )));
                }

                for x in x_max..image_width {
                    let pos = ivec2(x, y);
                    let pixel = image.image_pixel_mut(pos);
                    *pixel = filter_fn(filter.pixel(ivec2(
                        *pixel as i32,
                        frame(uvec2(x as u32, y as u32)) as i32,
                    )));
                }
            }
        } else {
            let image_width = image.image_width();
            image.for_each_mut(|_, image_index, pixel| {
                let x = (image_index % image_width) as u32;
                let y = (image_index / image_width) as u32;
                *pixel = filter_fn(filter.pixel(ivec2(*pixel as i32, frame(uvec2(x, y)) as i32)));
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{filter::PxFilterAsset, frame::draw_frame, image::PxImage};

    fn filter_asset() -> PxFilterAsset {
        PxFilterAsset(PxImage::new(vec![0, 2, 0, 0], 4))
    }

    fn pixels(image: &PxImage) -> Vec<u8> {
        let size = image.size();
        let mut out = Vec::with_capacity((size.x * size.y) as usize);
        for y in 0..size.y as i32 {
            for x in 0..size.x as i32 {
                out.push(image.pixel(IVec2::new(x, y)));
            }
        }
        out
    }

    #[test]
    fn rect_draws_inside_only() {
        let mut image = PxImage::new(vec![1; 16], 4);
        let mut slice = image.slice_all_mut();
        let mut rect_slice = slice.slice_mut(IRect {
            min: ivec2(1, 1),
            max: ivec2(3, 3),
        });
        let rect = PxRect(UVec2::new(2, 2));
        let filter = filter_asset();

        draw_frame(&(rect, &filter), false, &mut rect_slice, None, []);

        let expected = vec![1, 1, 1, 1, 1, 2, 2, 1, 1, 2, 2, 1, 1, 1, 1, 1];
        assert_eq!(pixels(&image), expected);
    }

    #[test]
    fn rect_invert_draws_outside_only() {
        let mut image = PxImage::new(vec![1; 16], 4);
        let mut slice = image.slice_all_mut();
        let mut rect_slice = slice.slice_mut(IRect {
            min: ivec2(1, 1),
            max: ivec2(3, 3),
        });
        let rect = PxRect(UVec2::new(2, 2));
        let filter = filter_asset();

        draw_frame(&(rect, &filter), true, &mut rect_slice, None, []);

        let expected = vec![2, 2, 2, 2, 2, 1, 1, 2, 2, 1, 1, 2, 2, 2, 2, 2];
        assert_eq!(pixels(&image), expected);
    }
}

impl Spatial for (PxRect, &PxFilterAsset) {
    fn frame_size(&self) -> UVec2 {
        *self.0
    }
}

pub(crate) type RectComponents<L> = (
    &'static PxRect,
    &'static PxFilter,
    &'static PxFilterLayers<L>,
    &'static PxPosition,
    &'static PxAnchor,
    &'static PxCanvas,
    Option<&'static PxFrame>,
    Has<PxInvertMask>,
);

#[cfg(feature = "headed")]
fn extract_rects<L: PxLayer>(
    rects: Extract<Query<(RectComponents<L>, &InheritedVisibility, RenderEntity)>>,
    mut cmd: Commands,
) {
    for ((&rect, filter, layers, &pos, &anchor, &canvas, frame, invert), visibility, id) in &rects {
        let mut entity = cmd.entity(id);

        if !visibility.get() {
            entity.remove::<PxFilterLayers<L>>();
            continue;
        }

        entity.insert((rect, filter.clone(), layers.clone(), pos, anchor, canvas));

        if let Some(&frame) = frame {
            entity.insert(frame);
        } else {
            entity.remove::<PxFrame>();
        }

        if invert {
            entity.insert(PxInvertMask);
        } else {
            entity.remove::<PxInvertMask>();
        }
    }
}
