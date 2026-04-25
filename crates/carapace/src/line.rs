use bevy_derive::{Deref, DerefMut};
use bevy_math::{ivec2, uvec2};
use bevy_platform::collections::HashSet;
use bevy_render::{Extract, RenderApp, sync_world::RenderEntity};
use line_drawing::Bresenham;

use crate::{
    filter::DefaultCxFilterLayers,
    frame::{Frames, draw_frame},
    image::CxImageSliceMut,
    position::{CxLayer, Spatial},
    prelude::*,
};

pub(crate) fn plug<L: CxLayer>(app: &mut App) {
    app.sub_app_mut(RenderApp)
        .add_systems(ExtractSchedule, extract_lines::<L>);
}

/// Point list for a line
#[derive(Component, Deref, DerefMut, Clone, Default, Debug, Reflect)]
#[require(DefaultCxFilterLayers, CxRenderSpace)]
pub struct CxLine(pub Vec<IVec2>);

impl Spatial for CxLine {
    fn frame_size(&self) -> UVec2 {
        if self.is_empty() {
            return UVec2::ZERO;
        }

        let (min, max) = self
            .iter()
            .copied()
            .fold((self[0], self[0]), |(min, max), point| {
                (min.min(point), max.max(point))
            });

        (max - min).as_uvec2()
    }
}

impl Frames for (&CxLine, &CxFilterAsset) {
    type Param = (IVec2, bool);

    fn frame_count(&self) -> usize {
        let (_, CxFilterAsset(filter)) = self;
        filter.area() / filter.width()
    }

    fn draw(
        &self,
        (offset, invert): Self::Param,
        image: &mut CxImageSliceMut,
        frame: impl Fn(UVec2) -> usize,
        _: impl Fn(u8) -> u8,
    ) {
        let (line, CxFilterAsset(filter)) = self;
        let slice_offset = image.offset();
        let image_width = image.img_width_i();
        let image_height = image.img_height_i();

        if invert {
            let mut line_points = Vec::new();

            for (segment_index, (start, end)) in line.iter().zip(line.iter().skip(1)).enumerate() {
                let start = *start + offset;
                let end = *end + offset;

                for (step, pos) in Bresenham::new(start.into(), end.into()).enumerate() {
                    if segment_index > 0 && step == 0 {
                        continue;
                    }

                    line_points.push(IVec2::from(pos));
                }
            }

            let mut originals = Vec::with_capacity(line_points.len());

            for world_pos in &line_points {
                let pos = *world_pos + slice_offset;

                if pos.x < 0 || pos.y < 0 || pos.x >= image_width || pos.y >= image_height {
                    continue;
                }

                let pixel = *image.abs_pixel_mut(pos);
                originals.push((pos, pixel));
            }

            for y in 0..image_height {
                for x in 0..image_width {
                    let pos = ivec2(x, y);
                    let pixel = image.abs_pixel_mut(pos);
                    *pixel = filter.pixel(ivec2(
                        i32::from(*pixel),
                        frame(uvec2(x as u32, y as u32)) as i32,
                    ));
                }
            }

            for (pos, pixel) in originals {
                *image.abs_pixel_mut(pos) = pixel;
            }
        } else {
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

            for world_pos in poses {
                let pos = world_pos + slice_offset;

                if pos.x < 0 || pos.y < 0 || pos.x >= image_width || pos.y >= image_height {
                    continue;
                }

                let pixel = image.abs_pixel_mut(pos);
                *pixel = filter.pixel(ivec2(
                    i32::from(*pixel),
                    frame(uvec2(pos.x as u32, pos.y as u32)) as i32,
                ));
            }
        }
    }
}

impl<T: IntoIterator<Item = IVec2>> From<T> for CxLine {
    fn from(line: T) -> Self {
        Self(line.into_iter().collect())
    }
}

pub(crate) type LineComponents<L> = (
    &'static CxLine,
    &'static CxFilter,
    &'static CxFilterLayers<L>,
    &'static CxRenderSpace,
    Option<&'static CxFrameView>,
    Has<CxInvertMask>,
);

fn extract_lines<L: CxLayer>(
    lines: Extract<Query<(LineComponents<L>, &InheritedVisibility, RenderEntity)>>,
    mut cmd: Commands,
) {
    for ((line, filter, layers, &canvas, frame, invert), visibility, id) in &lines {
        let mut entity = cmd.entity(id);

        if !visibility.get() {
            entity.remove::<CxFilterLayers<L>>();
            continue;
        }

        entity.insert((line.clone(), filter.clone(), layers.clone(), canvas));

        if let Some(&frame) = frame {
            entity.insert(frame);
        } else {
            entity.remove::<CxFrameView>();
        }

        if invert {
            entity.insert(CxInvertMask);
        } else {
            entity.remove::<CxInvertMask>();
        }
    }
}

pub(crate) fn draw_line(
    line: &CxLine,
    filter: &CxFilterAsset,
    invert: bool,
    image: &mut CxImageSliceMut,
    canvas: CxRenderSpace,
    frame: Option<CxFrameView>,
    camera: CxCamera,
) {
    // TODO Make an `animated_line` example
    draw_frame(
        &(line, filter),
        (
            match canvas {
                CxRenderSpace::World => -*camera,
                CxRenderSpace::Camera => IVec2::ZERO,
            },
            invert,
        ),
        image,
        frame,
        [],
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{camera::CxCamera, filter::CxFilterAsset, image::CxImage};

    fn filter_asset() -> CxFilterAsset {
        CxFilterAsset(CxImage::new(vec![0, 2, 0, 0], 4))
    }

    fn pixels(image: &CxImage) -> Vec<u8> {
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
    fn line_draws_only_line_pixels() {
        let mut image = CxImage::new(vec![1; 25], 5);
        let mut slice = image.slice_all_mut();
        let filter = filter_asset();
        let line = CxLine(vec![IVec2::new(1, 1), IVec2::new(3, 1)]);

        draw_line(
            &line,
            &filter,
            false,
            &mut slice,
            CxRenderSpace::Camera,
            None,
            CxCamera::default(),
        );

        let expected = vec![
            1, 1, 1, 1, 1, 1, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        ];

        assert_eq!(pixels(&image), expected);
    }

    #[test]
    fn line_invert_draws_outside_only() {
        let mut image = CxImage::new(vec![1; 25], 5);
        let mut slice = image.slice_all_mut();
        let filter = filter_asset();
        let line = CxLine(vec![IVec2::new(1, 1), IVec2::new(3, 1)]);

        draw_line(
            &line,
            &filter,
            true,
            &mut slice,
            CxRenderSpace::Camera,
            None,
            CxCamera::default(),
        );

        let expected = vec![
            2, 2, 2, 2, 2, 2, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2,
        ];

        assert_eq!(pixels(&image), expected);
    }
}
