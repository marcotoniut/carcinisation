//! Dev-only RON hot reload infrastructure.
//!
//! Provides:
//! - [`DevReloadRequest`] message, fired on Cmd+R or when file changes are detected
//! - [`ConfigFileWatcher`] resource that polls registered file paths for mtime changes
//! - [`DevReloadPlugin`] that wires up keybind input + file polling
//!
//! Downstream crates register reload systems via [`reload_ron_system!`](crate::reload_ron_system)
//! that listen for `DevReloadRequest` and re-read configs from disk.
//!
//! Gated behind `cfg(feature = "hot_reload")` — compiled out entirely in
//! production builds.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use bevy::prelude::*;

/// Fired when configs should be reloaded — either from Cmd+R or auto-poll.
///
/// Multiple messages may arrive in a single frame (e.g., Cmd+R + poll
/// both fire). Consumers should drain all events and reload once — the
/// [`reload_ron_system!`](crate::reload_ron_system) macro handles this.
#[derive(Event, Message, Debug, Clone, Copy)]
pub struct DevReloadRequest;

/// Tracks filesystem modification times for registered config files.
///
/// Downstream plugins call [`watch`](Self::watch) during setup to register
/// their RON file paths. The [`poll_config_changes`] system checks mtimes
/// periodically and fires [`DevReloadRequest`] when any file changes.
#[derive(Resource, Default)]
pub struct ConfigFileWatcher {
    entries: HashMap<PathBuf, Option<SystemTime>>,
}

impl ConfigFileWatcher {
    /// Register a file path for change detection.
    pub fn watch(&mut self, path: impl Into<PathBuf>) {
        let path = path.into();
        let mtime = Self::read_mtime(&path);
        self.entries.insert(path, mtime);
    }

    /// Check all watched files for mtime changes. Returns `true` if any changed.
    ///
    /// Only triggers on files that exist with a new mtime — temporary
    /// disappearances during atomic saves (write-to-temp + rename) are
    /// ignored to avoid spurious reloads.
    fn poll(&mut self) -> bool {
        let mut changed = false;
        for (path, last_mtime) in &mut self.entries {
            let current = Self::read_mtime(path);
            if current != *last_mtime {
                // Only trigger when the file exists with a new mtime.
                // If the file disappeared temporarily (atomic save), just
                // update our record without triggering a reload.
                if current.is_some() {
                    info!("{}: file changed", path.display());
                    changed = true;
                }
                *last_mtime = current;
            }
        }
        changed
    }

    fn read_mtime(path: &Path) -> Option<SystemTime> {
        std::fs::metadata(path).and_then(|m| m.modified()).ok()
    }
}

/// Detects Cmd+R (macOS) / Ctrl+R and fires [`DevReloadRequest`].
pub fn dev_reload_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut writer: MessageWriter<DevReloadRequest>,
) {
    let modifier_held = keys.any_pressed([
        KeyCode::SuperLeft,
        KeyCode::SuperRight,
        KeyCode::ControlLeft,
        KeyCode::ControlRight,
    ]);
    if modifier_held && keys.just_pressed(KeyCode::KeyR) {
        info!("Dev reload requested (Cmd+R / Ctrl+R)");
        writer.write(DevReloadRequest);
    }
}

/// Interval between filesystem polls.
const POLL_INTERVAL_SECS: f64 = 0.5;

/// Polls [`ConfigFileWatcher`] for mtime changes and fires [`DevReloadRequest`].
fn poll_config_changes(
    time: Res<Time<Real>>,
    mut watcher: ResMut<ConfigFileWatcher>,
    mut timer: Local<f64>,
    mut writer: MessageWriter<DevReloadRequest>,
) {
    *timer += time.delta_secs_f64();
    if *timer < POLL_INTERVAL_SECS {
        return;
    }
    // Reset to zero, not remainder — we do not want to burst-poll after
    // debugger pauses or long frame hitches.
    *timer = 0.0;

    if watcher.poll() {
        writer.write(DevReloadRequest);
    }
}

/// Registers the dev reload keybind, file watcher, and poll system.
///
/// Safe to add multiple times — skips if already registered. This allows
/// both `FpsPlugin` and `StagePlugin` to add it without coordination.
pub struct DevReloadPlugin;

impl Plugin for DevReloadPlugin {
    fn build(&self, app: &mut App) {
        if app.world().get_resource::<DevReloadRegistered>().is_some() {
            return;
        }
        app.insert_resource(DevReloadRegistered)
            .init_resource::<ConfigFileWatcher>()
            .add_message::<DevReloadRequest>()
            .add_systems(Update, (dev_reload_input, poll_config_changes));
    }
}

/// Marker resource to prevent double-registration of [`DevReloadPlugin`].
#[derive(Resource)]
struct DevReloadRegistered;
