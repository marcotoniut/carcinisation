use bevy::prelude::*;
use bevy_common_assets::ron::RonAssetPlugin;

use super::data::CutsceneData;

// fn load_cutscene_data(
//     asset_server: Res<AssetServer>,
//     mut cutscene_data_assets: ResMut<Assets<CutsceneData>>,
// ) {
//     let handle: Handle<CutsceneData> = asset_server.load("cinematic_intro.ron");
//     cutscene_data_assets.add(handle);
// }

// fn use_cutscene_data(
//     cutscene_data_assets: Res<Assets<CutsceneData>>,
//     asset_server: Res<AssetServer>,
//     query: Query<&Handle<CutsceneData>>,
// ) {
//     for handle in query.iter() {
//         if let Some(cutscene_data) = cutscene_data_assets.get(handle) {
//             println!("Loaded cutscene data: {:?}", cutscene_data);
//             // Use the cutscene data as needed
//         }
//     }
// }
