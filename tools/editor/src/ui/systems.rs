use bevy::{prelude::*, window::PrimaryWindow};
use bevy_inspector_egui::{
    bevy_egui::EguiContext,
    egui::{self, epaint::Shadow},
};

use crate::resources::{StageControlsUI, StageElapsedUI};

pub fn update_ui(world: &mut World) {
    let Ok((egui_context, window)) = world
        .query_filtered::<(&mut EguiContext, &Window), With<PrimaryWindow>>()
        .get_single(world)
    else {
        return;
    };
    let mut egui_context = egui_context.clone();

    let egui_style: egui::Style = egui::Style {
        visuals: egui::Visuals {
            window_shadow: Shadow {
                offset: egui::Vec2::new(0.0, 0.0),
                ..default()
            },
            ..egui::Visuals::dark()
        },
        ..default()
    };

    let ctx = egui_context.get_mut();
    egui::Window::new("Stage Elapsed")
        .anchor(egui::Align2::LEFT_TOP, [0.0, 70.0])
        .collapsible(false)
        .resizable(false)
        .movable(false)
        .show(ctx, |ui| {
            ctx.set_style(egui_style.clone());
            egui::ScrollArea::horizontal().show(ui, |ui| {
                bevy_inspector_egui::bevy_inspector::ui_for_resource::<StageElapsedUI>(world, ui);
            });
        });

    egui::Window::new("Depth UI")
        .anchor(egui::Align2::LEFT_TOP, [0.0, 137.0])
        .resizable(false)
        .show(ctx, |ui| {
            ctx.set_style(egui_style.clone());
            egui::ScrollArea::horizontal().show(ui, |ui| {
                bevy_inspector_egui::bevy_inspector::ui_for_resource::<StageControlsUI>(world, ui);
            });
        });
}
