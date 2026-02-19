//! Frame selection and drawing helpers.

use crate::{
    animation::PxAnimation, filter::PxFilterAsset, image::PxImageSliceMut, position::Spatial,
    prelude::*, set::PxSet,
};

pub(crate) fn plug(app: &mut App) {
    app.add_systems(
        PostUpdate,
        seed_animation_control.before(PxSet::FinishAnimations),
    )
    .add_systems(PostUpdate, sync_frame_view.after(PxSet::FinishAnimations));
}

/// Selects a frame by absolute index or normalized progress.
#[derive(Clone, Copy)]
pub enum PxFrameSelector {
    /// Direct frame index (may be fractional for transitions).
    Index(f32),
    /// Normalized progress from 0.0 to 1.0.
    Normalized(f32),
}

impl Default for PxFrameSelector {
    fn default() -> Self {
        Self::Normalized(0.)
    }
}

/// Method the animation uses to interpolate between frames.
#[derive(Clone, Copy, Debug, Default)]
pub enum PxFrameTransition {
    /// Frames are not interpolated.
    #[default]
    None,
    /// Dithering is used to interpolate between frames, smoothing the animation.
    Dither,
}

/// Maps a master frame selection to a part-specific frame selection.
#[derive(Clone, Debug, Default)]
pub enum PxFrameBinding {
    /// Use the master's normalized progress.
    #[default]
    Inherit,
    /// Map master frame indices to explicit part frame indices.
    Map(Vec<usize>),
    /// Offset the master frame index by the given amount.
    Offset(i32),
    /// Scale the master's normalized progress.
    Scale(f32),
}

/// Per-entity frame view consumed by drawables.
#[derive(Component, Default, Clone, Copy)]
pub struct PxFrameView {
    /// Frame selection mode.
    pub selector: PxFrameSelector,
    /// Frame interpolation mode.
    pub transition: PxFrameTransition,
}

/// Cached frame count for the entity's active frame source.
#[derive(Component, Default, Clone, Copy, Debug)]
pub struct PxFrameCount(pub usize);

/// Backwards-compatible alias for the frame view.
pub type PxFrame = PxFrameView;

impl From<PxFrameSelector> for PxFrameView {
    fn from(value: PxFrameSelector) -> Self {
        Self {
            selector: value,
            ..default()
        }
    }
}

/// Per-entity frame control input (e.g., for animation or manual control).
#[derive(Component, Default, Clone, Copy)]
pub struct PxFrameControl {
    /// Frame selection mode.
    pub selector: PxFrameSelector,
    /// Frame interpolation mode.
    pub transition: PxFrameTransition,
}

impl From<PxFrameSelector> for PxFrameControl {
    fn from(value: PxFrameSelector) -> Self {
        Self {
            selector: value,
            ..default()
        }
    }
}

impl From<PxFrameView> for PxFrameControl {
    fn from(value: PxFrameView) -> Self {
        Self {
            selector: value.selector,
            transition: value.transition,
        }
    }
}

impl From<PxFrameControl> for PxFrameView {
    fn from(value: PxFrameControl) -> Self {
        Self {
            selector: value.selector,
            transition: value.transition,
        }
    }
}

fn sync_frame_view(
    mut commands: Commands,
    mut frames: Query<(Entity, &PxFrameControl, Option<&mut PxFrameView>), Changed<PxFrameControl>>,
) {
    for (entity, control, view) in &mut frames {
        match view {
            Some(mut view) => {
                *view = (*control).into();
            }
            None => {
                commands.entity(entity).insert(PxFrameView::from(*control));
            }
        }
    }
}

fn seed_animation_control(
    mut animations: Query<(&PxFrameView, &mut PxFrameControl), Added<PxAnimation>>,
) {
    for (view, mut control) in &mut animations {
        *control = (*view).into();
    }
}

pub(crate) trait Frames {
    type Param;

    fn frame_count(&self) -> usize;
    fn draw(
        &self,
        param: Self::Param,
        image: &mut PxImageSliceMut,
        frame: impl Fn(UVec2) -> usize,
        filter: impl Fn(u8) -> u8,
    );
}

const DITHERING: [u16; 16] = [
    0b0000_0000_0000_0000,
    0b1000_0000_0000_0000,
    0b1000_0000_0010_0000,
    0b1010_0000_0010_0000,
    0b1010_0000_1010_0000,
    0b1010_0100_1010_0000,
    0b1010_0100_1010_0001,
    0b1010_0101_1010_0001,
    0b1010_0101_1010_0101,
    0b1110_0101_1010_0101,
    0b1110_0101_1011_0101,
    0b1111_0101_1011_0101,
    0b1111_0101_1111_0101,
    0b1111_1101_1111_0101,
    0b1111_1101_1111_0111,
    0b1111_1111_1111_0111,
];

pub(crate) fn animate(frame: PxFrameView, frame_count: usize) -> impl Fn(UVec2) -> usize {
    let index = match frame.selector {
        PxFrameSelector::Normalized(frame) => frame * (frame_count - 1) as f32,
        PxFrameSelector::Index(frame) => frame,
    };

    let dithering = match frame.transition {
        PxFrameTransition::Dither => DITHERING[(index.fract() * 16.) as usize % 16],
        PxFrameTransition::None => 0,
    };
    let index = index.floor() as usize;

    move |pos| {
        (index
            + usize::from((0b1000_0000_0000_0000 >> (pos.x % 4 + pos.y % 4 * 4)) & dithering != 0))
            % frame_count
    }
}

fn frame_index_f32(frame: PxFrameView, frame_count: usize) -> f32 {
    match frame.selector {
        PxFrameSelector::Normalized(progress) => {
            if frame_count <= 1 {
                0.
            } else {
                progress.clamp(0., 1.) * (frame_count - 1) as f32
            }
        }
        PxFrameSelector::Index(index) => index.max(0.),
    }
}

fn frame_index(frame: PxFrameView, frame_count: usize) -> usize {
    frame_index_f32(frame, frame_count).floor() as usize
}

fn frame_progress(frame: PxFrameView, frame_count: usize) -> f32 {
    match frame.selector {
        PxFrameSelector::Normalized(progress) => progress.clamp(0., 1.),
        PxFrameSelector::Index(index) => {
            if frame_count <= 1 {
                0.
            } else {
                (index.max(0.) / (frame_count - 1) as f32).clamp(0., 1.)
            }
        }
    }
}

pub(crate) fn resolve_frame_binding(
    master: Option<PxFrameView>,
    master_count: usize,
    part_count: usize,
    binding: &PxFrameBinding,
) -> Option<PxFrameView> {
    let master = master?;
    if part_count == 0 {
        return None;
    }

    match binding {
        PxFrameBinding::Inherit => Some(PxFrameView {
            selector: PxFrameSelector::Normalized(frame_progress(master, master_count)),
            transition: master.transition,
        }),
        PxFrameBinding::Map(map) => {
            if map.is_empty() {
                return None;
            }
            let index = frame_index(master, map.len());
            let mapped = map.get(index).copied().unwrap_or(0) as f32;
            Some(PxFrameView {
                selector: PxFrameSelector::Index(mapped),
                transition: master.transition,
            })
        }
        PxFrameBinding::Offset(offset) => {
            let index = frame_index(master, master_count) as i32 + offset;
            let index = index.rem_euclid(part_count as i32) as f32;
            Some(PxFrameView {
                selector: PxFrameSelector::Index(index),
                transition: master.transition,
            })
        }
        PxFrameBinding::Scale(scale) => {
            let progress = frame_progress(master, master_count) * *scale;
            let progress = progress - progress.floor();
            Some(PxFrameView {
                selector: PxFrameSelector::Normalized(progress),
                transition: master.transition,
            })
        }
    }
}

pub(crate) fn draw_frame<'a, A: Frames>(
    animation: &A,
    param: A::Param,
    image: &mut PxImageSliceMut,
    frame: Option<PxFrameView>,
    filters: impl IntoIterator<Item = &'a PxFilterAsset>,
) {
    let frame_count = animation.frame_count();
    if frame_count == 0 {
        return;
    }

    let mut filter: Box<dyn Fn(u8) -> u8> = Box::new(|pixel| pixel);
    for filter_part in filters {
        let filter_part = filter_part.as_fn();
        filter = Box::new(move |pixel| filter_part(filter(pixel)));
    }

    if let Some(frame) = frame {
        let frame = animate(frame, frame_count);

        animation.draw(param, image, frame, filter);
    } else {
        let frame = |_| 0;
        animation.draw(param, image, frame, filter);
    }
}

pub(crate) fn draw_spatial<'a, A: Frames + Spatial>(
    spatial: &A,
    param: <A as Frames>::Param,
    image: &mut PxImageSliceMut,
    position: PxPosition,
    anchor: PxAnchor,
    canvas: PxCanvas,
    frame: Option<PxFrameView>,
    filters: impl IntoIterator<Item = &'a PxFilterAsset>,
    camera: PxCamera,
) {
    // Coordinate convention: image space has origin at top-left.
    // World/camera positions are bottom-left, so Y is flipped here.
    let size = spatial.frame_size();
    let position = *position - anchor.pos(size).as_ivec2();
    let position = match canvas {
        PxCanvas::World => position - *camera,
        PxCanvas::Camera => position,
    };
    let position = IVec2::new(position.x, image.height() as i32 - position.y);
    let size = size.as_ivec2();

    let mut image = image.slice_mut(IRect {
        min: position - IVec2::new(0, size.y),
        max: position + IVec2::new(size.x, 0),
    });

    draw_frame(spatial, param, &mut image, frame, filters);
}
