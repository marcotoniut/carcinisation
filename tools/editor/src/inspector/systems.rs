use bevy::{prelude::*, window::PrimaryWindow};
use bevy_inspector_egui::{
    bevy_egui::{EguiContext, PrimaryEguiContext},
    egui::{self, Align2, epaint::Shadow},
};

use crate::components::{SceneData, ScenePath, SelectedItem};
use crate::file_manager::actions::{request_file_picker, save_scene};

/// @system Renders inspector windows (world, scene, and file path controls).
pub fn inspector_ui(world: &mut World) {
    let window_height = {
        let Ok(window) = world
            .query_filtered::<&Window, With<PrimaryWindow>>()
            .single(world)
        else {
            return;
        };
        window.height()
    };
    let mut egui_context = {
        let Ok(egui_context) = world
            .query_filtered::<&mut EguiContext, With<PrimaryEguiContext>>()
            .single(world)
        else {
            return;
        };
        egui_context.clone()
    };

    let ctx = egui_context.get_mut();
    let scene_path = world.resource::<ScenePath>().0.clone();
    let scene_data = world.get_resource::<SceneData>().cloned();
    let selected_entity = world
        .query_filtered::<Entity, With<SelectedItem>>()
        .iter(world)
        .next();
    let has_stage = matches!(scene_data, Some(SceneData::Stage(_)));

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
                if let Some(entity) = selected_entity {
                    ui.heading("Selection");
                    bevy_inspector_egui::bevy_inspector::ui_for_entity(world, entity, ui);
                } else {
                    bevy_inspector_egui::bevy_inspector::ui_for_resource::<SceneData>(world, ui);
                }
            });
        });

    egui::Window::new("Path")
        .title_bar(false)
        .anchor(egui::Align2::LEFT_TOP, [0.0, 0.0])
        .collapsible(false)
        .resizable(false)
        .movable(false)
        .show(ctx, |ui| {
            ctx.set_style(egui::Style {
                visuals: egui::Visuals {
                    window_shadow: Shadow {
                        offset: [0, 0],
                        ..default()
                    },
                    ..egui::Visuals::dark()
                },
                ..default()
            });
            ui.horizontal(|ui| {
                let button_height = ui.spacing().interact_size.y;
                let save_width = 46.0;
                let select_width = 92.0;
                let button_total = save_width + select_width + ui.spacing().item_spacing.x * 2.0;
                let path_width = (ui.available_width() - button_total).max(140.0);

                ui.add_sized(
                    [path_width, button_height],
                    egui::Label::new(scene_path.clone()),
                );

                let save_text = egui::RichText::new("Save").size(12.0);
                let select_text = egui::RichText::new("Select file").size(12.0);

                if ui
                    .add_enabled(
                        has_stage,
                        egui::Button::new(save_text).min_size(egui::vec2(save_width, 0.0)),
                    )
                    .clicked()
                {
                    if let Some(scene_data) = scene_data.as_ref() {
                        save_scene(world, &scene_path, scene_data);
                    }
                }

                if ui
                    .add(egui::Button::new(select_text).min_size(egui::vec2(select_width, 0.0)))
                    .clicked()
                {
                    request_file_picker(world);
                }
            });
        });
}
