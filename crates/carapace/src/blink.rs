//! Contains the [`CxBlink`] component.

use std::time::Duration;

use bevy_derive::{Deref, DerefMut};

use crate::prelude::*;

pub(crate) fn plug(app: &mut App) {
    #[cfg(feature = "headed")]
    app.add_systems(PostUpdate, blink);
}

/// Toggles [`Visibility`] on a repeating timer.
#[derive(Component, Deref, DerefMut, Reflect)]
#[cfg_attr(feature = "headed", require(Visibility))]
pub struct CxBlink(Timer);

impl CxBlink {
    /// Creates a `CxBlink` with the given period
    #[must_use]
    pub fn new(period: Duration) -> Self {
        Self(Timer::new(period, TimerMode::Repeating))
    }
}

#[cfg(feature = "headed")]
fn blink(mut blinks: Query<(&mut CxBlink, &mut Visibility)>, time: Res<Time>) {
    for (mut blink, mut visibility) in &mut blinks {
        blink.tick(time.delta());

        if blink.just_finished() {
            visibility.toggle_inherited_hidden();
        }
    }
}
