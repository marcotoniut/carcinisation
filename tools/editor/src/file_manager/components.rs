use std::path::PathBuf;

use bevy::{prelude::Component, tasks::Task};

#[derive(Component)]
pub struct SelectedFile(pub Task<Option<PathBuf>>);
