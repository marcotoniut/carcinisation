use bevy::prelude::*;
use seldom_pixel::{asset::*, filter::*, prelude::*};

use crate::{globals::SCREEN_RESOLUTION, Layer};

use super::{components::TransitionVenetian, resources::*};

pub fn make_transition_venetian_row(
    filter: Handle<PxAsset<PxFilterData>>,
    row: u32,
) -> (PxLineBundle<Layer>, TransitionVenetian, Name) {
    (
        PxLineBundle::<Layer> {
            canvas: PxCanvas::Camera,
            line: [
                (0, row as i32).into(),
                (SCREEN_RESOLUTION.x as i32, row as i32).into(),
            ]
            .into(),
            layers: PxFilterLayers::single_over(Layer::Transition),
            filter,
            ..default()
        },
        TransitionVenetian { row },
        Name::new("TransitionSpiralLine"),
    )
}

pub fn update_transition(
    mut commands: Commands,
    timer: Res<TransitionUpdateTimer>,
    mut counter: ResMut<TransitionCounter>,
    mut filters: PxAssets<PxFilter>,
    transition_line_query: Query<(Entity, &TransitionVenetian)>,
) {
    let n = 6;

    if timer.timer.finished() {
        let filter = filters.load("filter/color2.png");
        let step_closing = SCREEN_RESOLUTION.y;
        let step_buffer = step_closing + (0.35 * SCREEN_RESOLUTION.y as f32) as u32;
        let step_opening = step_buffer + (SCREEN_RESOLUTION.y as f32) as u32;

        if counter.value < step_closing {
            let remaining_rows = step_closing - counter.value;
            let rows_to_generate = remaining_rows.min(n);
            for i in 0..rows_to_generate {
                let row = (step_closing - counter.value - i);
                commands.spawn(make_transition_venetian_row(filter.clone(), row));
            }
        } else if counter.value < step_buffer {
        } else if counter.value < step_opening {
            let remaining_rows = step_opening - counter.value;
            let rows_to_despawn = remaining_rows.min(n);
            for i in 0..rows_to_despawn {
                let row = (step_opening - counter.value - i) as i32;
                if let Some((entity, _)) = transition_line_query
                    .iter()
                    .find(|(_, line)| line.row as i32 == row)
                {
                    commands.entity(entity).despawn();
                }
            }
        } else if counter.value == 2 * SCREEN_RESOLUTION.y {
            // send event to finish transition
        }

        counter.value += n;
    }
}
