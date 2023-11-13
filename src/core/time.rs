use bevy::prelude::*;
use std::{marker::PhantomData, time::Duration};

pub trait DeltaTime: Send + Sync + 'static {
    fn delta_seconds(&self) -> f32;
    fn delta(&self) -> Duration;
}

pub trait ElapsedTime: Send + Sync + 'static {
    fn elapsed(&self) -> Duration;
}

pub trait Ticker: DeltaTime + ElapsedTime {
    fn tick(&mut self, delta: Duration);
}

#[derive(Resource)]
pub struct TimeMultiplier<T: Ticker> {
    _phantom: PhantomData<T>,
    pub value: f32,
}

impl<T: Ticker> TimeMultiplier<T> {
    pub fn new(value: f32) -> Self {
        Self {
            _phantom: PhantomData,
            value,
        }
    }
}

#[cfg(not(debug_assertions))]
pub fn tick_time<T: Ticker + Resource>(mut time: ResMut<T>, app_time: Res<Time>) {
    time.tick(app_time.delta());
}
#[cfg(debug_assertions)]
pub fn tick_time<T: Ticker + Resource>(
    mut time: ResMut<T>,
    app_time: Res<Time>,
    time_multiplier: Option<Res<TimeMultiplier<T>>>,
) {
    time.tick(
        app_time
            .delta()
            .mul_f32(time_multiplier.map(|x| x.value).unwrap_or(1.)),
    );
}
