pub mod data;
pub mod cinemachine;

pub mod scene_intro;
pub mod scene_park;

use std::env;

use bevy::prelude::*;

use seldom_pixel::{
    prelude::{
        IRect, PxAnchor, PxAssets, PxCanvas, PxFilter, PxFilterLayers, PxLineBundle, PxSubPosition,
        PxTextBundle, PxTypeface,
    },
    sprite::{PxSprite, PxSpriteBundle},
};
use crate::{stage::{GameState, score::components::Score}, globals::{TYPEFACE_INVERTED_PATH, SCREEN_RESOLUTION, TYPEFACE_CHARACTERS, FONT_SIZE}, Layer, AppState};

use self::{data::{CinemachineData, Clip}, cinemachine::{UIBackground, CinemachineModule, CinemachineScene, CurrentClipInfo, ClipBundle}};


pub fn spawn_cinemachine_module(
    commands: &mut Commands,
    typefaces: &mut PxAssets<PxTypeface>,
    assets_sprite: &mut PxAssets<PxSprite>,
    filters: &mut PxAssets<PxFilter>,
    cinemachine_data: CinemachineData
) -> Entity {
    
    let entity = 
    commands.spawn((CinemachineModule {}, Name::new("CINEMACHINE_MODULE")))
        .with_children(|parent| {
            for i in 0..(160 as i32) {
                parent.spawn((
                    PxLineBundle::<Layer> {   
                        canvas: PxCanvas::Camera,
                        line: [(0, i).into(), (160, i).into()].into(),
                        layers: PxFilterLayers::single_over(Layer::CutsceneBackground),
                        filter: filters.load("filter/color3.png"),
                        ..default()
                    },
                    UIBackground {},
                    Name::new(format!("CINEMA_{}_UIBackground", cinemachine_data.name)),
                ));
            }
    })
    .id();

    return entity;
}

fn render_clip(
    mut commands: Commands,
    mut typefaces: PxAssets<PxTypeface>,
    mut assets_sprite: PxAssets<PxSprite>,
    mut filters: PxAssets<PxFilter>,
    query: Query<Entity, With<CinemachineModule>>,
    clip: &Clip,
    cinemachine_data: CinemachineData
) {
    if let Some(path) = clip.image_path.clone() {
        let mut fix_path = format!("{}/assets{}", env::current_dir().unwrap().to_str().unwrap().to_string(), path);
        //fix_path = format!("{}{}", ".", path.as_str());
        info!("path: {}", fix_path);
        let texture = assets_sprite.load(fix_path);

        if let Ok(entity) = query.get_single() {

            let clipBundle = commands.spawn((
                    PxSpriteBundle::<Layer> {
                    sprite: texture,
                    layer: Layer::CutsceneText,
                    anchor: PxAnchor::TopLeft,
                    position: IVec2::new(clip.start_coordinates.x as i32, clip.start_coordinates.y as i32).into(),
                    ..Default::default()
                },
                ClipBundle{},
                Name::new(format!("CINEMA_{}_clip", cinemachine_data.name))
            )).id();

            commands.entity(entity)
            .add_child(clipBundle);
        }
    }
}

fn despawn_clip(
) {

}

pub fn render_cutscene(
    mut commands: Commands,
    mut typefaces: PxAssets<PxTypeface>,
    mut assets_sprite: PxAssets<PxSprite>,
    mut filters: PxAssets<PxFilter>,
    query: Query<Entity, With<CinemachineModule>>,
    state: Res<State<GameState>>,
    mut game_state_next_state: ResMut<NextState<GameState>>,
    mut current_scene: ResMut<CinemachineScene>,
    mut current_clip_info: ResMut<CurrentClipInfo>,
    time: Res<Time>
) {
    if state.get().to_owned() == GameState::Cutscene {
        let current_scene_option = current_scene.0.to_owned();

        if let Ok(entity) = query.get_single() {
            if let Some(mut scene)= current_scene_option {
                let current_clip = &scene.clips[current_clip_info.index.to_owned()];

                if !current_clip_info.isRendered{
                    render_clip(
                        commands,
                        typefaces,
                        assets_sprite,
                        filters,
                        query,
                        &current_clip,
                        scene.clone()
                    );
                    match current_clip.goal {
                        data::CutsceneGoal::MOVEMENT { .. } => {
                            
                        },
                        data::CutsceneGoal::TIMED { mut waitInSeconds  } => {

                        },
                    }

                    current_clip_info.startedRender();
                } else if !current_clip_info.hasFinished {
                    match current_clip.goal {
                        data::CutsceneGoal::MOVEMENT { .. } => {
                            
                        },
                        data::CutsceneGoal::TIMED { mut waitInSeconds  } => {
                            
                            current_clip.goal.subtract_time(time.delta_seconds());
                            //waitInSeconds -= time.delta_seconds();
                            warn!("{}", waitInSeconds.to_string());
                        },
                    }
                }
                
                current_clip_info.inc();
                //scene.clips.iter()
                if current_clip_info.index.to_owned() >= scene.clips.len()
                {
                    current_clip_info.reset();
                    game_state_next_state.set(GameState::Running);
                }
            }
        } else {
            if let Some(scene) = current_scene_option {
                spawn_cinemachine_module(
                    &mut commands,
                    &mut typefaces,
                    &mut assets_sprite,
                    &mut filters,
                    scene
                );
            }
        }
    } else {
        despawn_cutscene(&mut commands, query);
    }
}

pub fn despawn_cutscene(
    mut commands: &mut Commands,
    query: Query<Entity, With<CinemachineModule>>,
) {
    if let Ok(entity) = query.get_single() {
        commands.entity(entity).despawn_recursive();
    }
}