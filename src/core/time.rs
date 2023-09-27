pub trait DeltaTime: Send + Sync + 'static {
    fn delta_seconds(&self) -> f32;
}
