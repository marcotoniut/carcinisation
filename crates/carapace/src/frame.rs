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
#[derive(Clone, Copy, Reflect)]
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
#[derive(Clone, Copy, Debug, Default, Reflect)]
pub enum PxFrameTransition {
    /// Frames are not interpolated.
    #[default]
    None,
    /// Dithering is used to interpolate between frames, smoothing the animation.
    Dither,
}

/// Maps a master frame selection to a part-specific frame selection.
#[derive(Clone, Debug, Default, Reflect)]
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
#[derive(Component, Default, Clone, Copy, Reflect)]
pub struct PxFrameView {
    /// Frame selection mode.
    pub selector: PxFrameSelector,
    /// Frame interpolation mode.
    pub transition: PxFrameTransition,
}

/// Cached frame count for the entity's active frame source.
#[derive(Component, Default, Clone, Copy, Debug, Reflect)]
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
#[derive(Component, Default, Clone, Copy, Reflect)]
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

/// Draws a spatial element with a presentation transform (scale + rotation) applied.
///
/// Renders the sprite at native size into a scratch buffer, then blits it
/// to the destination with the combined scale/rotation transform around the
/// anchor point, using nearest-neighbour sampling.
///
/// Returns immediately if the source has zero-area frame size (unresolved
/// asset or degenerate geometry). No scratch buffer is allocated in that case.
pub(crate) fn draw_spatial_transformed<'a, A: Frames + Spatial>(
    spatial: &A,
    param: <A as Frames>::Param,
    image: &mut PxImageSliceMut,
    position: PxPosition,
    anchor: PxAnchor,
    canvas: PxCanvas,
    frame: Option<PxFrameView>,
    filters: impl IntoIterator<Item = &'a PxFilterAsset>,
    camera: PxCamera,
    scale: Vec2,
    rotation: f32,
    offset: Vec2,
) {
    let native_size = spatial.frame_size();
    if native_size.x == 0 || native_size.y == 0 {
        return;
    }

    // Render at native size into a scratch buffer.
    let mut scratch = crate::image::PxImage::empty(native_size);
    let mut scratch_slice = scratch.slice_all_mut();
    draw_frame(spatial, param, &mut scratch_slice, frame, filters);

    blit_transformed(
        &scratch,
        native_size,
        image,
        position,
        anchor,
        canvas,
        camera,
        scale,
        rotation,
        offset,
    );
}

/// Nearest-neighbour blit from a scratch buffer to a destination image slice,
/// applying scale, rotation, and offset around the anchor point.
///
/// Shared by both the single-sprite and composite transform paths.
///
/// **Signed scale**: negative values produce mirroring (horizontal and/or
/// vertical flip). The sign is carried through the inverse-transform math —
/// no separate flip path is needed.
///
/// The transform pipeline (per destination pixel):
/// 1. Express destination pixel relative to anchor in image space.
/// 2. Apply inverse rotation (negate angle).
/// 3. Apply inverse signed scale (negative inverts axis → mirror).
/// 4. Add anchor offset in source space → source pixel coordinate.
/// 5. Nearest-neighbour sample from scratch buffer.
pub(crate) fn blit_transformed(
    scratch: &crate::image::PxImage,
    native_size: UVec2,
    image: &mut PxImageSliceMut,
    position: PxPosition,
    anchor: PxAnchor,
    canvas: PxCanvas,
    camera: PxCamera,
    scale: Vec2,
    rotation: f32,
    offset: Vec2,
) {
    // Zero-area native_size means "nothing to draw" (e.g., unresolved asset,
    // collapsed transform result). Skip without allocating — this is the normal
    // empty-output path, not an error.
    if native_size.x == 0 || native_size.y == 0 {
        return;
    }

    let src_w = native_size.x as f32;
    let src_h = native_size.y as f32;

    // Anchor in source image space (top-left origin).
    // PxAnchor::pos returns (x, y) with y=0 at bottom, so flip y for image space.
    let anchor_world = anchor.pos(native_size).as_vec2();
    let anchor_src = Vec2::new(anchor_world.x, src_h - anchor_world.y);

    // Precompute sin/cos once per entity.
    let (sin_r, cos_r) = rotation.sin_cos();

    // Compute destination bounding box by transforming source corners around anchor.
    let corners = [
        Vec2::new(0.0, 0.0) - anchor_src,
        Vec2::new(src_w, 0.0) - anchor_src,
        Vec2::new(src_w, src_h) - anchor_src,
        Vec2::new(0.0, src_h) - anchor_src,
    ];

    let mut min_x = f32::MAX;
    let mut max_x = f32::MIN;
    let mut min_y = f32::MAX;
    let mut max_y = f32::MIN;

    for corner in &corners {
        let scaled = Vec2::new(corner.x * scale.x, corner.y * scale.y);
        let rotated = Vec2::new(
            scaled.x * cos_r - scaled.y * sin_r,
            scaled.x * sin_r + scaled.y * cos_r,
        );
        min_x = min_x.min(rotated.x);
        max_x = max_x.max(rotated.x);
        min_y = min_y.min(rotated.y);
        max_y = max_y.max(rotated.y);
    }

    // Destination extents in pixels (ceil + 1 to cover boundary pixels that
    // may land exactly on the edge when scale is negative). The extra pixel
    // on each side is harmless — the source-bounds check rejects any inverse
    // sample that falls outside the scratch buffer.
    let half_left = (-min_x).ceil() as i32 + 1;
    let half_right = max_x.ceil() as i32 + 1;
    let half_top = (-min_y).ceil() as i32 + 1;
    let half_bottom = max_y.ceil() as i32 + 1;

    if half_left + half_right <= 0 || half_top + half_bottom <= 0 {
        return;
    }

    // Anchor position in image space (top-left origin).
    let world_pos = *position + offset.round().as_ivec2();
    let world_pos = match canvas {
        PxCanvas::World => world_pos - *camera,
        PxCanvas::Camera => world_pos,
    };
    let anchor_img = IVec2::new(world_pos.x, image.height() as i32 - world_pos.y);

    // Destination rectangle around anchor.
    let dest_min = IVec2::new(anchor_img.x - half_left, anchor_img.y - half_top);
    let dest_max = IVec2::new(anchor_img.x + half_right, anchor_img.y + half_bottom);

    // Clamp to image bounds.
    let img_w_i = image.image_width() as i32;
    let img_h_i = image.image_height() as i32;
    let x_min = dest_min.x.clamp(0, img_w_i);
    let x_max = dest_max.x.clamp(0, img_w_i);
    let y_min = dest_min.y.clamp(0, img_h_i);
    let y_max = dest_max.y.clamp(0, img_h_i);

    // Precompute inverse scale.
    let inv_sx = 1.0 / scale.x;
    let inv_sy = 1.0 / scale.y;

    let src_w_i = native_size.x as i32;
    let src_h_i = native_size.y as i32;

    // Nearest-neighbour blit with inverse transform.
    for dy in y_min..y_max {
        for dx in x_min..x_max {
            // Destination pixel relative to anchor in image space.
            let rel_x = (dx - anchor_img.x) as f32;
            let rel_y = (dy - anchor_img.y) as f32;

            // Inverse rotation.
            let unrot_x = rel_x * cos_r + rel_y * sin_r;
            let unrot_y = -rel_x * sin_r + rel_y * cos_r;

            // Inverse scale + re-centre on source anchor.
            let src_x = (unrot_x * inv_sx + anchor_src.x).round() as i32;
            let src_y = (unrot_y * inv_sy + anchor_src.y).round() as i32;

            if src_x >= 0
                && src_x < src_w_i
                && src_y >= 0
                && src_y < src_h_i
                && let Some(pixel) = scratch.get_pixel(IVec2::new(src_x, src_y))
                && pixel != 0
                && let Some(dest) = image.get_pixel_mut(IVec2::new(dx, dy))
            {
                *dest = pixel;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image::PxImage;

    /// Helper: create a scratch image filled with a single palette index.
    fn solid_scratch(w: u32, h: u32, color: u8) -> PxImage {
        PxImage::new(vec![color; (w * h) as usize], w as usize)
    }

    /// Helper: count non-zero pixels in an image.
    fn count_nonzero(img: &PxImage) -> usize {
        let size = img.size();
        (0..size.y as i32)
            .flat_map(|y| (0..size.x as i32).map(move |x| IVec2::new(x, y)))
            .filter(|&pos| img.get_pixel(pos).is_some_and(|p| p != 0))
            .count()
    }

    /// Helper: blit a tiny source pattern through `blit_transformed` and return
    /// the non-zero pixels as a row-major grid.
    ///
    /// Uses a padded destination with the source centred, then extracts the
    /// tight bounding box of non-zero pixels.
    fn blit_grid(src: &[u8], w: u32, h: u32, scale: Vec2, rotation: f32) -> Vec<Vec<u8>> {
        let scratch = PxImage::new(src.to_vec(), w as usize);
        let pad = ((w.max(h) as f32) * scale.x.abs().max(scale.y.abs())).ceil() as u32 + 4;
        let dest_side = pad * 2;
        let mut dest = PxImage::empty(UVec2::splat(dest_side));
        let mut dest_slice = dest.slice_all_mut();
        let center = dest_side as i32 / 2;
        blit_transformed(
            &scratch,
            UVec2::new(w, h),
            &mut dest_slice,
            PxPosition(IVec2::new(center, center)),
            PxAnchor::Center,
            PxCanvas::Camera,
            PxCamera(IVec2::ZERO),
            scale,
            rotation,
            Vec2::ZERO,
        );
        dest.nonzero_grid()
    }

    // ---- Pixel-count / tolerance tests ----
    //
    // These verify that transforms preserve or change pixel coverage within
    // expected ranges. They use uniform-colour sources so they only test
    // coverage, not orientation. Exact layout correctness is covered by the
    // `exact_*` tests below.

    #[test]
    fn identity_scale_no_rotation_preserves_pixel_count() {
        let scratch = solid_scratch(4, 4, 1);
        let mut dest = PxImage::empty(UVec2::new(16, 16));
        let mut dest_slice = dest.slice_all_mut();

        blit_transformed(
            &scratch,
            UVec2::new(4, 4),
            &mut dest_slice,
            PxPosition(IVec2::new(8, 8)),
            PxAnchor::Center,
            PxCanvas::Camera,
            PxCamera(IVec2::ZERO),
            Vec2::ONE,
            0.0,
            Vec2::ZERO,
        );

        assert_eq!(count_nonzero(&dest), 16); // 4x4 = 16 pixels
    }

    #[test]
    fn scale_2x_approximately_quadruples_pixel_count() {
        let scratch = solid_scratch(4, 4, 1);
        let mut dest = PxImage::empty(UVec2::new(32, 32));
        let mut dest_slice = dest.slice_all_mut();

        blit_transformed(
            &scratch,
            UVec2::new(4, 4),
            &mut dest_slice,
            PxPosition(IVec2::new(16, 16)),
            PxAnchor::Center,
            PxCanvas::Camera,
            PxCamera(IVec2::ZERO),
            Vec2::splat(2.0),
            0.0,
            Vec2::ZERO,
        );

        // Nearest-neighbour rounding at anchor boundaries can lose a few
        // edge pixels. Expect roughly 4x coverage (64), within tolerance.
        let n = count_nonzero(&dest);
        assert!(
            (48..=64).contains(&n),
            "2x scale should produce ~64 pixels, got {n}"
        );
    }

    #[test]
    fn rotation_90_approximately_preserves_pixel_count() {
        // Use a larger sprite to reduce rounding noise at boundaries.
        let scratch = solid_scratch(8, 8, 1);
        let mut dest = PxImage::empty(UVec2::new(32, 32));
        let mut dest_slice = dest.slice_all_mut();

        blit_transformed(
            &scratch,
            UVec2::new(8, 8),
            &mut dest_slice,
            PxPosition(IVec2::new(16, 16)),
            PxAnchor::Center,
            PxCanvas::Camera,
            PxCamera(IVec2::ZERO),
            Vec2::ONE,
            std::f32::consts::FRAC_PI_2, // 90°
            Vec2::ZERO,
        );

        // A square rotated 90° should produce approximately the same count.
        // Small rounding variance at edges is expected.
        let n = count_nonzero(&dest);
        assert!(
            (56..=72).contains(&n),
            "90° rotation of 8x8 should produce ~64 pixels, got {n}"
        );
    }

    #[test]
    fn rotation_45_expands_bounding_box() {
        // A 4x4 square at 45° should fill ~22-23 pixels (rotated diamond),
        // more than the original 16, confirming bounding box expansion works.
        let scratch = solid_scratch(4, 4, 1);
        let mut dest = PxImage::empty(UVec2::new(16, 16));
        let mut dest_slice = dest.slice_all_mut();

        blit_transformed(
            &scratch,
            UVec2::new(4, 4),
            &mut dest_slice,
            PxPosition(IVec2::new(8, 8)),
            PxAnchor::Center,
            PxCanvas::Camera,
            PxCamera(IVec2::ZERO),
            Vec2::ONE,
            std::f32::consts::FRAC_PI_4, // 45°
            Vec2::ZERO,
        );

        let n = count_nonzero(&dest);
        assert!(n > 16, "45° rotation should expand coverage, got {n}");
        assert!(n < 40, "45° rotation shouldn't overshoot, got {n}");
    }

    #[test]
    fn zero_native_size_draws_nothing() {
        // Zero-area native_size triggers an early return in blit_transformed.
        // The scratch buffer is never accessed, so we use a minimal valid 1×1
        // image — PxImage requires width > 0.
        let scratch = PxImage::new(vec![1], 1);
        let mut dest = PxImage::empty(UVec2::new(8, 8));
        let mut dest_slice = dest.slice_all_mut();

        blit_transformed(
            &scratch,
            UVec2::ZERO,
            &mut dest_slice,
            PxPosition(IVec2::new(4, 4)),
            PxAnchor::Center,
            PxCanvas::Camera,
            PxCamera(IVec2::ZERO),
            Vec2::ONE,
            0.0,
            Vec2::ZERO,
        );

        assert_eq!(count_nonzero(&dest), 0);
    }

    #[test]
    fn zero_width_native_size_draws_nothing() {
        // Only one dimension is zero — still early-returns.
        let scratch = PxImage::new(vec![1], 1);
        let mut dest = PxImage::empty(UVec2::new(8, 8));
        let mut dest_slice = dest.slice_all_mut();

        blit_transformed(
            &scratch,
            UVec2::new(0, 4),
            &mut dest_slice,
            PxPosition(IVec2::new(4, 4)),
            PxAnchor::Center,
            PxCanvas::Camera,
            PxCamera(IVec2::ZERO),
            Vec2::ONE,
            0.0,
            Vec2::ZERO,
        );

        assert_eq!(count_nonzero(&dest), 0);
    }

    #[test]
    fn zero_height_native_size_draws_nothing() {
        let scratch = PxImage::new(vec![1], 1);
        let mut dest = PxImage::empty(UVec2::new(8, 8));
        let mut dest_slice = dest.slice_all_mut();

        blit_transformed(
            &scratch,
            UVec2::new(4, 0),
            &mut dest_slice,
            PxPosition(IVec2::new(4, 4)),
            PxAnchor::Center,
            PxCanvas::Camera,
            PxCamera(IVec2::ZERO),
            Vec2::ONE,
            0.0,
            Vec2::ZERO,
        );

        assert_eq!(count_nonzero(&dest), 0);
    }

    #[test]
    fn offset_shifts_without_changing_pixel_count() {
        let scratch = solid_scratch(4, 4, 1);
        let mut dest_a = PxImage::empty(UVec2::new(16, 16));
        let mut dest_b = PxImage::empty(UVec2::new(16, 16));
        let mut slice_a = dest_a.slice_all_mut();
        let mut slice_b = dest_b.slice_all_mut();

        // Without offset.
        blit_transformed(
            &scratch,
            UVec2::new(4, 4),
            &mut slice_a,
            PxPosition(IVec2::new(8, 8)),
            PxAnchor::Center,
            PxCanvas::Camera,
            PxCamera(IVec2::ZERO),
            Vec2::ONE,
            0.0,
            Vec2::ZERO,
        );

        // With offset.
        blit_transformed(
            &scratch,
            UVec2::new(4, 4),
            &mut slice_b,
            PxPosition(IVec2::new(8, 8)),
            PxAnchor::Center,
            PxCanvas::Camera,
            PxCamera(IVec2::ZERO),
            Vec2::ONE,
            0.0,
            Vec2::new(2.0, -1.0),
        );

        assert_eq!(count_nonzero(&dest_a), count_nonzero(&dest_b));
    }

    #[test]
    fn horizontal_flip_preserves_pixel_count() {
        let scratch = solid_scratch(4, 4, 1);
        let mut dest = PxImage::empty(UVec2::new(16, 16));
        let mut dest_slice = dest.slice_all_mut();

        blit_transformed(
            &scratch,
            UVec2::new(4, 4),
            &mut dest_slice,
            PxPosition(IVec2::new(8, 8)),
            PxAnchor::Center,
            PxCanvas::Camera,
            PxCamera(IVec2::ZERO),
            Vec2::new(-1.0, 1.0), // horizontal flip
            0.0,
            Vec2::ZERO,
        );

        assert_eq!(count_nonzero(&dest), 16);
    }

    #[test]
    fn vertical_flip_preserves_pixel_count() {
        let scratch = solid_scratch(4, 4, 1);
        let mut dest = PxImage::empty(UVec2::new(16, 16));
        let mut dest_slice = dest.slice_all_mut();

        blit_transformed(
            &scratch,
            UVec2::new(4, 4),
            &mut dest_slice,
            PxPosition(IVec2::new(8, 8)),
            PxAnchor::Center,
            PxCanvas::Camera,
            PxCamera(IVec2::ZERO),
            Vec2::new(1.0, -1.0), // vertical flip
            0.0,
            Vec2::ZERO,
        );

        assert_eq!(count_nonzero(&dest), 16);
    }

    #[test]
    fn horizontal_flip_mirrors_content() {
        // Asymmetric source: left column = 1, right column = 2.
        // Layout in row-major: [row0: 1, 2], [row1: 1, 2]
        let scratch = PxImage::new(vec![1, 2, 1, 2], 2);

        // Unflipped reference.
        let mut dest_normal = PxImage::empty(UVec2::new(8, 8));
        let mut slice_normal = dest_normal.slice_all_mut();
        blit_transformed(
            &scratch,
            UVec2::new(2, 2),
            &mut slice_normal,
            PxPosition(IVec2::new(4, 4)),
            PxAnchor::Center,
            PxCanvas::Camera,
            PxCamera(IVec2::ZERO),
            Vec2::ONE,
            0.0,
            Vec2::ZERO,
        );

        // Flipped.
        let mut dest_flip = PxImage::empty(UVec2::new(8, 8));
        let mut slice_flip = dest_flip.slice_all_mut();
        blit_transformed(
            &scratch,
            UVec2::new(2, 2),
            &mut slice_flip,
            PxPosition(IVec2::new(4, 4)),
            PxAnchor::Center,
            PxCanvas::Camera,
            PxCamera(IVec2::ZERO),
            Vec2::new(-1.0, 1.0),
            0.0,
            Vec2::ZERO,
        );

        // The two results should have the same pixel count but different layout.
        assert_eq!(count_nonzero(&dest_normal), count_nonzero(&dest_flip));
        // Content should differ (mirrored).
        let mut differs = false;
        for y in 0..8_i32 {
            for x in 0..8_i32 {
                let pos = IVec2::new(x, y);
                if dest_normal.get_pixel(pos) != dest_flip.get_pixel(pos) {
                    differs = true;
                }
            }
        }
        assert!(differs, "flipped content should differ from unflipped");
    }

    #[test]
    fn flip_with_rotation_preserves_pixel_count() {
        let scratch = solid_scratch(4, 4, 1);
        let mut dest = PxImage::empty(UVec2::new(16, 16));
        let mut dest_slice = dest.slice_all_mut();

        blit_transformed(
            &scratch,
            UVec2::new(4, 4),
            &mut dest_slice,
            PxPosition(IVec2::new(8, 8)),
            PxAnchor::Center,
            PxCanvas::Camera,
            PxCamera(IVec2::ZERO),
            Vec2::new(-1.0, -1.0),       // both axes flipped
            std::f32::consts::FRAC_PI_2, // + 90° rotation
            Vec2::ZERO,
        );

        // A 4x4 square with flip+90° should still produce ~16 pixels.
        let n = count_nonzero(&dest);
        assert!(
            (12..=20).contains(&n),
            "flip + 90° rotation should produce ~16 pixels, got {n}"
        );
    }

    #[test]
    fn negative_scale_2x_flip_approximately_quadruples_pixel_count() {
        let scratch = solid_scratch(4, 4, 1);
        let mut dest = PxImage::empty(UVec2::new(32, 32));
        let mut dest_slice = dest.slice_all_mut();

        blit_transformed(
            &scratch,
            UVec2::new(4, 4),
            &mut dest_slice,
            PxPosition(IVec2::new(16, 16)),
            PxAnchor::Center,
            PxCanvas::Camera,
            PxCamera(IVec2::ZERO),
            Vec2::splat(-2.0), // 2x scale + flip both axes
            0.0,
            Vec2::ZERO,
        );

        let n = count_nonzero(&dest);
        assert!(
            (48..=64).contains(&n),
            "-2x scale should produce ~64 pixels, got {n}"
        );
    }

    // ---- Exact pixel-grid matrix tests ----
    //
    // Use a 2x2 asymmetric pattern with 4 distinct values so any orientation
    // mistake (wrong axis, wrong direction) is immediately visible.
    //
    //   Source (image-space, top-left origin):
    //     1 2
    //     3 4

    #[test]
    fn exact_identity() {
        let grid = blit_grid(&[1, 2, 3, 4], 2, 2, Vec2::ONE, 0.0);
        assert_eq!(grid, vec![vec![1, 2], vec![3, 4]]);
    }

    #[test]
    fn exact_horizontal_flip_via_signed_scale() {
        // H-flip mirrors columns: 1↔2, 3↔4.
        let grid = blit_grid(&[1, 2, 3, 4], 2, 2, Vec2::new(-1.0, 1.0), 0.0);
        assert_eq!(grid, vec![vec![2, 1], vec![4, 3]]);
    }

    #[test]
    fn exact_vertical_flip_via_signed_scale() {
        // V-flip mirrors rows: row0↔row1.
        let grid = blit_grid(&[1, 2, 3, 4], 2, 2, Vec2::new(1.0, -1.0), 0.0);
        assert_eq!(grid, vec![vec![3, 4], vec![1, 2]]);
    }

    #[test]
    fn exact_both_flip_via_signed_scale() {
        // Both axes flipped = 180° rotation.
        let grid = blit_grid(&[1, 2, 3, 4], 2, 2, Vec2::new(-1.0, -1.0), 0.0);
        assert_eq!(grid, vec![vec![4, 3], vec![2, 1]]);
    }

    #[test]
    fn exact_rotation_180() {
        // 180° rotation should produce the same result as flipping both axes.
        let grid = blit_grid(&[1, 2, 3, 4], 2, 2, Vec2::ONE, std::f32::consts::PI);
        assert_eq!(grid, vec![vec![4, 3], vec![2, 1]]);
    }

    #[test]
    fn exact_rotation_90_ccw() {
        // 90° counter-clockwise rotation (image-space, top-left origin).
        // Source:  1 2    Result:  3 1
        //          3 4             4 2
        let grid = blit_grid(&[1, 2, 3, 4], 2, 2, Vec2::ONE, std::f32::consts::FRAC_PI_2);
        assert_eq!(grid, vec![vec![3, 1], vec![4, 2]]);
    }

    #[test]
    fn exact_rotation_270_ccw() {
        // 270° CCW = 90° CW (image-space, top-left origin).
        // Source:  1 2    Result:  2 4
        //          3 4             1 3
        let grid = blit_grid(
            &[1, 2, 3, 4],
            2,
            2,
            Vec2::ONE,
            3.0 * std::f32::consts::FRAC_PI_2,
        );
        assert_eq!(grid, vec![vec![2, 4], vec![1, 3]]);
    }

    #[test]
    fn exact_scale_2x() {
        // 2x nearest-neighbour scaling. Center anchor on a 2x2 source rounds
        // to a 3x3 output (boundary pixel lands outside the half-pixel centre).
        let grid = blit_grid(&[1, 2, 3, 4], 2, 2, Vec2::splat(2.0), 0.0);
        assert_eq!(grid, vec![vec![1, 2, 2], vec![3, 4, 4], vec![3, 4, 4],]);
    }

    #[test]
    fn exact_hflip_plus_90_ccw() {
        // Horizontal flip then 90° CCW rotation.
        // (blit applies inverse(rot) then inverse(scale), equivalent to
        //  scale-then-rotate in forward order.)
        let grid = blit_grid(
            &[1, 2, 3, 4],
            2,
            2,
            Vec2::new(-1.0, 1.0),
            std::f32::consts::FRAC_PI_2,
        );
        assert_eq!(grid, vec![vec![4, 2], vec![3, 1]]);
    }

    #[test]
    fn exact_3x2_hflip() {
        // Non-square source: 3 wide × 2 tall.
        //   1 2 3       3 2 1
        //   4 5 6  →    6 5 4
        let grid = blit_grid(&[1, 2, 3, 4, 5, 6], 3, 2, Vec2::new(-1.0, 1.0), 0.0);
        assert_eq!(grid, vec![vec![3, 2, 1], vec![6, 5, 4]]);
    }

    #[test]
    fn exact_3x2_vflip() {
        //   1 2 3       4 5 6
        //   4 5 6  →    1 2 3
        let grid = blit_grid(&[1, 2, 3, 4, 5, 6], 3, 2, Vec2::new(1.0, -1.0), 0.0);
        assert_eq!(grid, vec![vec![4, 5, 6], vec![1, 2, 3]]);
    }

    #[test]
    fn exact_hflip_scale_2x() {
        // H-flip + 2x scale. Same center-anchor rounding as exact_scale_2x.
        let grid = blit_grid(&[1, 2, 3, 4], 2, 2, Vec2::new(-2.0, 2.0), 0.0);
        assert_eq!(grid, vec![vec![2, 2, 1], vec![4, 4, 3], vec![4, 4, 3],]);
    }
}
