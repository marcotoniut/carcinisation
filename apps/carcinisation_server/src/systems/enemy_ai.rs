//! Server-authoritative enemy AI adapters.
//!
//! The actual headless rules live in `carcinisation_fps_core`; this module only
//! translates replicated network components to and from that portable model.

use crate::ServerMap;
use bevy::prelude::*;
use carcinisation_fps_core::{
    EnemyPlayerTarget, EnemySim, FpsEnemyAiState, FpsEnemyKind, MosquitonAiConfig, tick_enemy_ai,
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
    let target_sims: Vec<EnemyPlayerTarget> = players
        .iter()
        .map(|player| EnemyPlayerTarget {
            position: player.position,
            alive: matches!(player.state, PlayerNetState::Alive),
            id: player.player_id.0,
        })
        .collect();

    for (mut net_enemy, net_health, ai_config) in &mut enemies {
        if net_enemy.enemy_type != NetEnemyType::Mosquiton {
            continue;
        }

        let Some(ai_config) = ai_config else {
            warn!(
                "Mosquiton NetEnemy {:?} has no ServerEnemyAiConfig; skipping AI",
                net_enemy.object_id
            );
            continue;
        };

        let mut sim = net_enemy_to_sim(&net_enemy, net_health);
        let _output = tick_enemy_ai(
            &mut sim,
            &target_sims,
            &server_map.0,
            fixed_time.delta_secs(),
            ai_config.0,
        );
        apply_sim_to_net_enemy(&sim, &mut net_enemy);
    }
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
