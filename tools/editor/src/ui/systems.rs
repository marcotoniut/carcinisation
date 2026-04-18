use bevy::{prelude::*, window::PrimaryWindow};
use bevy_inspector_egui::{
    bevy_egui::{EguiContext, PrimaryEguiContext},
    egui::{self},
};
use carcinisation::stage::components::placement::Depth;

use crate::components::SceneData;
use crate::placement::{EDITOR_DEPTHS, PlacementMode, PlacementState, SpawnTemplate};
use crate::resources::{EditorState, StageControlsUI};
use crate::timeline::{StageTimeline, StageTimelineConfig};
use crate::ui::style::{apply_editor_style, field_label, section_header};
use std::time::Duration;

/// @system Builds the editor UI: stage timeline slider, stage control toggles, and spawn palette.
#[allow(clippy::too_many_lines)]
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

    let ctx = egui_context.get_mut();
    apply_editor_style(ctx);

    let has_stage = world
        .get_resource::<SceneData>()
        .is_some_and(|sd| matches!(sd, SceneData::Stage(_)));

    if let Some(stage_data) =
        world
            .get_resource::<SceneData>()
            .and_then(|scene_data| match scene_data {
                SceneData::Stage(stage_data) => Some(stage_data.clone()),
                SceneData::Cutscene(_) => None,
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

        let slider_width = window_width * 0.45;
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
                let step_text = format!("Step {step_index}");
                let button_height = ui.spacing().interact_size.y;
                let value_width = 64.0;
                let step_width = 72.0;
                let mut edited_secs = current_secs;
                let mut edited = false;

                ui.horizontal(|ui| {
                    ui.add_sized([step_width, button_height], egui::Label::new(step_text));

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

                    let track_width = ui.available_width().max(0.0);
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

                    if track_rect.width() > 0.0
                        && (response.dragged() || response.clicked())
                        && let Some(pointer) = response.interact_pointer_pos()
                    {
                        let new_t =
                            ((pointer.x - track_rect.left()) / track_rect.width()).clamp(0.0, 1.0);
                        t = new_t;
                    }
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

    stage_controls_window(world, ctx);

    // Spawn palette and undo/redo status
    if has_stage {
        spawn_palette_window(world, ctx);
    }
}

fn stage_controls_window(world: &mut World, ctx: &egui::Context) {
    let mut editor_state = world.resource::<EditorState>().clone();
    let original_editor_state = editor_state.clone();
    let mut controls = world.resource::<StageControlsUI>().clone();
    let original = controls.clone();

    egui::Window::new("Stage Controls")
        .anchor(egui::Align2::LEFT_TOP, [0.0, 30.0])
        .resizable(false)
        .show(ctx, |ui| {
            section_header(ui, "Mode");
            ui.checkbox(&mut editor_state.depth_preview_enabled, "Depth Preview");

            section_header(ui, "Overlays");
            ui.horizontal(|ui| {
                ui.checkbox(&mut controls.elapsed_path, "Path");
                ui.checkbox(&mut controls.skybox, "Skybox");
                ui.checkbox(&mut controls.background, "Background");
            });
            ui.checkbox(&mut controls.show_all_spawns, "Show all spawns (ghost)");

            section_header(ui, "Projection");
            ui.checkbox(&mut controls.projection_grid, "Perspective Grid");
            ui.checkbox(&mut controls.projection_markers, "Horizon / Floor markers");

            section_header(ui, "Depth layers");
            let depths: &mut [(&str, &mut bool)] = &mut [
                ("9", &mut controls.nine),
                ("8", &mut controls.eight),
                ("7", &mut controls.seven),
                ("6", &mut controls.six),
                ("5", &mut controls.five),
                ("4", &mut controls.four),
                ("3", &mut controls.three),
                ("2", &mut controls.two),
                ("1", &mut controls.one),
            ];

            egui::Grid::new("depth_toggles")
                .num_columns(5)
                .spacing([4.0, 2.0])
                .show(ui, |ui| {
                    for (i, (label, value)) in depths.iter_mut().enumerate() {
                        ui.checkbox(value, *label);
                        if (i + 1) % 5 == 0 {
                            ui.end_row();
                        }
                    }
                });
        });

    if editor_state.depth_preview_enabled != original_editor_state.depth_preview_enabled {
        *world.resource_mut::<EditorState>() = editor_state;
    }

    // Write back if changed (compare serialised to avoid spurious change detection).
    if controls.elapsed_path != original.elapsed_path
        || controls.show_all_spawns != original.show_all_spawns
        || controls.skybox != original.skybox
        || controls.background != original.background
        || controls.projection_grid != original.projection_grid
        || controls.projection_markers != original.projection_markers
        || controls.nine != original.nine
        || controls.eight != original.eight
        || controls.seven != original.seven
        || controls.six != original.six
        || controls.five != original.five
        || controls.four != original.four
        || controls.three != original.three
        || controls.two != original.two
        || controls.one != original.one
        || controls.zero != original.zero
    {
        *world.resource_mut::<StageControlsUI>() = controls;
    }
}

#[allow(clippy::too_many_lines)]
fn spawn_palette_window(world: &mut World, ctx: &egui::Context) {
    let placing_label = world
        .get_resource::<PlacementMode>()
        .and_then(|pm| pm.active.as_ref().map(|s| s.template.label().to_string()));
    let current_depth = world
        .get_resource::<PlacementMode>()
        .and_then(|pm| pm.active.as_ref().map(|s| s.depth))
        .unwrap_or(Depth::Three);

    let objects = SpawnTemplate::all_objects();
    let destructibles = SpawnTemplate::all_destructibles();
    let pickups = SpawnTemplate::all_pickups();
    let enemies = SpawnTemplate::all_enemies();

    let mut set_placement: Option<SpawnTemplate> = None;
    let mut set_depth: Option<Depth> = None;
    let mut set_animation_tag: Option<Option<String>> = None;

    egui::Window::new("Spawn Palette")
        .default_pos([200.0, 30.0])
        .resizable(false)
        .default_open(true)
        .show(ctx, |ui| {
            if let Some(label) = &placing_label {
                ui.colored_label(
                    egui::Color32::YELLOW,
                    format!("Placing: {label} (ESC to cancel)"),
                );
                ui.add_space(2.0);
            }

            let mut selected_depth = current_depth;
            let active_template = world
                .get_resource::<PlacementMode>()
                .and_then(|pm| pm.active.as_ref().map(|s| s.template.clone()));
            let current_animation_tag = world
                .get_resource::<PlacementMode>()
                .and_then(|pm| pm.active.as_ref().and_then(|s| s.animation_tag.clone()));

            ui.horizontal(|ui| {
                // Left column: depth radio buttons (always enabled)
                ui.vertical(|ui| {
                    field_label(ui, "Depth");
                    for &depth in EDITOR_DEPTHS {
                        let has_native = active_template
                            .as_ref()
                            .is_some_and(|t| t.has_sprite_at_depth(depth));
                        let label = if has_native {
                            format!("{}", depth.to_i8())
                        } else {
                            format!("{}*", depth.to_i8())
                        };
                        if ui.radio_value(&mut selected_depth, depth, label).changed() {
                            set_depth = Some(selected_depth);
                        }
                    }
                });

                ui.separator();

                // Middle column: spawn type lists
                ui.vertical(|ui| {
                    palette_section(ui, "Objects", &objects, &mut set_placement);
                    palette_section(ui, "Destructibles", &destructibles, &mut set_placement);
                    palette_section(ui, "Pickups", &pickups, &mut set_placement);
                    palette_section(ui, "Enemies", &enemies, &mut set_placement);
                });

                // Pose dropdown (only for composed enemies with animation tags)
                if let Some(ref template) = active_template
                    && let Some(tags) = template.available_animation_tags()
                {
                    ui.separator();
                    ui.vertical(|ui| {
                        field_label(ui, "Pose");
                        let selected_tag = current_animation_tag.clone().unwrap_or_else(|| {
                            template
                                .default_animation_tag()
                                .unwrap_or("idle")
                                .to_string()
                        });
                        egui::ComboBox::from_id_salt("pose_selector")
                            .selected_text(&selected_tag)
                            .show_ui(ui, |ui| {
                                for &tag in tags {
                                    if ui.selectable_label(selected_tag == tag, tag).clicked() {
                                        set_animation_tag = Some(Some(tag.to_string()));
                                    }
                                }
                            });
                    });
                }
            });
        });

    if let Some(template) = set_placement
        && let Some(mut pm) = world.get_resource_mut::<PlacementMode>()
    {
        // Keep current depth if already placing, otherwise use the template's default.
        let depth = if pm.active.is_some() {
            current_depth
        } else {
            template.default_depth()
        };
        let animation_tag = template
            .default_animation_tag()
            .map(std::string::ToString::to_string);
        pm.active = Some(PlacementState {
            template,
            depth,
            animation_tag,
        });
    }
    if let Some(depth) = set_depth
        && let Some(mut pm) = world.get_resource_mut::<PlacementMode>()
        && let Some(state) = pm.active.as_mut()
    {
        state.depth = depth;
    }
    if let Some(tag) = set_animation_tag
        && let Some(mut pm) = world.get_resource_mut::<PlacementMode>()
        && let Some(state) = pm.active.as_mut()
    {
        state.animation_tag = tag;
    }
}

const PALETTE_BUTTON_WIDTH: f32 = 110.0;

fn palette_section(
    ui: &mut egui::Ui,
    heading: &str,
    templates: &[SpawnTemplate],
    set_placement: &mut Option<SpawnTemplate>,
) {
    ui.collapsing(heading, |ui| {
        for template in templates {
            if ui
                .add_sized(
                    [PALETTE_BUTTON_WIDTH, 0.0],
                    egui::Button::new(template.label()),
                )
                .clicked()
            {
                *set_placement = Some(template.clone());
            }
        }
    });
}
