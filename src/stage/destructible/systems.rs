use super::{
    components::{make_animation_bundle, Destructible, DestructibleState, DestructibleType},
    data::destructibles::DESTRUCTIBLE_ANIMATIONS,
};
use crate::stage::components::{
    interactive::{Dead, Flickerer, Hittable},
    placement::Depth,
};
use bevy::prelude::*;
use seldom_pixel::{prelude::*, sprite::PxSprite};

pub fn check_dead_destructible(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    query: Query<
        (Entity, &DestructibleType, &PxSubPosition, &Depth),
        (With<Destructible>, Added<Dead>),
    >,
) {
    for (entity, destructible_type, position, depth) in query.iter() {
        // TODO Should I do a bundle?
        let mut entity_commands = commands.entity(entity);
        entity_commands.remove::<(Hittable, Flickerer, Destructible)>();

        let animations_map = &DESTRUCTIBLE_ANIMATIONS.get_animation_data(destructible_type);
        let animation_bundle_o = make_animation_bundle(
            &asset_server,
            animations_map,
            &DestructibleState::Broken,
            depth,
        );
        if let Some(animation_bundle) = animation_bundle_o {
            entity_commands
                .insert(animation_bundle)
                .insert(PxSubPosition::from(position.0));
        }
    }
}
