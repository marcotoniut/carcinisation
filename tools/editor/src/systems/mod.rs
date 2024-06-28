pub mod cutscene;
pub mod input;

use bevy::{
    asset::Assets,
    prelude::{Camera2dBundle, Commands, Res, ResMut},
};
use carcinisation::CutsceneData;

use crate::{components::LoadedScene, resources::CutsceneAssetHandle};

pub fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

pub fn check_cutscene_data_loaded(
    cutscene_asset_handle: Res<CutsceneAssetHandle>,
    cutscene_data_assets: Res<Assets<CutsceneData>>,
    mut commands: Commands,
) {
    if let Some(cutscene_data) = cutscene_data_assets.get(&cutscene_asset_handle.handle) {
        println!("Cutscene data loaded: {:?}", cutscene_data);
        commands.remove_resource::<CutsceneAssetHandle>();
        commands.insert_resource(LoadedScene::Cutscene(cutscene_data.clone()));
    } else {
        // Asset is not yet loaded
        println!("Cutscene data is still loading...");
    }
}
