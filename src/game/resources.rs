use super::data::GameStep;
use bevy::prelude::*;
use num_enum::TryFromPrimitive;
use std::iter::Step;
use strum_macros::EnumIter;

// TODO should default be 3?
#[derive(Resource, Debug, Default, Clone, Copy)]
pub struct Lives(pub u8);

#[derive(Resource, Default, Clone, Copy)]
pub struct GameProgress {
    pub index: usize,
}

#[derive(Clone, Debug, Resource)]
pub struct GameData {
    pub name: String,
    pub steps: Vec<GameStep>,
}

#[derive(
    Resource,
    Debug,
    Clone,
    Copy,
    Eq,
    PartialEq,
    PartialOrd,
    Hash,
    Default,
    EnumIter,
    TryFromPrimitive,
)]
#[repr(i8)]
pub enum Difficulty {
    Easy,
    #[default]
    Normal,
    Hard,
}

impl Step for Difficulty {
    fn steps_between(start: &Self, end: &Self) -> Option<usize> {
        Some(((*end as i8 - *start as i8).abs()) as usize)
    }

    fn forward_checked(start: Self, count: usize) -> Option<Self> {
        let end = start as i8 + count as i8;
        Difficulty::try_from(end).ok()
    }

    fn backward_checked(start: Self, count: usize) -> Option<Self> {
        let end = start as i8 - count as i8;
        Difficulty::try_from(end).ok()
    }
}
