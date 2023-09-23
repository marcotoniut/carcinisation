use bevy::prelude::*;

use super::data::StageData;

#[derive(Clone, Debug, Default, Resource)]
pub struct StageProgress {
    pub elapsed: f32,
    pub step: usize,
    pub step_elapsed: f32,
    pub spawn_step: usize,
    pub spawn_step_elapsed: f32,
}

#[derive(Resource)]
pub struct StageActionTimer {
    pub timer: Timer,
}

impl Default for StageActionTimer {
    fn default() -> Self {
        let mut timer = Timer::from_seconds(0., TimerMode::Once);
        timer.pause();
        StageActionTimer { timer }
    }
}

#[derive(Resource)]
pub struct StageDataHandle(pub Handle<StageData>);
