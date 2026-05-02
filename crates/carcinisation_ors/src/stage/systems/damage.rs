use crate::assets::CxAssets;
use crate::stage::{
    components::{
        damage::{DamageFlicker, InvertFilter},
        interactive::{BurningCorpse, Dead, Flickerer, Health},
    },
    enemy::components::Enemy,
    messages::{DamageMessage, DamageSource},
    player::components::Player,
    player::flamethrower::FlamethrowerConfig,
    resources::StageTimeDomain,
};
#[cfg(debug_assertions)]
use crate::stubs::DebugGodMode;
use assert_assets_path::assert_assets_path;
use bevy::prelude::*;
use carapace::prelude::CxFilter;
use carapace::prelude::WorldPos;
use carcinisation_base::fire_death::corpse_seed;
use std::time::Duration;

pub const DAMAGE_FLICKER_COUNT: u8 = 4;

pub static DAMAGE_REGULAR_DURATION: std::sync::LazyLock<Duration> =
    std::sync::LazyLock::new(|| Duration::from_secs_f32(0.2));
pub static DAMAGE_INVERT_DURATION: std::sync::LazyLock<Duration> =
    std::sync::LazyLock::new(|| Duration::from_secs_f32(0.15));

/// @system Applies incoming entity-level damage and marks entities as `Dead` when health reaches zero.
///
/// Part-specific damage on composed enemies is routed through `PartDamageMessage` →
/// `apply_composed_part_damage`. Entity-level `DamageMessage` (flamethrower, environmental)
/// bypasses part durability and hits the entity's `Health` directly, regardless of whether
/// it also has `ComposedHealthPools`.
pub fn on_damage(
    mut commands: Commands,
    mut event_reader: MessageReader<DamageMessage>,
    mut query: Query<(&mut Health, Has<Enemy>, Option<&WorldPos>), Without<Dead>>,
    players: Query<(), With<Player>>,
    config: Res<FlamethrowerConfig>,
    stage_time: Res<Time<StageTimeDomain>>,
    #[cfg(debug_assertions)] god_mode: Option<Res<DebugGodMode>>,
) {
    for e in event_reader.read() {
        #[cfg(debug_assertions)]
        if god_mode.as_ref().is_some_and(|god_mode| god_mode.enabled) && players.contains(e.entity)
        {
            continue;
        }

        if let Ok((mut health, is_enemy, position)) = query.get_mut(e.entity) {
            health.0 = health.0.saturating_sub(e.value);
            if health.0 == 0 {
                let mut entity_commands = commands.entity(e.entity);
                if is_enemy && e.source == DamageSource::Fire {
                    let position = position.map_or(Vec2::ZERO, |position| position.0);
                    entity_commands.insert(BurningCorpse {
                        started: stage_time.elapsed(),
                        duration: Duration::from_secs_f32(config.burning_corpse_duration_secs),
                        seed: corpse_seed(position),
                    });
                }
                entity_commands.insert(Dead);
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
    query: Query<
        (Entity, Option<&DamageFlicker>),
        (With<Flickerer>, Without<Dead>, Without<BurningCorpse>),
    >,
) {
    for e in reader.read() {
        if let Ok((entity, active_flicker)) = query.get(e.entity)
            && active_flicker.is_none()
        {
            commands.entity(entity).insert(DamageFlicker {
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
    mut query: Query<(Entity, &mut DamageFlicker), (Without<InvertFilter>, Without<BurningCorpse>)>,
    filters: CxAssets<CxFilter>,
) {
    for (entity, mut damage_flicker) in &mut query.iter_mut() {
        if stage_time.elapsed() >= damage_flicker.phase_start {
            damage_flicker.phase_start = stage_time.elapsed();
            commands.entity(entity).insert((
                InvertFilter,
                CxFilter(filters.load(assert_assets_path!("filter/invert.px_filter.png"))),
            ));
        }
    }
}

/// @system Removes the invert filter and advances/ends the flicker cycle.
pub fn remove_invert_filter(
    mut commands: Commands,
    stage_time: Res<Time<StageTimeDomain>>,
    mut query: Query<(Entity, &mut DamageFlicker), (With<InvertFilter>, Without<BurningCorpse>)>,
) {
    let invert_duration = *DAMAGE_INVERT_DURATION;
    for (entity, mut damage_flicker) in &mut query.iter_mut() {
        if stage_time.elapsed() >= damage_flicker.phase_start + invert_duration {
            let mut entity_commands = commands.entity(entity);
            entity_commands
                .remove::<InvertFilter>()
                .remove::<CxFilter>();
            if damage_flicker.count > 0 {
                damage_flicker.count -= 1;
                damage_flicker.phase_start = stage_time.elapsed() + *DAMAGE_REGULAR_DURATION;
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
        app.init_resource::<Time<StageTimeDomain>>();
        app.insert_resource(FlamethrowerConfig::load());
        #[cfg(debug_assertions)]
        app.insert_resource(DebugGodMode::new(true));
        app.add_systems(Update, on_damage);

        let entity = app.world_mut().spawn((Player, Health(10))).id();
        app.world_mut().write_message(DamageMessage::new(entity, 4));
        app.update();

        assert_eq!(app.world().entity(entity).get::<Health>().unwrap().0, 10);
        assert!(app.world().entity(entity).get::<Dead>().is_none());
    }

    #[test]
    fn lethal_fire_damage_marks_enemy_as_burning_corpse() {
        let mut app = App::new();
        app.add_message::<DamageMessage>();
        app.init_resource::<Time<StageTimeDomain>>();
        app.insert_resource(FlamethrowerConfig::load());
        app.add_systems(Update, on_damage);

        let entity = app
            .world_mut()
            .spawn((
                crate::stage::enemy::components::Enemy,
                Health(10),
                WorldPos(Vec2::new(4.0, 5.0)),
            ))
            .id();
        app.world_mut()
            .write_message(DamageMessage::fire(entity, 10));
        app.update();

        let entity_ref = app.world().entity(entity);
        assert!(entity_ref.get::<Dead>().is_some());
        assert!(entity_ref.get::<BurningCorpse>().is_some());
    }

    #[test]
    fn lethal_fire_damage_does_not_start_normal_flicker_in_stage_order() {
        let mut app = App::new();
        app.add_message::<DamageMessage>();
        app.init_resource::<Time<StageTimeDomain>>();
        app.insert_resource(FlamethrowerConfig::load());
        app.add_systems(Update, (on_damage, check_damage_flicker_taken).chain());

        let entity = app
            .world_mut()
            .spawn((Enemy, Flickerer, Health(10), WorldPos(Vec2::new(4.0, 5.0))))
            .id();
        app.world_mut()
            .write_message(DamageMessage::fire(entity, 10));
        app.update();

        let entity_ref = app.world().entity(entity);
        assert!(entity_ref.get::<Dead>().is_some());
        assert!(entity_ref.get::<BurningCorpse>().is_some());
        assert!(entity_ref.get::<DamageFlicker>().is_none());
    }
}
