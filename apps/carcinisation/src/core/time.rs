//! Time abstractions and helpers shared across plugins (stage, movement, etc.).

use bevy::prelude::*;
use derive_new::new;
use std::{marker::PhantomData, time::Duration};

pub trait DeltaTime: Send + Sync + 'static {
    fn delta(&self) -> Duration;
}

pub trait ElapsedTime: Send + Sync + 'static {
    fn elapsed(&self) -> Duration;
}

pub trait Ticker: DeltaTime + ElapsedTime {
    fn tick(&mut self, delta: Duration);
}

#[derive(new, Resource)]
/// Optional runtime multiplier applied to ticking resources (debug only).
pub struct TimeMultiplier<T: Ticker> {
    #[new(default)]
    _phantom: PhantomData<T>,
    pub value: f32,
}

#[cfg(not(debug_assertions))]
/// @system Ticks a `Ticker` resource using Bevy's `Time` delta (release builds).
pub fn tick_time<T: Ticker + Resource>(mut time: ResMut<T>, app_time: Res<Time>) {
    time.tick(app_time.delta());
}
#[cfg(debug_assertions)]
/// @system Ticks a `Ticker` resource, respecting optional debug multipliers.
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
