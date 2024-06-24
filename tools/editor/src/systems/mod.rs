pub mod cutscene;
pub mod input;

use std::sync::Arc;

use bevy::{
    asset::Assets,
    prelude::{Camera2dBundle, Commands, EventWriter, Res},
};
use carcinisation::CutsceneData;

use crate::{events::CutsceneLoadedEvent, resources::CutsceneAssetHandle};

pub fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

pub fn check_cutscene_data_loaded(
    cutscene_asset_handle: Res<CutsceneAssetHandle>,
    cutscene_data_assets: Res<Assets<CutsceneData>>,
    mut cinematic_startup_event_writer: EventWriter<CutsceneLoadedEvent>,
    mut commands: Commands,
) {
    if let Some(cutscene_data) = cutscene_data_assets.get(&cutscene_asset_handle.handle) {
        println!("Cutscene data loaded: {:?}", cutscene_data);
        cinematic_startup_event_writer.send(CutsceneLoadedEvent {
            // TODO do I need Arc for this? Can it not be handled by a simple pointer reference?
            data: Arc::new(cutscene_data.clone()),
        });
        commands.remove_resource::<CutsceneAssetHandle>();
    } else {
        // Asset is not yet loaded
        println!("Cutscene data is still loading...");
    }
}
