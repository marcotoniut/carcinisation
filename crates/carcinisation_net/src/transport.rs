//! Multiplayer transport boundary.
//!
//! Native multiplayer currently uses the native UDP/netcode transport only. Browser
//! clients cannot use raw UDP sockets, so browser multiplayer is intentionally not
//! exposed by the deployment flow yet.
//!
//! Future browser multiplayer should add a transport behind this boundary instead
//! of coupling gameplay replication directly to platform socket APIs. Prefer
//! WebTransport when hosting/support is ready; WebSocket is acceptable as an
//! earlier bridge if it keeps the gameplay protocol isolated from the browser
//! transport details.

/// Implemented multiplayer transports.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MultiplayerTransport {
    /// Native UDP transport used by the dedicated server and native clients.
    NativeUdp,
}

impl MultiplayerTransport {
    #[must_use]
    pub const fn supports_browser(self) -> bool {
        match self {
            Self::NativeUdp => false,
        }
    }
}
