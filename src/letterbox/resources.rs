use crate::core::time::*;
use bevy::prelude::*;
use std::time::Duration;

#[derive(Resource, Default, Debug, Clone, Copy)]
pub struct LetterboxTime {
    pub delta: Duration,
    pub elapsed: Duration,
}

impl DeltaTime for LetterboxTime {
    fn delta_seconds(&self) -> f32 {
        self.delta.as_secs_f32()
    }

    fn delta(&self) -> Duration {
        self.delta
    }
}

impl ElapsedTime for LetterboxTime {
    fn elapsed(&self) -> Duration {
        self.elapsed
    }
}

impl Ticker for LetterboxTime {
    fn tick(&mut self, delta: Duration) {
        self.delta = delta;
        self.elapsed += delta;
    }
}
