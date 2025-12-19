use bevy::{prelude::*, window::PrimaryWindow};
use bevy_inspector_egui::{
    bevy_egui::{EguiContext, PrimaryEguiContext},
    egui::{self, epaint::Shadow},
};

use crate::components::SceneData;
use crate::resources::StageControlsUI;
use crate::timeline::{StageTimeline, StageTimelineConfig};
use std::time::Duration;

pub fn update_ui(world: &mut World) {
    let window_width = {
        let Ok(window) = world
            .query_filtered::<&Window, With<PrimaryWindow>>()
            .single(world)
        else {
            return;
        };
        window.width()
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

    let egui_style: egui::Style = egui::Style {
        visuals: egui::Visuals {
            window_shadow: Shadow {
                offset: [0, 0],
                ..default()
            },
            ..egui::Visuals::dark()
        },
        ..default()
    };

    let ctx = egui_context.get_mut();

    if let Some(stage_data) =
        world
            .get_resource::<SceneData>()
            .and_then(|scene_data| match scene_data {
                SceneData::Stage(stage_data) => Some(stage_data.clone()),
                _ => None,
            })
    {
        let timeline = StageTimeline::from_stage(&stage_data, StageTimelineConfig::SLIDER);
        let mut current_elapsed = {
            let mut stage_controls_ui = world.resource_mut::<StageControlsUI>();
            if stage_controls_ui.elapsed_duration > timeline.total {
                stage_controls_ui.elapsed_duration = timeline.total;
            }
            stage_controls_ui.elapsed_duration
        };
        current_elapsed = timeline.clamp_elapsed(current_elapsed);

        let slider_width = window_width * 0.6;
        let step_index = timeline.step_index_at(current_elapsed);
        let total_secs = timeline.total.as_secs_f32().max(0.0001);
        let current_secs = current_elapsed.as_secs_f32().clamp(0.0, total_secs);
        let mut t = (current_secs / total_secs).clamp(0.0, 1.0);

        egui::Window::new("camera_elapsed_slider")
            .anchor(egui::Align2::CENTER_BOTTOM, [0.0, -10.0])
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .default_width(slider_width)
            .min_width(slider_width)
            .max_width(slider_width)
            .show(ctx, |ui| {
                let step_text = format!("Step {}", step_index);
                let button_height = ui.spacing().interact_size.y;
                let value_width = 64.0;
                let step_width = 72.0;
                let mut edited_secs = current_secs;
                let mut edited = false;

                ui.horizontal(|ui| {
                    let track_width = (ui.available_width()
                        - value_width
                        - step_width
                        - ui.spacing().item_spacing.x * 2.0)
                        .max(0.0);

                    let (rect, response) = ui.allocate_exact_size(
                        egui::vec2(track_width, button_height),
                        egui::Sense::click_and_drag(),
                    );

                    let visuals = ui.style().interact(&response);
                    let radius = rect.height() * 0.25;
                    let track_rect = rect.shrink2(egui::vec2(radius, rect.height() * 0.3));
                    ui.painter()
                        .rect_filled(track_rect, radius, visuals.bg_fill);
                    ui.painter().rect_stroke(
                        track_rect,
                        radius,
                        visuals.bg_stroke,
                        egui::StrokeKind::Inside,
                    );

                    let fill_width = track_rect.width() * t;
                    if fill_width > 0.0 {
                        let fill_rect = egui::Rect::from_min_max(
                            track_rect.min,
                            egui::pos2(track_rect.left() + fill_width, track_rect.bottom()),
                        );
                        ui.painter()
                            .rect_filled(fill_rect, radius, ui.visuals().selection.bg_fill);
                    }

                    let thumb_x = track_rect.left() + track_rect.width() * t;
                    let thumb_center = egui::pos2(thumb_x, rect.center().y);
                    ui.painter().circle_filled(
                        thumb_center,
                        rect.height() * 0.35,
                        ui.visuals().widgets.active.bg_fill,
                    );

                    if track_rect.width() > 0.0 && (response.dragged() || response.clicked()) {
                        if let Some(pointer) = response.interact_pointer_pos() {
                            let new_t = ((pointer.x - track_rect.left()) / track_rect.width())
                                .clamp(0.0, 1.0);
                            t = new_t;
                        }
                    }

                    let value_response = ui.add_sized(
                        [value_width, button_height],
                        egui::DragValue::new(&mut edited_secs)
                            .range(0.0..=total_secs)
                            .speed(0.1)
                            .max_decimals(3),
                    );
                    if value_response.changed() {
                        edited = true;
                    }
                    ui.add_sized([step_width, button_height], egui::Label::new(step_text));
                });

                if edited {
                    t = (edited_secs / total_secs).clamp(0.0, 1.0);
                }
                let target_secs = total_secs * t;
                if (target_secs - current_secs).abs() > f32::EPSILON {
                    let mut stage_controls_ui = world.resource_mut::<StageControlsUI>();
                    stage_controls_ui.elapsed_duration = Duration::from_secs_f32(target_secs);
                }
            });
    }

    egui::Window::new("Stage Controls")
        .anchor(egui::Align2::LEFT_TOP, [0.0, 30.0])
        .resizable(false)
        .show(ctx, |ui| {
            ctx.set_style(egui_style.clone());
            egui::ScrollArea::horizontal().show(ui, |ui| {
                bevy_inspector_egui::bevy_inspector::ui_for_resource::<StageControlsUI>(world, ui);
            });
        });
}
