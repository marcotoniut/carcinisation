use crate::{cutscene::data::CutsceneData, stage::data::StageData};

use super::data::GameStep;
use bevy::prelude::*;
use num_enum::TryFromPrimitive;
use std::iter::Step;
use strum_macros::EnumIter;

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

#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Hash, Default)]
#[repr(i8)]
pub enum Difficulty {
    Easy = 0,
    #[default]
    Normal = 1,
    Hard = 2,
}

// Nightly-only Step impl.
impl Step for Difficulty {
    // The new signature returns (min_steps, Some(max_steps)) unless a > b
    // or overflow might happen. If start > end, we return (0, None).
    fn steps_between(start: &Self, end: &Self) -> (usize, Option<usize>) {
        let s = *start as i8;
        let e = *end as i8;

        // If start > end, return (0, None) per the docs.
        if s > e {
            return (0, None);
        }

        // If they're equal, it’s (0, Some(0)).
        if s == e {
            return (0, Some(0));
        }

        // Otherwise s < e. Calculate how many forward steps are needed.
        let diff = e.wrapping_sub(s); // or just e - s since we’re dealing with small i8
                                      // In a real type you would check for overflow, but with 3 variants, we’re safe:
                                      // e - s can't overflow i8 in this small enum.

        let steps = diff as usize;
        // Return (steps, Some(steps)) as long as it fits in usize and doesn’t overflow.
        (steps, Some(steps))
    }

    fn forward_checked(start: Self, count: usize) -> Option<Self> {
        let s = start as i8;
        // Checking that adding `count` as i8 won't overflow i8 could be done,
        // but for three variants it’s not strictly necessary.
        let end = s.checked_add(count as i8)?;
        Self::try_from(end).ok()
    }

    fn backward_checked(start: Self, count: usize) -> Option<Self> {
        let s = start as i8;
        let end = s.checked_sub(count as i8)?;
        Self::try_from(end).ok()
    }
}

// Provide a TryFrom<i8> that covers your enum range.
impl std::convert::TryFrom<i8> for Difficulty {
    type Error = ();

    fn try_from(value: i8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Difficulty::Easy),
            1 => Ok(Difficulty::Normal),
            2 => Ok(Difficulty::Hard),
            _ => Err(()),
        }
    }
}

#[derive(Resource)]
pub struct CutsceneAssetHandle {
    pub handle: Handle<CutsceneData>,
}

#[derive(Resource)]
pub struct StageAssetHandle {
    pub handle: Handle<StageData>,
}
