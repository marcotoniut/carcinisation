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
/// @system Advances a `Time<D>` domain using Bevy's global time delta (release builds).
pub fn tick_time_domain<D: Send + Sync + Default + 'static>(
    mut time: ResMut<Time<D>>,
    app_time: Res<Time>,
) {
    time.advance_by(app_time.delta());
}

#[cfg(debug_assertions)]
/// @system Advances a `Time<D>` domain, respecting optional debug multipliers.
pub fn tick_time_domain<D: Send + Sync + Default + 'static>(
    mut time: ResMut<Time<D>>,
    app_time: Res<Time>,
    time_multiplier: Option<Res<TimeMultiplier<D>>>,
) {
    let delta = app_time
        .delta()
        .mul_f32(time_multiplier.map(|x| x.value).unwrap_or(1.0));
    time.advance_by(delta);
}
