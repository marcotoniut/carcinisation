use crate::{
    cutscene::{
        components::{build_letterbox_bottom, build_letterbox_top, Cinematic, CutsceneEntity},
        data::CutsceneData,
        events::{CutsceneShutdownEvent, CutsceneStartupEvent},
        resources::CutsceneProgress,
        CutscenePluginUpdateState,
    },
    globals::{mark_for_despawn_by_component_query, GBColor},
};
use bevy::prelude::*;
use seldom_pixel::prelude::{PxAssets, PxFilter, PxSubPosition};

pub fn on_startup(
    mut commands: Commands,
    mut event_reader: EventReader<CutsceneStartupEvent>,
    mut cutscene_state_next_state: ResMut<NextState<CutscenePluginUpdateState>>,
    mut filters: PxAssets<PxFilter>,
) {
    for e in event_reader.iter() {
        cutscene_state_next_state.set(CutscenePluginUpdateState::Active);

        let data = e.data.as_ref();

        commands.insert_resource::<CutsceneData>(data.clone());
        commands.insert_resource::<CutsceneProgress>(CutsceneProgress { index: 0 });

        commands.spawn((
            Cinematic,
            Name::new("Cutscene"),
            PxSubPosition(Vec2::new(50., 30.)),
        ));

        let filter = filters.load(GBColor::Black.get_filter_path());

        build_letterbox_top(&mut commands, &filter);
        build_letterbox_bottom(&mut commands, &filter);
    }
}

pub fn on_shutdown(
    mut commands: Commands,
    mut event_reader: EventReader<CutsceneShutdownEvent>,
    cinematic_query: Query<Entity, With<Cinematic>>,
    cutscene_entity_query: Query<Entity, With<CutsceneEntity>>,
) {
    for _ in event_reader.iter() {
        mark_for_despawn_by_component_query(&mut commands, &cutscene_entity_query);
        mark_for_despawn_by_component_query(&mut commands, &cinematic_query);
    }
}
