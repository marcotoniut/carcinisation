use bevy::{prelude::*, time::*};
use serde::Deserialize;

use super::data::StageData;

#[derive(Debug, Deserialize, Resource, Clone, Default)]
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

// TODO
// impl StageDataHandle {
//     pub fn get_action_by_index<'a>(
//         &self,
//         assets_stage_data: &Res<'a, Assets<StageData>>,
//         step: usize,
//     ) -> Option<&'a StageAction> {
//         if let Some(stage) = assets_stage_data.get(&self.0) {
//             let x = stage.actions.get(step);
//             x
//         } else {
//             None
//         }
//     }
// }
