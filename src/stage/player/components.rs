use std::time::Duration;

use bevy::{
    audio::{PlaybackMode, Volume},
    prelude::*,
};
use seldom_pixel::{
    asset::PxAsset,
    position,
    prelude::{
        PxAnchor, PxAnimationBundle, PxAnimationDuration, PxAnimationFinishBehavior, PxAssets,
        PxCanvas, PxSubPosition,
    },
    sprite::{PxSprite, PxSpriteBundle, PxSpriteData},
};

use crate::Layer;

#[derive(Component)]
pub struct Player {}

pub const PLAYER_SPEED: f32 = 125.;
pub const PLAYER_SIZE: f32 = 9.;

#[derive(Clone, Copy, Debug)]
pub enum Weapon {
    Pincer,
    Gun,
}

#[derive(Component, Clone, Copy, Debug)]
pub struct PlayerAttack {
    pub weapon: Weapon,
    pub position: Vec2,
}

impl PlayerAttack {
    pub fn make_bundle(
        &self,
        assets_sprite: &mut PxAssets<PxSprite>,
        asset_server: Res<AssetServer>,
    ) -> (
        Self,
        PxSpriteBundle<Layer>,
        PxAnimationBundle,
        AudioSourceBundle,
        PxSubPosition,
        Name,
    ) {
        let position = PxSubPosition::from(self.position);
        let name = Name::new("PlayerAttack");

        // TODO sprite
        let (sprite_bundle, animation_bundle, audio_source_bundle) = match self.weapon {
            Weapon::Pincer => {
                let melee_slash_sound = asset_server.load("audio/melee_attack_01.ogg");
                let sprite = assets_sprite.load_animated("sprites/melee_slash.png", 9);
                (
                    PxSpriteBundle::<Layer> {
                        sprite,
                        anchor: PxAnchor::Center,
                        canvas: PxCanvas::Camera,
                        layer: Layer::Attack,
                        ..default()
                    },
                    PxAnimationBundle {
                        duration: PxAnimationDuration::millis_per_animation(700),
                        on_finish: PxAnimationFinishBehavior::Despawn,
                        ..default()
                    },
                    AudioBundle {
                        source: melee_slash_sound,
                        settings: PlaybackSettings {
                            mode: PlaybackMode::Despawn,
                            ..default()
                        },
                        ..default()
                    },
                )
            }
            Weapon::Gun => {
                let shoot_sound = asset_server.load("audio/melee_attack_01.ogg");
                let sprite = assets_sprite.load("sprites/star.png");
                (
                    PxSpriteBundle::<Layer> {
                        sprite,
                        anchor: PxAnchor::Center,
                        canvas: PxCanvas::Camera,
                        layer: Layer::Attack,
                        ..default()
                    },
                    PxAnimationBundle {
                        duration: PxAnimationDuration::millis_per_animation(80),
                        on_finish: PxAnimationFinishBehavior::Despawn,
                        ..default()
                    },
                    AudioBundle {
                        source: shoot_sound,
                        settings: PlaybackSettings {
                            mode: PlaybackMode::Despawn,
                            ..default()
                        },
                        ..default()
                    },
                )
            }
        };

        (
            *self,
            sprite_bundle,
            animation_bundle,
            audio_source_bundle,
            position,
            name,
        )
    }
}
