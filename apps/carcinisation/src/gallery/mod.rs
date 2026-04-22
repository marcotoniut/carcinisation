//! Character gallery: standalone viewer for enemy sprites and animations.
//!
//! This is an asset-preview authority model, not the gameplay spawn pipeline.
//! Gallery spawns must not be used as a reference for the stage spawn-time
//! presentation invariant.

pub mod components;
pub mod messages;
pub mod resources;
mod systems;

use self::{
    messages::GalleryStartupEvent,
    resources::GalleryState,
    systems::{
        apply_gallery_animation, apply_gallery_playback_control, cleanup_gallery, gallery_panel_ui,
        on_gallery_startup, react_to_gallery_selection_changed, update_gallery_composed_animation,
    },
};
use crate::{
    core::time::tick_time,
    stage::{
        enemy::composed::{
            apply_composed_enemy_visuals, ensure_composed_enemy_parts,
            prepare_composed_atlas_assets, update_composed_enemy_visuals,
        },
        player::PlayerPlugin,
        resources::StageTimeDomain,
    },
};
use activable::{Activable, ActivableAppExt, activate_system, deactivate_system};
use bevy::prelude::*;
use bevy_inspector_egui::bevy_egui::EguiPrimaryContextPass;
use carapace::set::CxSet;

/// Registers the gallery scene: egui panel, animation viewers, and player integration.
#[derive(Activable)]
pub struct GalleryPlugin;

impl Plugin for GalleryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GalleryState>()
            .add_message::<GalleryStartupEvent>()
            .add_observer(on_gallery_startup)
            .add_systems(EguiPrimaryContextPass, gallery_panel_ui)
            .on_active::<GalleryPlugin, _>(activate_system::<PlayerPlugin>)
            .on_inactive::<GalleryPlugin, _>((deactivate_system::<PlayerPlugin>, cleanup_gallery))
            .add_active_systems_in::<GalleryPlugin, _>(
                FixedUpdate,
                tick_time::<Fixed, StageTimeDomain>,
            )
            .add_active_systems_in::<GalleryPlugin, _>(
                PostUpdate,
                apply_composed_enemy_visuals.in_set(CxSet::CompositePresentationWrites),
            )
            .add_active_systems::<GalleryPlugin, _>((
                react_to_gallery_selection_changed,
                apply_gallery_animation,
                apply_gallery_playback_control,
                update_gallery_composed_animation,
                (
                    prepare_composed_atlas_assets,
                    ensure_composed_enemy_parts,
                    update_composed_enemy_visuals,
                )
                    .chain(),
            ));
    }
}
