use std::time::Duration;

pub trait DeltaTime: Send + Sync + 'static {
    fn delta_seconds(&self) -> f32;
    fn delta(&self) -> Duration;
}

pub trait ElapsedTime: Send + Sync + 'static {
    fn elapsed(&self) -> Duration;
}
