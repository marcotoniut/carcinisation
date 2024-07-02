use bevy::{prelude::*, window::PrimaryWindow};
use bevy_inspector_egui::{
    bevy_egui::EguiContext,
    egui::{self, epaint::Shadow},
};

use crate::resources::StageElapsedUI;

pub fn update_ui(world: &mut World) {
    let Ok((egui_context, window)) = world
        .query_filtered::<(&mut EguiContext, &Window), With<PrimaryWindow>>()
        .get_single(world)
    else {
        return;
    };
    let mut egui_context = egui_context.clone();

    let ctx = egui_context.get_mut();
    egui::Window::new("Stage Elapsed")
        .anchor(egui::Align2::LEFT_TOP, [0.0, 70.0])
        .collapsible(false)
        .resizable(false)
        .movable(false)
        .show(ctx, |ui| {
            ctx.set_style(egui::Style {
                visuals: egui::Visuals {
                    window_shadow: Shadow {
                        offset: egui::Vec2::new(0.0, 0.0),
                        ..default()
                    },
                    ..egui::Visuals::dark()
                },
                ..default()
            });
            egui::ScrollArea::horizontal().show(ui, |ui| {
                bevy_inspector_egui::bevy_inspector::ui_for_resource::<StageElapsedUI>(world, ui);
            });
        });
}
