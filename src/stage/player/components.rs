use std::time::Duration;

use bevy::prelude::*;
use seldom_pixel::{
    asset::PxAsset,
    prelude::{
        PxAnchor, PxAnimationBundle, PxAnimationDuration, PxAnimationFinishBehavior, PxAssets,
        PxCanvas,
    },
    sprite::{PxSprite, PxSpriteBundle, PxSpriteData},
};

use crate::Layer;

#[derive(Component)]
pub struct Player {}

pub const PLAYER_SPEED: f32 = 125.;
pub const PLAYER_SIZE: f32 = 9.;

pub enum Weapon {
    Pincer,
    Gun,
}

#[derive(Component)]
pub struct PlayerAttack {
    pub weapon: Weapon,
    pub position: Vec2,
}

impl PlayerAttack {
    pub fn get_sprite_bundle(
        &self,
        assets_sprite: &mut PxAssets<PxSprite>,
    ) -> (PxSpriteBundle<Layer>, PxAnimationBundle) {
        // TODO sprite
        match self.weapon {
            Weapon::Pincer => {
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
                )
            }
            Weapon::Gun => {
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
                )
            }
        }
    }
}
