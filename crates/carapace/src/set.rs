//! Sets used by this crate

use crate::prelude::*;

// TODO Many of these aren't necessary anymore
/// Sets used by this crate
#[derive(Clone, Debug, Eq, Hash, PartialEq, SystemSet)]
pub enum CxSet {
    // `PreUpdate`
    /// [`crate::cursor::CxCursorPosition`] is updated. In [`CoreSet::PreUpdate`].
    UpdateCursorPosition,

    // `PostUpdate`
    /// [`CxPosition`] (integer cache) is synced from [`WorldPos`]
    /// (authoritative world-space float). In `PostUpdate`, after all gameplay
    /// writes during `Update` are complete.
    SyncPosFromWorld,
    /// Game-side composite presentation writes must finish before `carapace`
    /// syncs composite metrics. In `PostUpdate`.
    CompositePresentationWrites,
    /// Animations are completed. In [`CoreSet::PostUpdate`].
    FinishAnimations,
    /// Update particle emitters. In [`CoreSet::PostUpdate`].
    #[cfg(feature = "particle")]
    UpdateEmitters,
    /// Picking backend runs. In [`CoreSet::PostUpdate`].
    Picking,
}
