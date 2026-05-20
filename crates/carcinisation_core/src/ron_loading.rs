//! Unified RON config loading with build-mode-aware strategy.
//!
//! - **Production** (`hot_reload` disabled): uses the embedded string directly.
//!   No filesystem access compiled in.
//! - **Dev** (`hot_reload` enabled): tries filesystem first, falls back to
//!   embedded on read/parse error with a warning.
//!
//! The [`load_ron`] function is the single entry point. The [`ron_config!`]
//! macro eliminates dual-path maintenance by deriving both the `include_str!`
//! constant and the `load_ron` call from one relative asset path.

#[cfg(all(feature = "hot_reload", target_arch = "wasm32"))]
compile_error!("hot_reload cannot be enabled on wasm32 targets (requires std::fs)");

use serde::de::DeserializeOwned;

/// Load a RON config value.
///
/// In production builds the `path` parameter does not exist — only the
/// compile-time embedded string is used.
///
/// In dev builds (`hot_reload` feature), the filesystem path is read first.
/// If the file is missing or contains invalid RON, the embedded fallback is
/// used and a warning is logged.
///
/// Set `CARCINISATION_STRICT_RON=1` to panic on filesystem parse errors instead
/// of falling back to embedded. Useful during active tuning sessions where
/// silent fallback would mask typos.
///
/// # Panics
///
/// Panics if the **embedded** RON fails to parse. Embedded data is validated
/// by unit tests and must always be correct.
///
/// Also panics on filesystem parse errors when `CARCINISATION_STRICT_RON=1`.
#[must_use]
pub fn load_ron<T: DeserializeOwned>(
    embedded: &str,
    #[cfg(feature = "hot_reload")] path: &str,
) -> T {
    #[cfg(feature = "hot_reload")]
    {
        match std::fs::read_to_string(path) {
            Ok(body) => match ron::from_str::<T>(&body) {
                Ok(val) => return val,
                Err(e) => {
                    assert!(
                        !std::env::var("CARCINISATION_STRICT_RON").is_ok_and(|v| v == "1"),
                        "{path}: RON parse error ({e}) [strict mode]"
                    );
                    bevy::log::warn!("{path}: RON parse error ({e}), using embedded fallback");
                }
            },
            Err(e) => {
                bevy::log::debug!("{path}: not found or unreadable ({e}), using embedded");
            }
        }
    }

    ron::from_str(embedded).expect("embedded RON must parse")
}

/// Load a RON config from a workspace-relative asset path.
///
/// Generates the `include_str!` constant and `load_ron` call from a single
/// path, eliminating the dual-maintenance of embedded and filesystem paths.
///
/// The path is relative to the workspace root (e.g., `"assets/config/player.ron"`).
/// The macro assumes the calling crate is two directories below the workspace root
/// (`crates/X/` or `apps/X/`), which is the standard layout.
///
/// # Example
///
/// ```rust,ignore
/// use carcinisation_core::ron_config;
///
/// let config: PlayerConfig = ron_config!("assets/config/player.ron");
/// ```
#[macro_export]
macro_rules! ron_config {
    ($path:literal) => {{
        const EMBEDDED: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../", $path));
        $crate::ron_loading::load_ron(
            EMBEDDED,
            #[cfg(feature = "hot_reload")]
            $path,
        )
    }};
}

/// Load binary asset data.
///
/// Same strategy as [`load_ron`]: production uses the embedded bytes directly,
/// dev reads from filesystem first with embedded fallback.
#[must_use]
pub fn load_binary<'a>(
    embedded: &'a [u8],
    #[cfg(feature = "hot_reload")] path: &str,
) -> std::borrow::Cow<'a, [u8]> {
    #[cfg(feature = "hot_reload")]
    {
        match std::fs::read(path) {
            Ok(bytes) => return std::borrow::Cow::Owned(bytes),
            Err(e) => {
                bevy::log::debug!("{path}: not found or unreadable ({e}), using embedded");
            }
        }
    }

    std::borrow::Cow::Borrowed(embedded)
}

/// Load a binary asset from a workspace-relative path.
///
/// Like [`ron_config!`] but for `include_bytes!` data. Returns `Cow<[u8]>`:
/// borrowed from the embedded constant in production, owned from filesystem
/// in dev mode.
///
/// # Example
///
/// ```rust,ignore
/// let pxi_data = carcinisation_core::binary_asset!("assets/sprites/ui/stage_gun_weapon/atlas.pxi");
/// ```
#[macro_export]
macro_rules! binary_asset {
    ($path:literal) => {{
        const EMBEDDED: &[u8] =
            include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../", $path));
        $crate::ron_loading::load_binary(
            EMBEDDED,
            #[cfg(feature = "hot_reload")]
            $path,
        )
    }};
}

/// Register a RON config path with the [`ConfigFileWatcher`](crate::dev_reload::ConfigFileWatcher)
/// for auto-polling. Call from plugin `build()`.
///
/// Initialises `ConfigFileWatcher` if not yet present, so call order
/// relative to `DevReloadPlugin` does not matter.
///
/// # Example
///
/// ```rust,ignore
/// #[cfg(feature = "hot_reload")]
/// carcinisation_core::watch_config!(app, "assets/config/player.ron");
/// ```
#[cfg(feature = "hot_reload")]
#[macro_export]
macro_rules! watch_config {
    ($app:expr, $path:literal) => {
        $app.init_resource::<$crate::dev_reload::ConfigFileWatcher>();
        $app.world_mut()
            .resource_mut::<$crate::dev_reload::ConfigFileWatcher>()
            .watch($path);
    };
}

/// Generate a Bevy system that reloads a RON config Resource on
/// [`DevReloadRequest`](crate::dev_reload::DevReloadRequest).
///
/// The generated system listens for `DevReloadRequest` messages and re-reads
/// the RON file from disk, updating the Resource in place.
///
/// The `events` parameter uses `Option<MessageReader<...>>` because the reload
/// systems may be registered before `DevReloadPlugin` calls `add_message`.
/// When the message type isn't initialised yet, the system skips gracefully.
///
/// Supports an optional validation closure. During hot reload, validation
/// failures log a warning and keep the previous value (no panic). At startup,
/// `load()` + `validate()` still panics on bad embedded data as before.
///
/// # Examples
///
/// ```rust,ignore
/// // Without validation:
/// carcinisation_core::reload_ron_system!(
///     reload_burn_config,
///     carcinisation_fps_core::BurnConfig,
///     "assets/config/status/burning.ron"
/// );
///
/// // With validation:
/// carcinisation_core::reload_ron_system!(
///     reload_player_config,
///     PlayerConfig,
///     "assets/config/player.ron",
///     |c: &PlayerConfig| c.validate()
/// );
/// ```
#[cfg(feature = "hot_reload")]
#[macro_export]
macro_rules! reload_ron_system {
    // Without validation.
    ($fn_name:ident, $ty:ty, $path:literal) => {
        fn $fn_name(
            events: Option<bevy::prelude::MessageReader<$crate::dev_reload::DevReloadRequest>>,
            mut resource: bevy::prelude::ResMut<$ty>,
        ) {
            let Some(mut events) = events else { return };
            // Drain all events and reload only once — auto-poll and Cmd+R
            // can both fire in the same frame.
            if events.read().count() == 0 {
                return;
            }
            let reloaded: $ty = $crate::ron_config!($path);
            *resource = reloaded;
            bevy::log::info!(concat!("Reloaded ", stringify!($ty), " from ", $path));
        }
    };
    // With validation closure. Panics are caught during hot reload to keep the
    // previous value; at startup the normal `load()` + `validate()` path still
    // panics on bad data.
    ($fn_name:ident, $ty:ty, $path:literal, $validate:expr) => {
        fn $fn_name(
            events: Option<bevy::prelude::MessageReader<$crate::dev_reload::DevReloadRequest>>,
            mut resource: bevy::prelude::ResMut<$ty>,
        ) {
            let Some(mut events) = events else { return };
            if events.read().next().is_none() {
                return;
            }
            let reloaded: $ty = $crate::ron_config!($path);
            let validate_fn: fn(&$ty) = $validate;
            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                validate_fn(&reloaded);
            })) {
                Ok(()) => {
                    *resource = reloaded;
                    bevy::log::info!(concat!("Reloaded ", stringify!($ty), " from ", $path));
                }
                Err(_) => {
                    bevy::log::warn!(concat!(
                        stringify!($ty),
                        " from ",
                        $path,
                        ": validation failed, keeping previous value"
                    ));
                }
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq, serde::Deserialize)]
    struct Simple {
        value: f32,
    }

    #[test]
    fn parses_valid_embedded_ron() {
        let result: Simple = load_ron(
            "Simple(value: 42.0)",
            #[cfg(feature = "hot_reload")]
            "/nonexistent/path.ron",
        );
        assert_eq!(result, Simple { value: 42.0 });
    }

    #[test]
    #[should_panic(expected = "embedded RON must parse")]
    fn panics_on_invalid_embedded_ron() {
        let _: Simple = load_ron(
            "not valid ron {{{",
            #[cfg(feature = "hot_reload")]
            "/nonexistent/path.ron",
        );
    }
}
