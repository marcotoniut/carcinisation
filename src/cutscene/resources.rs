use crate::core::time::*;
use bevy::prelude::*;
use std::time::Duration;

#[derive(Resource, Default, Debug, Clone, Copy)]
pub struct CutsceneTime {
    pub delta: Duration,
    pub elapsed: Duration,
}

impl DeltaTime for CutsceneTime {
    fn delta_seconds(&self) -> f32 {
        self.delta.as_secs_f32()
    }

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
pub struct CutsceneProgress {
    pub index: usize,
}
