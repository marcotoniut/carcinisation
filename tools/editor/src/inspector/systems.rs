use bevy::{prelude::*, window::PrimaryWindow};
use bevy_inspector_egui::{
    bevy_egui::EguiContext,
    egui::{self, Align2},
};

use crate::components::LoadedScene;

pub fn inspector_ui(world: &mut World) {
    let Ok((egui_context, window)) = world
        .query_filtered::<(&mut EguiContext, &Window), With<PrimaryWindow>>()
        .get_single(world)
    else {
        return;
    };
    let mut egui_context = egui_context.clone();

    let window_height = window.height();

    let ctx = egui_context.get_mut();

    let window = egui::Window::new("World");
    window
        .anchor(Align2::LEFT_BOTTOM, [0.0, 0.0])
        .default_open(false)
        .default_height(275.0)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                bevy_inspector_egui::bevy_inspector::ui_for_world(world, ui);
            });
        });

    egui::Window::new("Scene")
        .anchor(egui::Align2::RIGHT_TOP, [0.0, 0.0])
        .collapsible(false)
        .movable(false)
        .min_height(window_height)
        .max_height(window_height)
        .default_width(525.0)
        .resizable([true, false])
        .show(ctx, |ui| {
            egui::ScrollArea::both().show(ui, |ui| {
                bevy_inspector_egui::bevy_inspector::ui_for_resource::<LoadedScene>(world, ui);
            });
        });
}
