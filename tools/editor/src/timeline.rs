use std::time::Duration;

use bevy::math::Vec2;
use carcinisation::cutscene::data::CutsceneAnimationsSpawn;
use carcinisation::stage::components::{CinematicStageStep, StopStageStep, TweenStageStep};
use carcinisation::stage::data::{StageData, StageStep};

#[derive(Clone, Copy, Debug)]
pub struct StageTimelineConfig {
    pub include_spawn_delays: bool,
    pub include_stop_durations: bool,
    pub include_cinematic_durations: bool,
    pub collapse_infinite_stops: bool,
}

impl StageTimelineConfig {
    pub const SLIDER: Self = Self {
        include_spawn_delays: false,
        include_stop_durations: true,
        include_cinematic_durations: true,
        collapse_infinite_stops: true,
    };
}

#[derive(Clone, Debug)]
pub struct StageTimelineStep {
    pub index: usize,
    pub duration: Duration,
}

#[derive(Clone, Debug)]
pub struct StageTimeline {
    pub steps: Vec<StageTimelineStep>,
    pub total: Duration,
}

impl StageTimeline {
    pub fn from_stage(stage_data: &StageData, config: StageTimelineConfig) -> Self {
        let mut steps = Vec::with_capacity(stage_data.steps.len());
        let mut total = Duration::ZERO;
        let mut current_position = stage_data.start_coordinates;

        for (index, step) in stage_data.steps.iter().enumerate() {
            let duration = step_duration(step, &mut current_position, config);
            total += duration;
            steps.push(StageTimelineStep { index, duration });
        }

        Self { steps, total }
    }

    pub fn clamp_elapsed(&self, elapsed: Duration) -> Duration {
        if elapsed > self.total {
            self.total
        } else {
            elapsed
        }
    }

    pub fn step_index_at(&self, elapsed: Duration) -> usize {
        if self.steps.is_empty() {
            return 0;
        }
        if elapsed.is_zero() {
            return self.steps[0].index;
        }

        let mut remaining = elapsed;
        for step in &self.steps {
            if step.duration.is_zero() {
                continue;
            }
            if remaining <= step.duration {
                return step.index;
            }
            remaining -= step.duration;
        }

        self.steps.last().map(|step| step.index).unwrap_or(0)
    }
}

pub fn tween_travel_duration(current_position: Vec2, step: &TweenStageStep) -> Duration {
    let distance = step.coordinates.distance(current_position);
    let base_speed = step.base_speed.max(0.0001);
    Duration::from_secs_f32(distance / base_speed)
}

pub fn stop_duration(step: &StopStageStep, config: StageTimelineConfig) -> Duration {
    if !config.include_stop_durations {
        return Duration::ZERO;
    }
    match step.max_duration {
        Some(duration) => duration,
        None => {
            if config.collapse_infinite_stops {
                Duration::ZERO
            } else {
                Duration::ZERO
            }
        }
    }
}

pub fn cinematic_duration(step: &CinematicStageStep, config: StageTimelineConfig) -> Duration {
    if !config.include_cinematic_durations {
        return Duration::ZERO;
    }
    match step {
        CinematicStageStep::CutsceneAnimationSpawn(CutsceneAnimationsSpawn { spawns }) => spawns
            .iter()
            .fold(Duration::ZERO, |total, spawn| total + spawn.duration),
    }
}

fn step_duration(
    step: &StageStep,
    current_position: &mut Vec2,
    config: StageTimelineConfig,
) -> Duration {
    match step {
        StageStep::Tween(tween_step) => {
            let mut duration = tween_travel_duration(*current_position, tween_step);
            if config.include_spawn_delays {
                for spawn in &tween_step.spawns {
                    duration += spawn.get_elapsed();
                }
            }
            *current_position = tween_step.coordinates;
            duration
        }
        StageStep::Stop(stop_step) => {
            let mut duration = stop_duration(stop_step, config);
            if config.include_spawn_delays {
                for spawn in &stop_step.spawns {
                    duration += spawn.get_elapsed();
                }
            }
            duration
        }
        StageStep::Cinematic(cinematic_step) => cinematic_duration(cinematic_step, config),
    }
}
