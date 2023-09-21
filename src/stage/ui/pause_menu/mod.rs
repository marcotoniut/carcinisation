pub mod pause_menu;

use bevy::prelude::*;

use seldom_pixel::{
    prelude::{
        IRect, PxAnchor, PxAssets, PxCanvas, PxFilter, PxFilterLayers, PxLineBundle, PxSubPosition,
        PxTextBundle, PxTypeface,
    },
    sprite::{PxSprite, PxSpriteBundle},
};
use crate::{stage::GameState, globals::{TYPEFACE_INVERTED_PATH, SCREEN_RESOLUTION, TYPEFACE_CHARACTERS, FONT_SIZE}, Layer, AppState};

use self::pause_menu::{PauseMenu, UIBackground, InfoText};

pub fn pause_menu_renderer(
    mut commands: Commands,
    mut typefaces: PxAssets<PxTypeface>,
    mut assets_sprite: PxAssets<PxSprite>,
    mut filters: PxAssets<PxFilter>,
    query: Query<Entity, With<PauseMenu>>,
    state: Res<State<GameState>>
) {
    if state.get().to_owned() == GameState::Paused {
        if let Ok(entity) = query.get_single()
        {
            //do nothing
        } else {
            spawn_pause_menu_bundle(
                &mut commands,
                &mut typefaces,
                &mut assets_sprite,
                &mut filters
            );
        }
    } else {
        despawn_pause_menu_bundle(
            &mut commands,
            query
        );
    }
}

pub fn despawn_pause_menu_bundle(
    mut commands: &mut Commands,
    query: Query<Entity, With<PauseMenu>>
) {
    if let Ok(entity) = query.get_single() {
        commands.entity(entity).despawn_recursive();
    }
}

pub fn spawn_pause_menu_bundle(
    commands: &mut Commands,
    typefaces: &mut PxAssets<PxTypeface>,
    assets_sprite: &mut PxAssets<PxSprite>,
    filters: &mut PxAssets<PxFilter>,
) -> Entity {
    let typeface = typefaces.load(TYPEFACE_INVERTED_PATH, TYPEFACE_CHARACTERS, [(' ', 4)]);

    let entity = commands
        .spawn((PauseMenu {}, Name::new("PauseMenu")))
        .with_children(|parent| {
            for i in 40..(100 as i32) {
                parent.spawn((
                    PxLineBundle::<Layer> {
                        canvas: PxCanvas::Camera,
                        line: [((SCREEN_RESOLUTION.x / 2) as i32 - 40, i).into(), ((SCREEN_RESOLUTION.x / 2) as i32 + 40 as i32, i).into()].into(),
                        layers: PxFilterLayers::single_over(Layer::UIBackground),
                        filter: filters.load("filter/color3.png"),
                        ..default()
                    },
                    UIBackground {},
                    Name::new("UIBackground"),
                ));

                parent.spawn((
                    PxTextBundle::<Layer> {
                        alignment: PxAnchor::BottomCenter,
                        canvas: PxCanvas::Camera,
                        layer: Layer::UI,
                        rect: IRect::new(
                            IVec2::new(
                                (SCREEN_RESOLUTION.x / 2) as i32 - 40,
                                90,
                            ),
                            IVec2::new(
                                (SCREEN_RESOLUTION.x / 2) as i32 + 40,
                                90 + (FONT_SIZE + 2) as i32,
                            ),
                        )
                        .into(),
                        text: "Paused".into(),
                        typeface: typeface.clone(),
                        ..default()
                    },
                    InfoText,
                    Name::new("InfoText"),
                ));
            }

        })
        .id();
    return entity;
}