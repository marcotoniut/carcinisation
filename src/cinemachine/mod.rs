pub mod data;
pub mod scene_park;
pub mod cinemachine;

use bevy::prelude::*;

use seldom_pixel::{
    prelude::{
        IRect, PxAnchor, PxAssets, PxCanvas, PxFilter, PxFilterLayers, PxLineBundle, PxSubPosition,
        PxTextBundle, PxTypeface,
    },
    sprite::{PxSprite, PxSpriteBundle},
};
use crate::{stage::{GameState, score::components::Score}, globals::{TYPEFACE_INVERTED_PATH, SCREEN_RESOLUTION, TYPEFACE_CHARACTERS, FONT_SIZE}, Layer, AppState};

use self::{data::CinemachineData, cinemachine::{UIBackground, CinemachineModule}};


pub fn default_background(
    commands: &mut Commands,
    typefaces: &mut PxAssets<PxTypeface>,
    assets_sprite: &mut PxAssets<PxSprite>,
    filters: &mut PxAssets<PxFilter>,
    cinemachine_data: CinemachineData
) -> Entity {
    let entity = 
    commands.spawn((CinemachineModule {}, Name::new("CINEMACHINE_MODULE")))
        .with_children(|parent| {
            for i in 40..(100 as i32) {
                parent.spawn((
                    PxLineBundle::<Layer> {   
                        canvas: PxCanvas::Camera,
                        line: [((SCREEN_RESOLUTION.x / 2) as i32 - 40, i).into(), ((SCREEN_RESOLUTION.x / 2) as i32 + 40 as i32, i).into()].into(),
                        layers: PxFilterLayers::single_over(Layer::UIBackground),
                        filter: filters.load(cinemachine_data.default_background_filter_color),
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

pub fn render(

) {
    
}