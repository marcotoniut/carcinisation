//! Time helpers shared across plugins (stage, movement, etc.).

use bevy::prelude::*;
use derive_new::new;
use std::marker::PhantomData;
use std::time::Duration;

#[derive(new, Resource)]
/// Optional runtime multiplier applied to time domains (debug only).
pub struct TimeMultiplier<D: Send + Sync + 'static> {
    #[new(default)]
    _phantom: PhantomData<D>,
    pub value: f32,
}

#[derive(Resource)]
/// Controls whether a time domain should advance or hold at zero delta.
pub struct TimeShouldRun<D: Send + Sync + 'static> {
    _phantom: PhantomData<D>,
    pub value: bool,
}

impl<D: Send + Sync + 'static> Default for TimeShouldRun<D> {
    fn default() -> Self {
        Self {
            _phantom: PhantomData,
            value: true,
        }
    }
}

#[cfg(not(debug_assertions))]
/// @system Advances a `Time<T>` domain using the delta of a source `Time<S>` (release builds).
pub fn tick_time<S: Send + Sync + Default + 'static, T: Send + Sync + Default + 'static>(
    mut target: ResMut<Time<T>>,
    source: Res<Time<S>>,
    time_should_run: Option<Res<TimeShouldRun<T>>>,
) {
    if time_should_run.is_none_or(|x| x.value) {
        target.advance_by(source.delta());
    } else {
        target.advance_by(Duration::ZERO);
    }
}

#[cfg(debug_assertions)]
/// @system Advances a `Time<T>` domain using the delta of a source `Time<S>`, respecting optional multipliers (debug builds).
pub fn tick_time<S: Send + Sync + Default + 'static, T: Send + Sync + Default + 'static>(
    mut target: ResMut<Time<T>>,
    source: Res<Time<S>>,
    time_should_run: Option<Res<TimeShouldRun<T>>>,
    time_multiplier: Option<Res<TimeMultiplier<T>>>,
) {
    if time_should_run.is_none_or(|x| x.value) {
        let multiplier = time_multiplier.map(|x| x.value).unwrap_or(1.0);
        target.advance_by(source.delta().mul_f32(multiplier));
    } else {
        target.advance_by(Duration::ZERO);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::SystemState;
    use bevy::time::Fixed;

    #[derive(Default)]
    struct TestDomain;

    #[test]
    fn tick_time_advances_when_should_run_true() {
        let mut world = World::new();
        let mut source = Time::<Fixed>::default();
        source.advance_by(Duration::from_secs_f32(0.2));
        world.insert_resource(source);
        world.insert_resource(Time::<TestDomain>::default());
        world.insert_resource(TimeShouldRun::<TestDomain> {
            value: true,
            ..Default::default()
        });

        #[allow(clippy::type_complexity)]
        let mut system_state: SystemState<(
            ResMut<Time<TestDomain>>,
            Res<Time<Fixed>>,
            Option<Res<TimeShouldRun<TestDomain>>>,
            Option<Res<TimeMultiplier<TestDomain>>>,
        )> = SystemState::new(&mut world);
        let (target, source, should_run, multiplier) = system_state.get_mut(&mut world);
        tick_time::<Fixed, TestDomain>(target, source, should_run, multiplier);
        system_state.apply(&mut world);

        let target = world.resource::<Time<TestDomain>>();
        assert_eq!(target.elapsed(), Duration::from_secs_f32(0.2));
    }

    #[test]
    fn tick_time_holds_when_should_run_false() {
        let mut world = World::new();
        let mut source = Time::<Fixed>::default();
        source.advance_by(Duration::from_secs_f32(0.2));
        world.insert_resource(source);
        world.insert_resource(Time::<TestDomain>::default());
        world.insert_resource(TimeShouldRun::<TestDomain> {
            value: false,
            ..Default::default()
        });

        #[allow(clippy::type_complexity)]
        let mut system_state: SystemState<(
            ResMut<Time<TestDomain>>,
            Res<Time<Fixed>>,
            Option<Res<TimeShouldRun<TestDomain>>>,
            Option<Res<TimeMultiplier<TestDomain>>>,
        )> = SystemState::new(&mut world);
        let (target, source, should_run, multiplier) = system_state.get_mut(&mut world);
        tick_time::<Fixed, TestDomain>(target, source, should_run, multiplier);
        system_state.apply(&mut world);

        let target = world.resource::<Time<TestDomain>>();
        assert_eq!(target.delta(), Duration::ZERO);
        assert_eq!(target.elapsed(), Duration::ZERO);
    }
}
