#[cfg(debug_assertions)]
use crate::debug::DebugGodMode;
use crate::pixel::PxAssets;
use crate::stage::{
    components::{
        damage::{DamageFlicker, InvertFilter},
        interactive::{Dead, Flickerer, Health},
    },
    enemy::composed::ComposedHealthPools,
    messages::DamageMessage,
    player::components::Player,
    resources::StageTimeDomain,
};
use assert_assets_path::assert_assets_path;
use bevy::prelude::*;
use carapace::prelude::PxFilter;
use std::time::Duration;

pub const DAMAGE_FLICKER_COUNT: u8 = 4;

pub static DAMAGE_REGULAR_DURATION: std::sync::LazyLock<Duration> =
    std::sync::LazyLock::new(|| Duration::from_secs_f32(0.2));
pub static DAMAGE_INVERT_DURATION: std::sync::LazyLock<Duration> =
    std::sync::LazyLock::new(|| Duration::from_secs_f32(0.15));

/// @system Applies incoming entity-level damage and marks entities as `Dead` when health reaches zero.
///
/// Entities with `ComposedHealthPools` are excluded because their damage is routed through
/// part-specific health pools via `PartDamageMessage`. Entity-level damage (fall damage, etc.)
/// should still go through this system, so composed enemies should use a simple `Health` component.
pub fn on_damage(
    mut commands: Commands,
    mut event_reader: MessageReader<DamageMessage>,
    mut query: Query<&mut Health, (Without<Dead>, Without<ComposedHealthPools>)>,
    players: Query<(), With<Player>>,
    #[cfg(debug_assertions)] god_mode: Option<Res<DebugGodMode>>,
) {
    for e in event_reader.read() {
        #[cfg(debug_assertions)]
        if god_mode.as_ref().is_some_and(|god_mode| god_mode.enabled) && players.contains(e.entity)
        {
            continue;
        }

        if let Ok(mut health) = query.get_mut(e.entity) {
            health.0 = health.0.saturating_sub(e.value);
            if health.0 == 0 {
                commands.entity(e.entity).insert(Dead);
            }
        }
    }
}

/// @system Starts a damage flicker cycle on flickerable entities that were just hit.
// NOTE: should be checked after damage taken
pub fn check_damage_flicker_taken(
    mut commands: Commands,
    stage_time: Res<Time<StageTimeDomain>>,
    mut reader: MessageReader<DamageMessage>,
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

/// @system Applies the invert filter during the active phase of a damage flicker.
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

/// @system Removes the invert filter and advances/ends the flicker cycle.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn player_damage_is_ignored_while_debug_god_mode_is_enabled() {
        let mut app = App::new();
        app.add_message::<DamageMessage>();
        #[cfg(debug_assertions)]
        app.insert_resource(DebugGodMode::new(true));
        app.add_systems(Update, on_damage);

        let entity = app.world_mut().spawn((Player, Health(10))).id();
        app.world_mut().write_message(DamageMessage::new(entity, 4));
        app.update();

        assert_eq!(app.world().entity(entity).get::<Health>().unwrap().0, 10);
        assert!(app.world().entity(entity).get::<Dead>().is_none());
    }
}
