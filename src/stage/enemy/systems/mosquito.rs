use bevy::prelude::*;
use seldom_pixel::{
    prelude::{PxAnchor, PxAnimationBundle, PxAnimationDuration, PxAssets, PxSubPosition},
    sprite::{PxSprite, PxSpriteBundle},
};

use crate::{
    stage::{
        components::Dead,
        enemy::{components::EnemyMosquito, data::mosquito::MOSQUITO_ANIMATIONS},
        score::components::Score,
    },
    Layer,
};

pub fn despawn_dead_mosquitoes(
    mut commands: Commands,
    mut assets_sprite: PxAssets<PxSprite>,
    mut score: ResMut<Score>,
    query: Query<(Entity, &EnemyMosquito, &PxSubPosition), With<Dead>>,
) {
    for (entity, mosquito, position) in query.iter() {
        // TODO Can I split this?
        commands.entity(entity).despawn();

        // HARDCODED depth, should be a component
        let depth = 1;
        let animation_o = MOSQUITO_ANIMATIONS.death.get(depth);

        if let Some(animation) = animation_o {
            let texture =
                assets_sprite.load_animated(animation.sprite_path.as_str(), animation.frames);

            commands.spawn((
                Name::new("EnemyMosquito - Death"),
                PxSubPosition::from(position.0),
                PxSpriteBundle::<Layer> {
                    sprite: texture,
                    layer: Layer::Middle(depth),
                    anchor: PxAnchor::Center,
                    ..default()
                },
                PxAnimationBundle {
                    duration: PxAnimationDuration::millis_per_animation(animation.speed),
                    on_finish: animation.finish_behavior,
                    ..default()
                },
            ));
        }

        score.value += mosquito.kill_score();
    }
}
