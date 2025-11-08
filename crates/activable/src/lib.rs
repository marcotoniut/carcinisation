//! Per-plugin activation state for Bevy.
//!
//! Each `#[derive(Activable)]` type `P` owns an [`ActiveState<P>`] (`Active`/`Inactive`)
//! that gates its systems. Use [`ActivableAppExt`] to register lifecycle systems and
//! [`activate::<P>`]/[`deactivate::<P>`] or [`activate_system::<P>`] to toggle state.

use bevy::ecs::system::ScheduleSystem;
use bevy::prelude::*;
pub use bevy::prelude::{App, Commands, States, Update};
use bevy_ecs::schedule::{IntoScheduleConfigs, ScheduleLabel};
use bevy_state::state::FreelyMutableState;
use core::{
    fmt,
    hash::{Hash, Hasher},
};
use std::marker::PhantomData;

pub use activable_macros::Activable;

/// Internal sealing for [`Activable`].
pub mod sealed {
    pub trait Sealed {}
}

/// Marker for types with an [`ActiveState`].
pub trait Activable: sealed::Sealed + 'static + Send + Sync {}

/// Binary state for an [`Activable`] type (distinct per `P`).
pub enum ActiveState<P: Activable> {
    Inactive(PhantomData<P>),
    Active(PhantomData<P>),
}

impl<P: Activable> ActiveState<P> {
    #[inline]
    pub fn inactive() -> Self {
        Self::Inactive(PhantomData)
    }
    #[inline]
    pub fn active() -> Self {
        Self::Active(PhantomData)
    }

    #[inline]
    fn is_active(&self) -> bool {
        matches!(self, Self::Active(_))
    }
}

impl<P: Activable> Default for ActiveState<P> {
    fn default() -> Self {
        Self::inactive()
    }
}
impl<P: Activable> Copy for ActiveState<P> {}
impl<P: Activable> Clone for ActiveState<P> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<P: Activable> PartialEq for ActiveState<P> {
    fn eq(&self, o: &Self) -> bool {
        self.is_active() == o.is_active()
    }
}
impl<P: Activable> Eq for ActiveState<P> {}
impl<P: Activable> Hash for ActiveState<P> {
    fn hash<H: Hasher>(&self, h: &mut H) {
        self.is_active().hash(h)
    }
}
impl<P: Activable> fmt::Debug for ActiveState<P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(if self.is_active() {
            "Active"
        } else {
            "Inactive"
        })
    }
}
impl<P: Activable> States for ActiveState<P> {}
impl<P: Activable> FreelyMutableState for ActiveState<P> {}

fn ensure_state<P: Activable>(app: &mut App) -> &mut App {
    if !app.world().contains_resource::<State<ActiveState<P>>>() {
        app.init_state::<ActiveState<P>>();
    }
    app
}

/// Extends [`App`] with helpers for registering systems bound to [`ActiveState<P>`].
pub trait ActivableAppExt {
    /// Runs once when entering `Active`.
    fn on_active<P, M>(
        &mut self,
        systems: impl IntoScheduleConfigs<ScheduleSystem, M>,
    ) -> &mut Self
    where
        P: Activable;

    /// Runs once when leaving `Active`.
    fn on_inactive<P, M>(
        &mut self,
        systems: impl IntoScheduleConfigs<ScheduleSystem, M>,
    ) -> &mut Self
    where
        P: Activable;

    /// Runs each frame while `Active` on [`Update`].
    fn add_active_systems<P, M>(
        &mut self,
        systems: impl IntoScheduleConfigs<ScheduleSystem, M>,
    ) -> &mut Self
    where
        P: Activable;

    /// Runs each frame while `Active` on a custom schedule.
    fn add_active_systems_in<P, M>(
        &mut self,
        schedule: impl ScheduleLabel,
        systems: impl IntoScheduleConfigs<ScheduleSystem, M>,
    ) -> &mut Self
    where
        P: Activable;
}

impl ActivableAppExt for App {
    fn on_active<P, M>(&mut self, systems: impl IntoScheduleConfigs<ScheduleSystem, M>) -> &mut Self
    where
        P: Activable,
    {
        ensure_state::<P>(self).add_systems(OnEnter(ActiveState::<P>::active()), systems)
    }

    fn on_inactive<P, M>(
        &mut self,
        systems: impl IntoScheduleConfigs<ScheduleSystem, M>,
    ) -> &mut Self
    where
        P: Activable,
    {
        ensure_state::<P>(self).add_systems(OnExit(ActiveState::<P>::active()), systems)
    }

    fn add_active_systems<P, M>(
        &mut self,
        systems: impl IntoScheduleConfigs<ScheduleSystem, M>,
    ) -> &mut Self
    where
        P: Activable,
    {
        self.add_active_systems_in::<P, _>(Update, systems)
    }

    fn add_active_systems_in<P, M>(
        &mut self,
        schedule: impl ScheduleLabel,
        systems: impl IntoScheduleConfigs<ScheduleSystem, M>,
    ) -> &mut Self
    where
        P: Activable,
    {
        ensure_state::<P>(self).add_systems(
            schedule,
            systems.run_if(in_state(ActiveState::<P>::active())),
        )
    }
}

/// Queues activation of `P`. No-ops if its state is unregistered.
pub fn activate<P: Activable>(commands: &mut Commands) {
    commands.queue(ActivateCommand::<P>(PhantomData));
}

/// Queues deactivation of `P`. No-ops if its state is unregistered.
pub fn deactivate<P: Activable>(commands: &mut Commands) {
    commands.queue(DeactivateCommand::<P>(PhantomData));
}

struct ActivateCommand<P: Activable>(PhantomData<P>);
impl<P: Activable> bevy::ecs::system::Command for ActivateCommand<P> {
    fn apply(self, world: &mut World) {
        if let Some(mut next) = world.get_resource_mut::<NextState<ActiveState<P>>>() {
            *next = NextState::Pending(ActiveState::<P>::active());
        }
    }
}

struct DeactivateCommand<P: Activable>(PhantomData<P>);
impl<P: Activable> bevy::ecs::system::Command for DeactivateCommand<P> {
    fn apply(self, world: &mut World) {
        if let Some(mut next) = world.get_resource_mut::<NextState<ActiveState<P>>>() {
            *next = NextState::Pending(ActiveState::<P>::inactive());
        }
    }
}

/// @system Activates the plugin type `P`.
pub fn activate_system<P: Activable>(mut commands: Commands) {
    activate::<P>(&mut commands);
}

/// @system Deactivates the plugin type `P`.
pub fn deactivate_system<P: Activable>(mut commands: Commands) {
    deactivate::<P>(&mut commands);
}
