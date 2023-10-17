use std::collections::VecDeque;
use std::time::Duration;

use bevy::prelude::*;
use seldom_pixel::prelude::PxSubPosition;

use crate::stage::data::{EnemyStep, MovementDirection};

#[derive(Component)]
pub enum GameStep {
    Credits,
    Cutscene,
    // GameOver,
    // Continue?
    Stage,
    StageCleared,
    Transition,
}

#[derive(Component)]
pub struct GameSteps(pub Vec<GameStep>);

#[derive(Component)]
pub struct GameCurrentStep {
    pub started: Duration,
    pub step: GameStep,
}

// #[derive(Component)]
// pub struct GameOver {}

impl GameSteps {
    pub fn new(steps: Vec<GameStep>) -> Self {
        GameSteps(steps.into())
    }

    pub fn next(&mut self) -> Option<GameStep> {
        self.0.pop_front()
    }
}
