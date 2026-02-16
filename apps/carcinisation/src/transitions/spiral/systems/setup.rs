use crate::{
    globals::{SCREEN_RESOLUTION, mark_for_despawn_by_query},
    pixel::PxAssets,
    transitions::{
        data::{TransitionVenetianData, TransitionVenetianDataState},
        spiral::{
            TransitionVenetianPlugin,
            bundles::spawn_transition_venetian_row,
            components::TransitionVenetian,
            messages::{TransitionVenetianShutdownEvent, TransitionVenetianStartupEvent},
            resources::{TransitionCounter, TransitionUpdateTimer},
        },
    },
};
use activable::{activate, deactivate};
use bevy::prelude::*;
use seldom_pixel::prelude::PxFilter;

/// @trigger Initialises a venetian-blind transition effect.
#[allow(dead_code)]
pub fn on_transition_startup(
    trigger: On<TransitionVenetianStartupEvent>,
    mut commands: Commands,
    mut timer: ResMut<TransitionUpdateTimer>,
    filters: PxAssets<PxFilter>,
    existing: Query<Entity, With<TransitionVenetian>>,
) {
    activate::<TransitionVenetianPlugin>(&mut commands);

    timer.timer.reset();

    mark_for_despawn_by_query(&mut commands, &existing);

    let data = trigger.event().data.as_ref().clone();
    let mut counter = TransitionCounter::default();

    if matches!(data.state, TransitionVenetianDataState::Opening) {
        counter.value = screen_height_with_buffer(&data);

        let filter = PxFilter(filters.load("filter/color2.px_filter.png"));
        for row in 0..=SCREEN_RESOLUTION.y {
            spawn_transition_venetian_row(&mut commands, filter.clone(), row);
        }
    }

    commands.insert_resource::<TransitionVenetianData>(data);
    commands.insert_resource(counter);

    commands.spawn((
        TransitionVenetian,
        Name::new("Transition - Venetian"),
        Visibility::Visible,
        InheritedVisibility::VISIBLE,
    ));
}

/// @trigger Tears down a venetian-blind transition and removes its resources.
#[allow(dead_code)]
pub fn on_transition_shutdown(
    _trigger: On<TransitionVenetianShutdownEvent>,
    mut commands: Commands,
    transition_query: Query<Entity, With<TransitionVenetian>>,
) {
    mark_for_despawn_by_query(&mut commands, &transition_query);
    deactivate::<TransitionVenetianPlugin>(&mut commands);
    commands.remove_resource::<TransitionVenetianData>();
    commands.remove_resource::<TransitionCounter>();
}

fn screen_height_with_buffer(data: &TransitionVenetianData) -> u32 {
    SCREEN_RESOLUTION.y + data.buffer_rows
}
