//! Game progression resources: lives, difficulty, queued data handles.

use crate::{cutscene::data::CutsceneData, stage::data::StageData};

use super::data::GameStep;
use bevy::prelude::*;
use num_enum::TryFromPrimitive;
use std::{convert::TryInto, iter::Step};
use strum_macros::EnumIter;

#[derive(Resource, Debug, Default, Clone, Copy)]
/// Number of lives remaining in the current run.
pub struct Lives(pub u8);

#[derive(Resource, Default, Clone, Copy)]
/// Tracks which game step is currently active.
pub struct GameProgress {
    pub index: usize,
}

#[derive(Clone, Debug, Resource)]
/// Describes the campaign being played.
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
    fn steps_between(start: &Self, end: &Self) -> (usize, Option<usize>) {
        if start <= end {
            let steps = (*end as i8 - *start as i8) as usize;
            (steps, Some(steps))
        } else {
            (0, None)
        }
    }

    fn forward_checked(start: Self, count: usize) -> Option<Self> {
        let offset: i16 = count.try_into().ok()?;
        let target = i16::from(start as i8) + offset;
        let target = i8::try_from(target).ok()?;
        Difficulty::try_from(target).ok()
    }

    fn backward_checked(start: Self, count: usize) -> Option<Self> {
        let offset: i16 = count.try_into().ok()?;
        let target = i16::from(start as i8) - offset;
        let target = i8::try_from(target).ok()?;
        Difficulty::try_from(target).ok()
    }
}

#[derive(Resource)]
/// Handle to a cutscene asset waiting to load.
pub struct CutsceneAssetHandle {
    pub handle: Handle<CutsceneData>,
}

#[derive(Resource)]
/// Handle to a stage asset waiting to load.
pub struct StageAssetHandle {
    pub handle: Handle<StageData>,
}
