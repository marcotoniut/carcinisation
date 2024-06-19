use crate::{
    layer::{Layer, MidDepth, PreBackgroundDepth},
    plugins::movement::structs::MovementVec2Position,
};
use bevy::{prelude::*, utils::HashMap};
use num_enum::TryFromPrimitive;
use std::{
    iter::Step,
    ops::{Add, Sub},
};
use strum_macros::EnumIter;

#[derive(
    Component,
    Debug,
    Clone,
    Copy,
    PartialEq,
    EnumIter,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Reflect,
    TryFromPrimitive,
)]
#[repr(i8)]
pub enum Depth {
    Nine = 9,
    Eight = 8,
    Seven = 7,
    Six = 6,
    Five = 5,
    Four = 4,
    Three = 3,
    Two = 2,
    One = 1,
    Zero = 0,
}

impl Default for Depth {
    fn default() -> Self {
        Self::MAX
    }
}

impl Add<i8> for Depth {
    type Output = Depth;

    fn add(self, other: i8) -> Depth {
        let value = (self as i8 + other)
            .min(Depth::MAX.to_i8())
            .max(Depth::MIN.to_i8());
        Depth::try_from(value).unwrap_or_else(|_| Depth::MAX)
    }
}

impl Sub<i8> for Depth {
    type Output = Depth;

    fn sub(self, other: i8) -> Depth {
        let value = (self as i8 - other)
            .min(Depth::MAX.to_i8())
            .max(Depth::MIN.to_i8());
        Depth::try_from(value).unwrap_or_else(|_| Depth::MIN)
    }
}

impl Step for Depth {
    fn steps_between(start: &Self, end: &Self) -> Option<usize> {
        Some(((*end as i8 - *start as i8).abs()) as usize)
    }

    fn forward_checked(start: Self, count: usize) -> Option<Self> {
        let end = start as i8 + count as i8;
        Depth::try_from(end).ok()
    }

    fn backward_checked(start: Self, count: usize) -> Option<Self> {
        let end = start as i8 - count as i8;
        Depth::try_from(end).ok()
    }
}

impl Depth {
    pub const MAX: Self = Self::Nine;
    pub const MIN: Self = Self::Zero;

    pub fn to_f32(&self) -> f32 {
        self.to_i8() as f32
    }

    pub fn to_i8(&self) -> i8 {
        *self as i8
    }

    pub fn to_layer(&self) -> Layer {
        match self {
            Self::Nine => Layer::PreBackgroundDepth(PreBackgroundDepth::Nine),
            Self::Eight => Layer::PreBackgroundDepth(PreBackgroundDepth::Eight),
            Self::Seven => Layer::PreBackgroundDepth(PreBackgroundDepth::Seven),
            Self::Six => Layer::MidDepth(MidDepth::Six),
            Self::Five => Layer::MidDepth(MidDepth::Five),
            Self::Four => Layer::MidDepth(MidDepth::Four),
            Self::Three => Layer::MidDepth(MidDepth::Three),
            Self::Two => Layer::MidDepth(MidDepth::Two),
            Self::One => Layer::MidDepth(MidDepth::One),
            Self::Zero => Layer::MidDepth(MidDepth::Zero),
        }
    }
}

#[derive(Component, Debug, Clone, Copy, Reflect)]
pub struct Floor(pub f32);

#[derive(Component, Debug, Clone, Copy, Reflect)]
pub struct Speed(pub f32);

#[derive(Component, Debug)]
pub struct InView;

#[derive(Component, Debug)]
pub struct LinearUpdateDisabled;

// DEPRECATED
#[derive(Clone, Component, Debug, Reflect)]
pub struct RailPosition(pub Vec2);

// DEPRECATED
impl MovementVec2Position for RailPosition {
    fn get(&self) -> Vec2 {
        self.0
    }
    fn set(&mut self, position: Vec2) {
        self.0 = position;
    }
    fn add(&mut self, position: Vec2) {
        self.0 += position;
    }
}

pub fn spawn_floor_depths(commands: &mut Commands, floor_depths: &HashMap<Depth, f32>) {
    for (depth, y) in floor_depths.iter() {
        commands.spawn((Floor(*y), depth.clone()));
    }
}
