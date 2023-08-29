use bevy::prelude::*;
use seldom_pixel::{
    prelude::{
        IRect, PxAnchor, PxAssets, PxFilter, PxFilterLayers, PxLineBundle, PxRect, PxSubPosition,
        PxTextBundle, PxTypeface,
    },
    sprite::{PxSprite, PxSpriteBundle},
};

use crate::{globals::*, Layer};

use super::super::{components::*, styles::*};

const LAYOUT_Y: f32 = 2.0;

pub fn spawn_hud(
    mut commands: Commands,
    mut typefaces: PxAssets<PxTypeface>,
    mut assets_sprite: PxAssets<PxSprite>,
    mut filters: PxAssets<PxFilter>,
) {
    let main_menu_entity = spawn_hud_bundle(
        &mut commands,
        &mut typefaces,
        &mut assets_sprite,
        &mut filters,
    );
}

pub fn despawn_hud(mut commands: Commands, query: Query<Entity, With<Hud>>) {
    if let Ok(main_menu_entity) = query.get_single() {
        commands.entity(main_menu_entity).despawn_recursive();
    }
}

pub fn spawn_hud_bundle(
    commands: &mut Commands,
    typefaces: &mut PxAssets<PxTypeface>,
    assets_sprite: &mut PxAssets<PxSprite>,
    filters: &mut PxAssets<PxFilter>,
) -> Entity {
    let typeface = typefaces.load(
        TYPEFACE_INVERTED_PATH,
        TYPEFACE_CHARACTERS,
        // Equivalent to, for example, `vec![PxSeparatorConfig { character: ' ', width: 4 }]`
        [(' ', 4)],
    );

    let main_menu_entity = commands
        .spawn((Hud {}, Name::new("Hud")))
        .with_children(|parent| {
            for i in 0..(HUD_HEIGHT as i32) {
                parent.spawn(PxLineBundle::<Layer> {
                    line: [(0, i).into(), (SCREEN_RESOLUTION.x as i32, i).into()].into(),
                    layers: PxFilterLayers::single_over(Layer::UIBackground),
                    filter: filters.load("filter/color3.png"),
                    ..default()
                });
            }

            parent.spawn((Name::new("Score"),)).with_children(|parent| {
                parent.spawn((
                    PxSpriteBundle::<Layer> {
                        sprite: assets_sprite.load("sprites/score-icon.png"),
                        anchor: PxAnchor::BottomLeft,
                        layer: Layer::UI,
                        ..default()
                    },
                    PxSubPosition::from(Vec2::new(6.0, LAYOUT_Y)),
                    Name::new("ScoreIcon"),
                ));
                parent.spawn((
                    PxTextBundle::<Layer> {
                        text: "0".into(),
                        typeface: typeface.clone(),
                        alignment: PxAnchor::BottomRight,
                        layer: Layer::UI,
                        // rect: IRect::new(IVec2::new(14, LAYOUT_Y as i32), IVec2::new(24, 12))
                        //     .into(),
                        ..default()
                    },
                    PxSubPosition::from(Vec2::new(14.0, LAYOUT_Y)),
                    ScoreText,
                    Name::new("ScoreText"),
                ));
            });

            parent
                .spawn((Name::new("Enemies"),))
                .with_children(|parent| {
                    parent.spawn((
                        PxSpriteBundle::<Layer> {
                            sprite: assets_sprite.load("sprites/enemy-count-icon.png"),
                            anchor: PxAnchor::BottomRight,
                            layer: Layer::UI,
                            ..default()
                        },
                        PxSubPosition::from(Vec2::new(SCREEN_RESOLUTION.x as f32 - 6.0, LAYOUT_Y)),
                        Name::new("EnemyCountIcon"),
                    ));

                    parent.spawn((
                        PxTextBundle::<Layer> {
                            text: "0".into(),
                            typeface: typeface.clone(),
                            alignment: PxAnchor::BottomRight,
                            layer: Layer::UI,
                            // rect: IRect::new(
                            //     IVec2::new(SCREEN_RESOLUTION.x as i32 - 30, LAYOUT_Y as i32),
                            //     IVec2::new(20, 10),
                            // )
                            // .into(),
                            ..default()
                        },
                        PxSubPosition::from(Vec2::new(SCREEN_RESOLUTION.x as f32 - 30.0, LAYOUT_Y)),
                        EnemyCountText,
                        Name::new("EnemyCountText"),
                    ));
                });
        })
        .id();

    return main_menu_entity;
}
