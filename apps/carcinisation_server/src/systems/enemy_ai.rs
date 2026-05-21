//! Server-authoritative enemy AI adapters.
//!
//! The actual headless rules live in `carcinisation_fps_core`; this module only
//! translates replicated network components to and from that portable model.

use crate::ServerMap;
use bevy::prelude::*;
use carcinisation_fps_core::{
    EnemyAiDisposition, EnemyPlayerTarget, EnemySim, FpsEnemyAiState, FpsEnemyKind,
    MosquitonAiConfig, tick_enemy_ai,
};
use carcinisation_net::{
    NetEnemy, NetEnemyState, NetEnemyType, NetHealth, NetPlayer, PlayerNetState,
};

use super::enemy_attack::{ServerMosquitonSim, ServerSpideySim};

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct EnemyAiSet;

/// Server-only AI tuning copied from map-authored enemy data at spawn time.
#[derive(Component, Clone, Copy, Debug, PartialEq, Reflect)]
#[reflect(Component)]
pub struct ServerEnemyAiConfig(pub MosquitonAiConfig);

impl ServerEnemyAiConfig {
    #[must_use]
    pub fn mosquiton(move_speed: f32) -> Self {
        Self(MosquitonAiConfig {
            move_speed,
            ..Default::default()
        })
    }
}

#[allow(clippy::type_complexity)]
pub fn tick_net_enemy_ai(
    mut enemies: Query<
        (&mut NetEnemy, &NetHealth, Option<&ServerEnemyAiConfig>),
        (Without<ServerMosquitonSim>, Without<ServerSpideySim>),
    >,
    players: Query<&NetPlayer>,
    server_map: Res<ServerMap>,
    fixed_time: Res<Time<Fixed>>,
) {
    let targets: Vec<(carcinisation_net::PlayerId, EnemyPlayerTarget)> = players
        .iter()
        .map(|player| {
            (
                player.player_id,
                EnemyPlayerTarget {
                    position: player.position,
                    alive: matches!(player.state, PlayerNetState::Alive),
                    id: player.player_id.0,
                },
            )
        })
        .collect();
    let target_sims: Vec<EnemyPlayerTarget> = targets.iter().map(|(_, target)| *target).collect();
    let alive_players = target_sims.iter().filter(|target| target.alive).count();

    let mut stats = EnemyAiTickStats::default();
    for (mut net_enemy, net_health, ai_config) in &mut enemies {
        if net_enemy.enemy_type != NetEnemyType::Mosquiton {
            stats.unsupported_type += 1;
            continue;
        }
        stats.processed += 1;

        let Some(ai_config) = ai_config else {
            warn!(
                "Mosquiton NetEnemy {:?} has no ServerEnemyAiConfig; skipping AI",
                net_enemy.object_id
            );
            continue;
        };

        let mut sim = net_enemy_to_sim(&net_enemy, net_health);
        let before_position = sim.position;
        let before_state = sim.state;
        let output = tick_enemy_ai(
            &mut sim,
            &target_sims,
            &server_map.0,
            fixed_time.delta_secs(),
            ai_config.0,
        );
        apply_sim_to_net_enemy(&sim, &mut net_enemy);
        stats.record(output.disposition);
        let target_player_id = output
            .target_position
            .and_then(|position| target_player_id_for_position(position, &targets));
        trace!(
            target: "carcinisation_server::enemy_ai",
            object_id = ?net_enemy.object_id,
            alive_players,
            target_player_id = ?target_player_id,
            target_position = ?output.target_position,
            distance = ?output.distance_to_target,
            desired_direction = ?output.desired_direction,
            attempted_step = ?output.attempted_step,
            old_position = ?before_position,
            new_position = ?sim.position,
            old_state = ?before_state,
            new_state = ?sim.state,
            speed = ai_config.0.move_speed,
            preferred_range = ai_config.0.preferred_range,
            hysteresis = ai_config.0.preferred_range_hysteresis,
            aggro_range = ai_config.0.aggro_range,
            collision_radius = ai_config.0.collision_radius,
            disposition = ?output.disposition,
            moved = output.moved,
            blocked = output.blocked_by_collision,
            "server enemy AI tick"
        );
    }
    trace!(
        target: "carcinisation_server::enemy_ai",
        processed = stats.processed,
        unsupported_type = stats.unsupported_type,
        dead = stats.dead,
        no_alive_players = stats.no_alive_players,
        outside_aggro_range = stats.outside_aggro_range,
        holding_preferred_range = stats.holding_preferred_range,
        chasing = stats.chasing,
        stalled_at_preferred_range = stats.stalled_at_preferred_range,
        blocked_by_collision = stats.blocked_by_collision,
        alive_players,
        "server enemy AI summary"
    );
}

#[derive(Default)]
struct EnemyAiTickStats {
    processed: usize,
    unsupported_type: usize,
    dead: usize,
    no_alive_players: usize,
    outside_aggro_range: usize,
    holding_preferred_range: usize,
    chasing: usize,
    stalled_at_preferred_range: usize,
    blocked_by_collision: usize,
}

impl EnemyAiTickStats {
    fn record(&mut self, disposition: EnemyAiDisposition) {
        match disposition {
            EnemyAiDisposition::None | EnemyAiDisposition::UnsupportedKind => {}
            EnemyAiDisposition::Dead => self.dead += 1,
            EnemyAiDisposition::NoAlivePlayers => self.no_alive_players += 1,
            EnemyAiDisposition::OutsideAggroRange => self.outside_aggro_range += 1,
            EnemyAiDisposition::HoldingPreferredRange => self.holding_preferred_range += 1,
            EnemyAiDisposition::Chasing => self.chasing += 1,
            EnemyAiDisposition::StalledAtPreferredRange => self.stalled_at_preferred_range += 1,
            EnemyAiDisposition::BlockedByCollision => self.blocked_by_collision += 1,
        }
    }
}

fn target_player_id_for_position(
    position: Vec2,
    targets: &[(carcinisation_net::PlayerId, EnemyPlayerTarget)],
) -> Option<carcinisation_net::PlayerId> {
    targets
        .iter()
        .find(|(_, target)| target.alive && target.position.distance_squared(position) < 0.000_001)
        .map(|(player_id, _)| *player_id)
}

fn net_enemy_to_sim(enemy: &NetEnemy, health: &NetHealth) -> EnemySim {
    EnemySim {
        kind: match enemy.enemy_type {
            NetEnemyType::Basic => FpsEnemyKind::Basic,
            NetEnemyType::Mosquiton => FpsEnemyKind::Mosquiton,
            NetEnemyType::Spidey => FpsEnemyKind::Spidey,
        },
        position: enemy.position,
        angle: enemy.angle,
        health: health.current,
        state: match enemy.state {
            NetEnemyState::Idle => FpsEnemyAiState::Idle,
            NetEnemyState::Chase => FpsEnemyAiState::Chasing,
            NetEnemyState::HoldingRange => FpsEnemyAiState::Attacking,
            // Dying/Dead enemies should not be ticked by AI; map to Dead.
            NetEnemyState::Dying { .. } | NetEnemyState::Dead { .. } => FpsEnemyAiState::Dead,
        },
    }
}

fn apply_sim_to_net_enemy(sim: &EnemySim, enemy: &mut NetEnemy) {
    enemy.position = sim.position;
    enemy.angle = sim.angle;
    // Only update state from AI if the enemy is not dying/dead (those are
    // managed by the death timer, not the AI).
    if !matches!(
        enemy.state,
        NetEnemyState::Dying { .. } | NetEnemyState::Dead { .. }
    ) {
        enemy.state = match sim.state {
            FpsEnemyAiState::Idle => NetEnemyState::Idle,
            FpsEnemyAiState::Chasing => NetEnemyState::Chase,
            FpsEnemyAiState::Attacking => NetEnemyState::HoldingRange,
            FpsEnemyAiState::Dead => NetEnemyState::Dead { burn: false },
        };
    }
}
