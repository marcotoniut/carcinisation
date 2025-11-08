//! Marker components for main menu entities.

use bevy::prelude::*;

#[derive(Component)]
/// Root marker for the main menu hierarchy.
pub struct MainMenu;

#[derive(Component)]
/// Generic marker for menu-owned entities (cleanup helper).
pub struct MainMenuEntity;

#[derive(Component)]
/// Marks the selection list entry.
pub struct MainMenuSelect;

#[derive(Component)]
/// Entity for the main selection screen container.
pub struct MainMenuSelectScreenEntity;

#[derive(Component)]
/// Entity for the press-start prompt screen.
pub struct PressStartScreenEntity;

#[derive(Component)]
/// Entity for the difficulty selection screen.
pub struct DifficultySelectScreenEntity;

#[derive(Component)]
/// Arrow indicator that points at the currently selected difficulty.
pub struct DifficultySelectionIndicator;
