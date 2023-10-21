use bevy::prelude::*;
use std::time::Duration;

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

pub fn tick_time<T: Ticker + Resource>(mut time: ResMut<T>, app_time: Res<Time>) {
    time.tick(app_time.delta());
}
