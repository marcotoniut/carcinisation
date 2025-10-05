//! Letterbox-specific time resource used for movement animations.

use crate::core::time::*;
use bevy::prelude::*;
use std::time::Duration;

#[derive(Resource, Default, Debug, Clone, Copy)]
/// Tracks elapsed time for letterbox movement easing.
pub struct LetterboxTime {
    pub delta: Duration,
    pub elapsed: Duration,
}

impl DeltaTime for LetterboxTime {
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
