use bevy::prelude::*;
use seldom_pixel::{asset::*, filter::*, prelude::*};

use crate::{globals::SCREEN_RESOLUTION, Layer};

use super::components::LetterboxRow;

pub fn make_letterbox_row(
    filter: Handle<PxAsset<PxFilterData>>,
    row: u32,
) -> (PxLineBundle<Layer>, LetterboxRow, Name) {
    (
        PxLineBundle::<Layer> {
            canvas: PxCanvas::Camera,
            line: [
                (0, row as i32).into(),
                (SCREEN_RESOLUTION.x as i32, row as i32).into(),
            ]
            .into(),
            layers: PxFilterLayers::single_over(Layer::Letterbox),
            filter,
            ..Default::default()
        },
        LetterboxRow { row },
        Name::new("LetterboxRow"),
    )
}
