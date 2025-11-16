use crate::pixel::PxAssets;
use crate::stage::{
    components::{
        damage::{DamageFlicker, InvertFilter},
        interactive::{Dead, Flickerer, Health},
    },
    events::DamageEvent,
    resources::StageTimeDomain,
};
use assert_assets_path::assert_assets_path;
use bevy::prelude::*;
use seldom_pixel::prelude::PxFilter;
use std::time::Duration;

pub const DAMAGE_FLICKER_COUNT: u8 = 4;

lazy_static! {
    pub static ref DAMAGE_REGULAR_DURATION: Duration = Duration::from_secs_f32(0.2);
    pub static ref DAMAGE_INVERT_DURATION: Duration = Duration::from_secs_f32(0.15);
}

pub fn on_damage(
    mut commands: Commands,
    mut event_reader: MessageReader<DamageEvent>,
    mut query: Query<&mut Health, Without<Dead>>,
) {
    for e in event_reader.read() {
        if let Ok(mut health) = query.get_mut(e.entity) {
            health.0 = health.0.saturating_sub(e.value);
            if health.0 == 0 {
                commands.entity(e.entity).insert(Dead);
            }
        }
    }
}

/**
 * Should be checked after damage taken
 */
pub fn check_damage_flicker_taken(
    mut commands: Commands,
    stage_time: Res<Time<StageTimeDomain>>,
    mut reader: MessageReader<DamageEvent>,
    // TODO Destructibles and Attacks
    query: Query<Entity, (With<Flickerer>, Without<Dead>)>,
) {
    for e in reader.read() {
        if query.get(e.entity).is_ok() {
            commands.entity(e.entity).insert(DamageFlicker {
                phase_start: stage_time.elapsed() + *DAMAGE_REGULAR_DURATION,
                count: DAMAGE_FLICKER_COUNT,
            });
        }
    }
}

pub fn add_invert_filter(
    mut commands: Commands,
    stage_time: Res<Time<StageTimeDomain>>,
    mut query: Query<(Entity, &mut DamageFlicker), Without<InvertFilter>>,
    filters: PxAssets<PxFilter>,
) {
    let regular_duration = *DAMAGE_REGULAR_DURATION;
    for (entity, mut damage_flicker) in &mut query.iter_mut() {
        if stage_time.elapsed() < damage_flicker.phase_start + regular_duration {
            damage_flicker.phase_start = stage_time.elapsed();
            commands.entity(entity).insert((
                InvertFilter,
                PxFilter(filters.load(assert_assets_path!("filter/invert.px_filter.png"))),
            ));
        }
    }
}

pub fn remove_invert_filter(
    mut commands: Commands,
    stage_time: Res<Time<StageTimeDomain>>,
    mut query: Query<(Entity, &mut DamageFlicker), With<InvertFilter>>,
) {
    let invert_duration = *DAMAGE_INVERT_DURATION;
    for (entity, mut damage_flicker) in &mut query.iter_mut() {
        if stage_time.elapsed() < damage_flicker.phase_start + invert_duration {
            let mut entity_commands = commands.entity(entity);
            entity_commands
                .remove::<InvertFilter>()
                .remove::<PxFilter>();
            if damage_flicker.count > 0 {
                damage_flicker.count -= 1;
                damage_flicker.phase_start = stage_time.elapsed();
            } else {
                entity_commands.remove::<DamageFlicker>();
            }
        }
    }
}
