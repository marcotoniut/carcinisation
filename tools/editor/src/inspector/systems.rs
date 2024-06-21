use bevy::{prelude::*, window::PrimaryWindow};
use bevy_inspector_egui::{
    bevy_egui::EguiContext,
    egui::{self, Align2},
};

pub fn inspector_ui(world: &mut World) {
    let Ok(egui_context) = world
        .query_filtered::<&mut EguiContext, With<PrimaryWindow>>()
        .get_single(world)
    else {
        return;
    };
    let mut egui_context = egui_context.clone();

    let window = egui::Window::new("UI");
    window
        .anchor(Align2::RIGHT_TOP, [10.0, 10.0])
        .show(egui_context.get_mut(), |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                // equivalent to `WorldInspectorPlugin`
                bevy_inspector_egui::bevy_inspector::ui_for_world(world, ui);
            });
        });
}
