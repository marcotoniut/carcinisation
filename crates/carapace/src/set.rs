//! Sets used by this crate

use crate::prelude::*;

// TODO Many of these aren't necessary anymore
/// Sets used by this crate
#[derive(Clone, Debug, Eq, Hash, PartialEq, SystemSet)]
pub enum PxSet {
    // `PreUpdate`
    /// [`crate::cursor::PxCursorPosition`] is updated. In [`CoreSet::PreUpdate`].
    UpdateCursorPosition,

    // `PostUpdate`
    /// The [`PxPosition`] is synced from [`PxSubPosition`]. In `PostUpdate`,
    /// after all gameplay writes during `Update` are complete.
    UpdatePosToSubPos,
    /// Animations are completed. In [`CoreSet::PostUpdate`].
    FinishAnimations,
    /// Update particle emitters. In [`CoreSet::PostUpdate`].
    #[cfg(feature = "particle")]
    UpdateEmitters,
    /// Picking backend runs. In [`CoreSet::PostUpdate`].
    Picking,
}
