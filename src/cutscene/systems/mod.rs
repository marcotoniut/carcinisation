pub mod interactions;
pub mod layout;
pub mod progress;
pub mod setup;

use super::cinemachine::{CinemachineModule, ClipBundle, CurrentClipInfo, UIBackground};
use crate::{
    game::GameProgressState, globals::mark_for_despawn_by_component_query,
    stage::resources::StageActionTimer, Layer,
};
use bevy::prelude::*;
use seldom_pixel::{
    prelude::{
        PxAnchor, PxAnimationBundle, PxAnimationDuration, PxAnimationFinishBehavior, PxAssets,
        PxCanvas, PxFilter, PxFilterLayers, PxLineBundle, PxSubPosition, PxTypeface,
    },
    sprite::{PxSprite, PxSpriteBundle},
};
use std::{env, time::Duration};

// pub fn make_clip_bundle(
//     assets_sprite: &mut PxAssets<PxSprite>,
//     cinemachine_data: CutsceneData,
// ) -> (
//     PxSpriteBundle<Layer>,
//     PxAnimationBundle,
//     PxSubPosition,
//     Name,
// ) {
//     //info!("skybox: {}", skybox_data.path);

//     let sprite = assets_sprite.load_animated(
//         cinemachine_data.clip.image_path,
//         cinemachine_data.clip.frame_count,
//     );
//     (
//         PxSpriteBundle::<Layer> {
//             sprite,
//             anchor: PxAnchor::BottomLeft,
//             canvas: PxCanvas::Camera,
//             layer: Layer::Skybox,
//             ..Default::default()
//         },
//         PxAnimationBundle {
//             // TODO variable time
//             duration: PxAnimationDuration::millis_per_animation(2000),
//             on_finish: PxAnimationFinishBehavior::Loop,
//             ..Default::default()
//         },
//         PxSubPosition::from(Vec2::new(0.0, 0.0)),
//         Name::new(format!("CINEMA_{}_clip", cinemachine_data.name)),
//     )
// }

// pub fn spawn_cinemachine_module(
//     commands: &mut Commands,
//     typefaces: &mut PxAssets<PxTypeface>,
//     assets_sprite: &mut PxAssets<PxSprite>,
//     filters: &mut PxAssets<PxFilter>,
//     cinemachine_data: CutsceneData,
// ) -> Entity {
//     let default_path = cinemachine_data.clip.image_path;
//     let mut fix_path = "".to_string();
//     if cfg!(windows) {
//         fix_path = format!(
//             "{}/assets{}",
//             env::current_dir().unwrap().to_str().unwrap().to_string(),
//             default_path
//         );
//     } else {
//         fix_path = format!("./assets{}", default_path);
//     }

//     let texture = assets_sprite.load(fix_path);

//     let entity = commands
//         .spawn((CinemachineModule {}, Name::new("CINEMACHINE_MODULE")))
//         .with_children(|parent| {
//             for i in 0..(160 as i32) {
//                 parent.spawn((
//                     PxLineBundle::<Layer> {
//                         canvas: PxCanvas::Camera,
//                         line: [(0, i).into(), (160, i).into()].into(),
//                         layers: PxFilterLayers::single_over(Layer::CutsceneBackground),
//                         filter: filters.load("filter/color3.png"),
//                         ..Default::default()
//                     },
//                     UIBackground {},
//                     Name::new(format!(
//                         "CINEMA_{}_UIBackground_LN_{}",
//                         cinemachine_data.name,
//                         i.to_string()
//                     )),
//                 ));
//             }
//             parent.spawn((
//                 PxSpriteBundle::<Layer> {
//                     sprite: texture,
//                     layer: Layer::CutsceneText,
//                     anchor: PxAnchor::BottomLeft,
//                     position: IVec2::new(
//                         cinemachine_data.clip.start_coordinates.x as i32,
//                         cinemachine_data.clip.start_coordinates.y as i32,
//                     )
//                     .into(),
//                     ..Default::default()
//                 },
//                 ClipBundle {},
//                 Name::new(format!("CINEMA_{}_clip", cinemachine_data.name)),
//             ));
//         })
//         .id();

//     return entity;
// }

// pub fn render_cutscene(
//     mut commands: Commands,
//     mut typefaces: PxAssets<PxTypeface>,
//     mut assets_sprite: PxAssets<PxSprite>,
//     mut filters: PxAssets<PxFilter>,
//     cinemachine_query: Query<Entity, With<CinemachineModule>>,
//     state: Res<State<GameProgressState>>,
//     mut game_state_next_state: ResMut<NextState<GameProgressState>>,
//     mut current_scene: ResMut<CinemachineScene>,
//     mut current_clip_info: ResMut<CurrentClipInfo>,
//     mut timer: ResMut<StageActionTimer>,
//     //curr_timer: Res<CinemachineTimer>,
//     time: Res<Time>,
// ) {
//     if state.get().to_owned() == GameProgressState::Cutscene {
//         let current_scene_option = current_scene.0.to_owned();

//         if let Ok(entity) = cinemachine_query.get_single() {
//             if let Some(mut scene) = current_scene_option {
//                 timer
//                     .timer
//                     .tick(Duration::from_secs_f32(time.delta_seconds()));

//                 if timer.timer.finished() {
//                     game_state_next_state.set(GameProgressState::Running)
//                 } else {
//                 }
//             }
//         } else {
//             if let Some(scene) = current_scene_option {
//                 spawn_cinemachine_module(
//                     &mut commands,
//                     &mut typefaces,
//                     &mut assets_sprite,
//                     &mut filters,
//                     scene,
//                 );
//             }
//         }
//     } else {
//         mark_for_despawn_by_component_query(&mut commands, &cinemachine_query);
//     }
// }
