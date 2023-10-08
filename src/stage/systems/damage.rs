use std::time::Duration;

use bevy::prelude::*;
use seldom_pixel::prelude::{PxAssets, PxFilter};

use crate::stage::{
    components::{
        damage::{DamageFlicker, InvertFilter},
        interactive::{Dead, Flickerer, Health},
    },
    events::DamageEvent,
    resources::StageTime,
};

pub const DAMAGE_FLICKER_COUNT: u8 = 4;

lazy_static! {
    pub static ref DAMAGE_REGULAR_DURATION: Duration = Duration::from_secs_f32(0.2);
    pub static ref DAMAGE_INVERT_DURATION: Duration = Duration::from_secs_f32(0.15);
}

pub fn check_damage_taken(
    mut commands: Commands,
    mut event_reader: EventReader<DamageEvent>,
    mut query: Query<(Entity, &mut Health), Without<Dead>>,
) {
    for event in event_reader.iter() {
        for (entity, mut health) in &mut query.iter_mut() {
            if entity == event.entity {
                health.0 = health.0.saturating_sub(event.value);
                if health.0 == 0 {
                    commands.entity(entity).insert(Dead);
                }
            }
        }
    }
}

/**
 * Should be checked after damage taken
 */
pub fn check_damage_flicker_taken(
    mut commands: Commands,
    stage_time: Res<StageTime>,
    mut event_reader: EventReader<DamageEvent>,
    // TODO Destructibles and Attacks
    query: Query<Entity, (With<Flickerer>, Without<Dead>)>,
) {
    for event in event_reader.iter() {
        for entity in query.iter() {
            if entity == event.entity {
                commands.entity(entity).insert(DamageFlicker {
                    phase_start: stage_time.elapsed + DAMAGE_REGULAR_DURATION.clone(),
                    count: DAMAGE_FLICKER_COUNT,
                });
            }
        }
    }
}

pub fn add_invert_filter(
    mut commands: Commands,
    stage_time: Res<StageTime>,
    mut query: Query<(Entity, &mut DamageFlicker), Without<InvertFilter>>,
    mut filters: PxAssets<PxFilter>,
) {
    let regular_duration = DAMAGE_REGULAR_DURATION.clone();
    for (entity, mut damage_flicker) in &mut query.iter_mut() {
        if stage_time.elapsed < damage_flicker.phase_start + regular_duration {
            damage_flicker.phase_start = stage_time.elapsed;
            commands
                .entity(entity)
                .insert((InvertFilter, filters.load("filter/invert.png")));
        }
    }
}

pub fn remove_invert_filter(
    mut commands: Commands,
    stage_time: Res<StageTime>,
    mut query: Query<(Entity, &mut DamageFlicker), With<InvertFilter>>,
) {
    let invert_duration = DAMAGE_INVERT_DURATION.clone();
    for (entity, mut damage_flicker) in &mut query.iter_mut() {
        if stage_time.elapsed < damage_flicker.phase_start + invert_duration {
            let mut entity_commands = commands.entity(entity);
            entity_commands
                .remove::<InvertFilter>()
                .remove::<Handle<PxFilter>>();
            if damage_flicker.count > 0 {
                damage_flicker.count -= 1;
                damage_flicker.phase_start = stage_time.elapsed;
            } else {
                entity_commands.remove::<DamageFlicker>();
            }
        }
    }
}
