use super::EnemyHoveringAttackType;
use crate::{
    layer::Layer,
    stage::components::{interactive::ColliderData, placement::Depth},
};
use bevy::prelude::*;
use seldom_pixel::prelude::{PxAnchor, PxAnimation, PxAnimationDuration, PxSprite};

#[derive(Bundle)]
pub struct HoveringAttackAnimationBundle {
    pub sprite: PxSprite,
    pub animation: PxAnimation,
    pub collider_data: ColliderData,
    pub depth: Depth,
    pub anchor: PxAnchor,
}

impl HoveringAttackAnimationBundle {
    pub fn new(
        asset_server: &Res<AssetServer>,
        attack_type: &EnemyHoveringAttackType,
        depth: Depth,
    ) -> Self {
        let animation_o = attack_type.get_animations().hovering.get(&depth);

        let animation = animation_o.unwrap();
        let texture = PxSprite(asset_server.load(animation.sprite_path.as_str()));
        // TODO animate animation.frames

        Self {
            sprite: texture,
            animation: animation.make_animation_bundle(),
            collider_data: ColliderData::new(),
            depth,
            anchor: PxAnchor::Center,
        }
    }
}
