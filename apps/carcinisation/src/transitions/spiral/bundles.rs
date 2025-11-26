use crate::{
    globals::SCREEN_RESOLUTION,
    layer::Layer,
    pixel::{PxAssets, PxLineBundle},
    transitions::data::TransitionVenetianData,
};
use bevy::prelude::*;
use seldom_pixel::prelude::{PxCanvas, PxFilter, PxFilterLayers};

use super::{
    components::{TransitionVenetian, TransitionVenetianRow},
    messages::TransitionVenetianShutdownEvent,
    resources::{TransitionCounter, TransitionUpdateTimer},
};

#[allow(dead_code)]
pub fn spawn_transition_venetian_row(commands: &mut Commands, filter: PxFilter, row: u32) {
    commands.spawn((
        PxLineBundle::<Layer> {
            canvas: PxCanvas::Camera,
            line: [
                (0, row as i32).into(),
                (SCREEN_RESOLUTION.x as i32, row as i32).into(),
            ]
            .into(),
            layers: PxFilterLayers::single_over(Layer::Transition),
            filter,
            visibility: Visibility::Visible,
        },
        TransitionVenetianRow { row },
        TransitionVenetian,
        Name::new("TransitionSpiralLine"),
    ));
}

#[allow(dead_code)]
pub fn update_transition(
    mut commands: Commands,
    timer: Res<TransitionUpdateTimer>,
    counter: Option<ResMut<TransitionCounter>>,
    filters: PxAssets<PxFilter>,
    transition_line_query: Query<(Entity, &TransitionVenetianRow)>,
    data: Option<Res<TransitionVenetianData>>,
) {
    // Early return if resources aren't ready yet (commands not yet applied)
    let Some(mut counter) = counter else { return };
    let Some(data) = data else { return };

    let n = 6;

    if counter.finished || !timer.timer.is_finished() {
        return;
    }

    let filter = PxFilter(filters.load("filter/color2.px_filter.png"));
    let closing_limit = SCREEN_RESOLUTION.y;
    let buffer_limit = closing_limit + data.buffer_rows;
    let opening_limit = buffer_limit + SCREEN_RESOLUTION.y;

    let current = counter.value;

    if current < closing_limit {
        let remaining_rows = closing_limit.saturating_sub(current);
        let rows_to_generate = remaining_rows.min(n);
        for i in 0..rows_to_generate {
            let row = closing_limit.saturating_sub(current + i);
            spawn_transition_venetian_row(&mut commands, filter.clone(), row);
        }
    } else if current < buffer_limit {
        // hold the closed state briefly to hide underlying changes
    } else if current < opening_limit {
        let remaining_rows = opening_limit.saturating_sub(current);
        let rows_to_despawn = remaining_rows.min(n);
        for i in 0..rows_to_despawn {
            let row = (opening_limit.saturating_sub(current + i)) as i32;
            if let Some((entity, _)) = transition_line_query
                .iter()
                .find(|(_, line)| line.row as i32 == row)
            {
                commands.entity(entity).despawn();
            }
        }
    } else {
        counter.finished = true;
        counter.value = current.saturating_add(n);
        return;
    }

    counter.value = current.saturating_add(n);
}

/// Separate system to check if transition is finished and trigger shutdown.
/// This avoids resource borrow conflicts when triggering observers.
#[allow(dead_code)]
pub fn check_transition_finished(mut commands: Commands, counter: Option<Res<TransitionCounter>>) {
    if let Some(counter) = counter {
        if counter.finished {
            commands.trigger(TransitionVenetianShutdownEvent);
        }
    }
}
