use bevy::{prelude::*, window::PrimaryWindow};
use bevy_inspector_egui::{
    bevy_egui::EguiContext,
    egui::{self, epaint::Shadow, Align2},
};

use crate::components::{SceneData, ScenePath};

pub fn inspector_ui(world: &mut World) {
    let Ok((egui_context, window)) = world
        .query_filtered::<(&mut EguiContext, &Window), With<PrimaryWindow>>()
        .single(world)
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
        .title_bar(true)
        .anchor(egui::Align2::RIGHT_TOP, [0.0, 0.0])
        .movable(false)
        .min_height(window_height - 15.0)
        .max_height(window_height - 15.0)
        .default_width(525.0)
        .resizable([true, false])
        .show(ctx, |ui| {
            egui::ScrollArea::both().show(ui, |ui| {
                bevy_inspector_egui::bevy_inspector::ui_for_resource::<SceneData>(world, ui);
            });
        });

    egui::Window::new("Path")
        .title_bar(false)
        .anchor(egui::Align2::LEFT_TOP, [0.0, 35.0])
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
                bevy_inspector_egui::bevy_inspector::ui_for_resource::<ScenePath>(world, ui);
            });
        });
}
