//! Time helpers shared across plugins (stage, movement, etc.).

use bevy::prelude::*;
use derive_new::new;
use std::marker::PhantomData;

#[derive(new, Resource)]
/// Optional runtime multiplier applied to time domains (debug only).
pub struct TimeMultiplier<D: Send + Sync + 'static> {
    #[new(default)]
    _phantom: PhantomData<D>,
    pub value: f32,
}

#[cfg(not(debug_assertions))]
/// @system Advances a `Time<T>` domain using the delta of a source `Time<S>` (release builds).
pub fn tick_time<S: Send + Sync + Default + 'static, T: Send + Sync + Default + 'static>(
    mut target: ResMut<Time<T>>,
    source: Res<Time<S>>,
) {
    target.advance_by(source.delta());
}

#[cfg(debug_assertions)]
/// @system Advances a `Time<T>` domain using the delta of a source `Time<S>`, respecting optional multipliers (debug builds).
pub fn tick_time<S: Send + Sync + Default + 'static, T: Send + Sync + Default + 'static>(
    mut target: ResMut<Time<T>>,
    source: Res<Time<S>>,
    time_multiplier: Option<Res<TimeMultiplier<T>>>,
) {
    let multiplier = time_multiplier.map(|x| x.value).unwrap_or(1.0);
    target.advance_by(source.delta().mul_f32(multiplier));
}
