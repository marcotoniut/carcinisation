//! Lightweight server diagnostics — tick budget, player/enemy counts.
//!
//! `tick_diagnostics_start` runs first in `MovementSet` and
//! `tick_diagnostics_end` runs last in `TickSet`, so the wall-clock
//! measurement spans the full `FixedUpdate` workload.

use bevy::prelude::*;
use carcinisation_net::{NetPlayer, TickCounter};

use super::NetEnemy;

/// Interval between periodic summary logs, in ticks (30 Hz x 10 s = 300).
const SUMMARY_INTERVAL_TICKS: u32 = 300;

/// Tick budget threshold. Warn if a single `FixedUpdate` exceeds this.
const TICK_BUDGET_WARN_MS: f32 = 30.0;

/// Rolling tick duration statistics over the last summary window.
#[derive(Resource)]
pub struct DiagnosticsState {
    last_summary_tick: u32,
    /// Tick durations (ms) accumulated since last summary.
    samples: Vec<f32>,
}

impl Default for DiagnosticsState {
    fn default() -> Self {
        Self {
            last_summary_tick: 0,
            samples: Vec::with_capacity(SUMMARY_INTERVAL_TICKS as usize),
        }
    }
}

/// Record wall-clock time at the start of the `FixedUpdate` schedule.
/// Must run first in `TickSet` (before `tick_diagnostics_end`).
pub fn tick_diagnostics_start(mut start: Local<Option<std::time::Instant>>) {
    *start = Some(std::time::Instant::now());
}

/// Measure tick budget and emit periodic diagnostics.
#[allow(clippy::cast_precision_loss)]
pub fn tick_diagnostics_end(
    start: Local<Option<std::time::Instant>>,
    players: Query<&NetPlayer>,
    enemies: Query<&NetEnemy>,
    tick_counter: Res<TickCounter>,
    mut state: ResMut<DiagnosticsState>,
) {
    // Tick budget warning + sample collection.
    if let Some(t0) = *start {
        let elapsed_ms = t0.elapsed().as_secs_f32() * 1000.0;
        state.samples.push(elapsed_ms);
        if elapsed_ms > TICK_BUDGET_WARN_MS {
            warn!(
                "Tick budget exceeded: {elapsed_ms:.1}ms (limit {TICK_BUDGET_WARN_MS}ms) at tick {}",
                tick_counter.0
            );
        }
    }

    // Periodic summary with rolling stats.
    let current_tick = tick_counter.0.0;
    if current_tick.wrapping_sub(state.last_summary_tick) >= SUMMARY_INTERVAL_TICKS {
        state.last_summary_tick = current_tick;
        let player_count = players.iter().count();
        let enemy_count = enemies.iter().count();

        if state.samples.is_empty() {
            info!("[server] tick={current_tick} players={player_count} enemies={enemy_count}");
        } else {
            let avg = state.samples.iter().sum::<f32>() / state.samples.len() as f32;
            let max = state
                .samples
                .iter()
                .copied()
                .reduce(f32::max)
                .unwrap_or(0.0);
            info!(
                "[server] tick={current_tick} players={player_count} enemies={enemy_count} tick_ms={avg:.2}/{max:.2} (avg/max)"
            );
            state.samples.clear();
        }
    }
}
