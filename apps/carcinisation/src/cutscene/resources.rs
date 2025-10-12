//! Cutscene timing and progression resources shared by systems.

use crate::core::time::*;
use bevy::prelude::*;
use std::time::Duration;

#[derive(Resource, Default, Debug, Clone, Copy)]
/// Tracks delta/elapsed time for cutscene playback.
pub struct CutsceneTime {
    pub delta: Duration,
    pub elapsed: Duration,
}

impl DeltaTime for CutsceneTime {
    fn delta(&self) -> Duration {
        self.delta
    }
}

impl ElapsedTime for CutsceneTime {
    fn elapsed(&self) -> Duration {
        self.elapsed
    }
}

impl Ticker for CutsceneTime {
    fn tick(&mut self, delta: Duration) {
        self.delta = delta;
        self.elapsed += delta;
    }
}

#[derive(Resource, Default, Clone, Copy)]
/// Index into the current cutscene act list.
pub struct CutsceneProgress {
    pub index: usize,
}
