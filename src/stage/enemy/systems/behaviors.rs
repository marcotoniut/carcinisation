use bevy::prelude::*;

use crate::stage::{
    enemy::{
        components::{Enemy, EnemyBehaviors, EnemyCurrentBehavior},
        resources::EnemyBehaviorTimer,
    },
    resources::StageTime,
};

pub fn check_no_behavior(
    mut commands: Commands,
    mut query: Query<(Entity, &mut EnemyBehaviors), (With<Enemy>, Without<EnemyCurrentBehavior>)>,
    stage_time: Res<StageTime>,
) {
    for (entity, mut behaviors) in query.iter_mut() {
        let behavior = behaviors.next();

        let duration_o = behavior.get_duration();

        commands
            .entity(entity)
            .insert(EnemyCurrentBehavior {
                started: stage_time.elapsed,
                behavior,
            })
            .with_children(|parent| {
                if let Some(duration) = duration_o {
                    parent.spawn(EnemyBehaviorTimer::new(entity, duration));
                }
            });
    }
}

pub fn check_behavior_timer(
    mut commands: Commands,
    mut query: Query<&mut EnemyBehaviorTimer>,
    stage_time: Res<StageTime>,
) {
    for mut behavior in query.iter_mut() {
        behavior.timer.tick(stage_time.delta);

        if behavior.finished() {
            commands
                .entity(behavior.entity)
                .remove::<EnemyCurrentBehavior>();
        }
    }
}
