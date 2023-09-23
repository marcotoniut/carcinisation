pub mod data;
pub mod scene_park;

use bevy::prelude::*;

pub fn default_background(
    commands: &mut Commands,
    cinemachine_data: CinemachineData
) -> Entity {
    let entity = commands.spawn((
        PxLineBundle::<Layer> {
            canvas: PxCanvas::Camera,
            line: [((SCREEN_RESOLUTION.x / 2) as i32 - 40, i).into(), ((SCREEN_RESOLUTION.x / 2) as i32 + 40 as i32, i).into()].into(),
            layers: PxFilterLayers::single_over(Layer::UIBackground),
            filter: filters.load("filter/color3.png"),
            ..default()
        },
        UIBackground {},
        Name::new(format("CINEMA_{}_UIBackground", cinemachine_data.name)),
    ))
    .id();

    return entity;
}

pub fn render(

) {
    
}