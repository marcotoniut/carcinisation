use std::path::PathBuf;

use bevy::{prelude::Component, tasks::Task};

/// Async task handle for a file picker selection.
#[derive(Component, Debug)]
pub struct SelectedFile(pub Task<Option<PathBuf>>);
