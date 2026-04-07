use bevy::ecs::world::CommandQueue;
use bevy::prelude::*;
use bevy_inspector_egui::bevy_egui::{EguiContext, PrimaryEguiContext};
use bevy_inspector_egui::egui::{self, Align2};
use bevy_inspector_egui::reflect_inspector::{Context, InspectorUi};
use bevy_inspector_egui::restricted_world_view::RestrictedWorldView;

use crate::components::{SceneData, ScenePath, SelectedItem};
use crate::file_manager::actions::{request_file_picker, save_scene};
use crate::resources::SceneInspectorLayout;
use crate::ui::style::{apply_editor_style, field_label, section_header};

const SCENE_PANEL_WIDTH: f32 = 360.0;
const SCENE_PANEL_GAP: f32 = 8.0;
const SCENE_SELECTION_MIN_HEIGHT: f32 = 120.0;
const SCENE_MIN_TOP_HEIGHT: f32 = 140.0;

/// @system Renders inspector windows (world, scene, and file path controls).
#[allow(clippy::too_many_lines)]
pub fn inspector_ui(world: &mut World) {
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

    let scene_path = world.resource::<ScenePath>().0.clone();
    let scene_data = world.get_resource::<SceneData>().cloned();
    let selected_entity = world
        .query_filtered::<Entity, With<SelectedItem>>()
        .iter(world)
        .next();
    let has_stage = matches!(scene_data, Some(SceneData::Stage(_)));
    let mut selection_height = world.resource::<SceneInspectorLayout>().selection_height;
    let has_selection = selected_entity.is_some();

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

    egui::SidePanel::right("scene_panel")
        .resizable(true)
        .default_width(SCENE_PANEL_WIDTH)
        .min_width(280.0)
        .max_width(640.0)
        .frame(egui::Frame::side_top_panel(ctx.style().as_ref()).inner_margin(10))
        .show_separator_line(false)
        .show(ctx, |ui| {
            let available_height = ui.available_height().max(SCENE_MIN_TOP_HEIGHT);
            let max_selection_height = (available_height - SCENE_MIN_TOP_HEIGHT - SCENE_PANEL_GAP)
                .max(SCENE_SELECTION_MIN_HEIGHT);
            selection_height =
                selection_height.clamp(SCENE_SELECTION_MIN_HEIGHT, max_selection_height);
            let scene_section_height = if has_selection {
                (available_height - selection_height - SCENE_PANEL_GAP).max(SCENE_MIN_TOP_HEIGHT)
            } else {
                available_height
            };

            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), scene_section_height),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.set_min_height(scene_section_height.max(0.0));
                    ui.set_width(ui.available_width());
                    ui.strong("Scene");
                    ui.separator();

                    egui::ScrollArea::vertical()
                        .id_salt("scene_scroll")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.set_min_width(ui.available_width());
                            match scene_data.as_ref() {
                                Some(SceneData::Stage(_)) => {
                                    stage_inspector(world, ui);
                                }
                                Some(SceneData::Cutscene(_)) => {
                                    bevy_inspector_egui::bevy_inspector::ui_for_resource::<
                                        SceneData,
                                    >(world, ui);
                                }
                                None => {
                                    ui.label("No scene loaded");
                                }
                            }
                            ui.add_space(16.0);
                        });
                },
            );

            if let Some(entity) = selected_entity {
                ui.add_space(SCENE_PANEL_GAP);
                let (rect, response) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), 6.0),
                    egui::Sense::click_and_drag(),
                );
                if response.hovered() || response.dragged() {
                    ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::ResizeVertical);
                }
                ui.painter().line_segment(
                    [rect.left_center(), rect.right_center()],
                    egui::Stroke::new(2.0, ui.visuals().widgets.noninteractive.bg_stroke.color),
                );
                if response.dragged() {
                    let delta_y = ui.ctx().input(|i| i.pointer.delta().y);
                    selection_height = (selection_height - delta_y)
                        .clamp(SCENE_SELECTION_MIN_HEIGHT, max_selection_height);
                }
                ui.add_space(2.0);

                ui.allocate_ui_with_layout(
                    egui::vec2(ui.available_width(), selection_height),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.set_min_height(selection_height.max(0.0));
                        ui.set_width(ui.available_width());
                        ui.strong("Selection");
                        ui.separator();

                        egui::ScrollArea::vertical()
                            .id_salt("selection_scroll")
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                ui.set_min_width(ui.available_width());
                                bevy_inspector_egui::bevy_inspector::ui_for_entity(
                                    world, entity, ui,
                                );
                                ui.add_space(16.0);
                            });
                    },
                );
            }
        });

    {
        let mut layout = world.resource_mut::<SceneInspectorLayout>();
        layout.selection_height = selection_height;
    }

    egui::Window::new("Path")
        .title_bar(false)
        .anchor(egui::Align2::LEFT_TOP, [0.0, 0.0])
        .collapsible(false)
        .resizable(false)
        .movable(false)
        .show(ctx, |ui| {
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
                    && let Some(scene_data) = scene_data.as_ref()
                {
                    save_scene(world, &scene_path, scene_data);
                }

                if ui
                    .add(egui::Button::new(select_text).min_size(egui::vec2(select_width, 0.0)))
                    .clicked()
                {
                    request_file_picker(world);
                }
            });
        });

    // Close confirmation dialog
    let mut confirm_open = world.resource::<crate::resources::CloseConfirmation>().0;
    if confirm_open {
        let mut should_exit = false;
        egui::Window::new("Unsaved changes")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label("Leaving without immortalising?");
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Quit anyway").clicked() {
                        should_exit = true;
                    }
                    if ui.button("Cancel").clicked() {
                        confirm_open = false;
                    }
                });
            });
        world
            .resource_mut::<crate::resources::CloseConfirmation>()
            .0 = confirm_open;
        if should_exit {
            world
                .resource_mut::<crate::resources::CloseConfirmation>()
                .0 = false;
            world.resource_mut::<crate::resources::ShouldExit>().0 = true;
        }
    }
}

/// Renders a hybrid inspector for StageData: manual top-level layout with
/// reflection-driven editors for each field's value.
///
/// Uses `RestrictedWorldView` to split off SceneData so that individual fields
/// can be reflected without triggering spurious change detection.
fn stage_inspector(world: &mut World, ui: &mut egui::Ui) {
    let type_registry = world.resource::<AppTypeRegistry>().0.clone();
    let type_registry = type_registry.read();

    let Some((mut resource, world_view)) =
        RestrictedWorldView::new(world).split_off_resource_typed::<SceneData>()
    else {
        return;
    };

    let SceneData::Stage(ref mut stage) = *resource.bypass_change_detection() else {
        return;
    };

    let mut queue = CommandQueue::default();
    let mut cx = Context {
        world: Some(world_view),
        queue: Some(&mut queue),
    };
    let mut env = InspectorUi::for_bevy(&type_registry, &mut cx);
    let mut changed = false;

    section_header(ui, "Stage");

    changed |= reflected_field(&mut env, ui, "name", &mut stage.name);
    changed |= reflected_field(&mut env, ui, "background_path", &mut stage.background_path);
    changed |= reflected_field(&mut env, ui, "music_path", &mut stage.music_path);

    ui.add_space(4.0);
    changed |= reflected_collapsing(&mut env, ui, "skybox", &mut stage.skybox);

    changed |= reflected_field_narrow(
        &mut env,
        ui,
        "start_coordinates",
        140.0,
        &mut stage.start_coordinates,
    );
    changed |= reflected_field_narrow(&mut env, ui, "gravity", 80.0, &mut stage.gravity);

    ui.add_space(4.0);
    changed |= reflected_field_narrow(
        &mut env,
        ui,
        "on_start_transition",
        140.0,
        &mut stage.on_start_transition_o,
    );
    changed |= reflected_field_narrow(
        &mut env,
        ui,
        "on_end_transition",
        140.0,
        &mut stage.on_end_transition_o,
    );

    ui.add_space(6.0);
    section_header(ui, &format!("Spawns ({})", stage.spawns.len()));
    changed |= spawn_list(&mut env, ui, &mut stage.spawns);

    ui.add_space(6.0);
    section_header(ui, &format!("Steps ({})", stage.steps.len()));
    changed |= step_list(&mut env, ui, &mut stage.steps, stage.start_coordinates);

    if changed {
        resource.set_changed();
    }

    queue.apply(world);
}

/// Renders a label above a reflection-driven field editor. Returns true if the value changed.
fn reflected_field(
    env: &mut InspectorUi,
    ui: &mut egui::Ui,
    label: &str,
    value: &mut dyn Reflect,
) -> bool {
    field_label(ui, label);
    let changed = ui
        .push_id(label, |ui| {
            env.ui_for_reflect(value.as_partial_reflect_mut(), ui)
        })
        .inner;
    ui.add_space(2.0);
    changed
}

/// Like `reflected_field`, but constrains the editor widget to a maximum width.
fn reflected_field_narrow(
    env: &mut InspectorUi,
    ui: &mut egui::Ui,
    label: &str,
    max_width: f32,
    value: &mut dyn Reflect,
) -> bool {
    field_label(ui, label);
    let changed = ui
        .push_id(label, |ui| {
            ui.set_max_width(max_width);
            env.ui_for_reflect(value.as_partial_reflect_mut(), ui)
        })
        .inner;
    ui.add_space(2.0);
    changed
}

/// Renders the spawns list with per-spawn hybrid layout (label above each field).
fn spawn_list(
    env: &mut InspectorUi,
    ui: &mut egui::Ui,
    spawns: &mut Vec<carcinisation::stage::data::StageSpawn>,
) -> bool {
    use carcinisation::stage::data::{ObjectSpawn, ObjectType, StageSpawn};

    let mut changed = false;
    let mut action: Option<ListAction> = None;
    let len = spawns.len();

    for (i, spawn) in spawns.iter_mut().enumerate() {
        let label = match spawn {
            StageSpawn::Object(s) => format!("[{}] Object ({:?})", i, s.object_type),
            StageSpawn::Destructible(s) => {
                format!("[{}] Destructible ({:?})", i, s.destructible_type)
            }
            StageSpawn::Pickup(s) => format!("[{}] Pickup ({:?})", i, s.pickup_type),
            StageSpawn::Enemy(s) => format!("[{}] Enemy ({:?})", i, s.enemy_type),
        };

        let id = ui.make_persistent_id(format!("spawn_{i}"));
        egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, true)
            .show_header(ui, |ui| {
                list_item_controls(ui, i, len, &mut action);
                ui.label(&label);
            })
            .body(|ui| {
                ui.push_id(i, |ui| {
                    changed |= spawn_fields(env, ui, spawn);
                });
            });
    }

    if ui.small_button("+ add spawn").clicked() {
        spawns.push(StageSpawn::Object(ObjectSpawn {
            object_type: ObjectType::BenchBig,
            coordinates: bevy::math::Vec2::ZERO,
            depth: carcinisation::stage::components::placement::Depth::Three,
            authored_depths: None,
        }));
        changed = true;
    }

    if let Some(act) = action {
        apply_list_action(spawns, act);
        changed = true;
    }

    changed
}

/// Renders a single spawn's fields with labels above each input.
fn spawn_fields(
    env: &mut InspectorUi,
    ui: &mut egui::Ui,
    spawn: &mut carcinisation::stage::data::StageSpawn,
) -> bool {
    use carcinisation::stage::data::StageSpawn;

    let mut changed = false;
    match spawn {
        StageSpawn::Object(s) => {
            changed |= reflected_field(env, ui, "object_type", &mut s.object_type);
            changed |= reflected_field_narrow(env, ui, "coordinates", 140.0, &mut s.coordinates);
            changed |= reflected_field(env, ui, "depth", &mut s.depth);
            changed |= reflected_collapsing(env, ui, "authored_depths", &mut s.authored_depths);
        }
        StageSpawn::Destructible(s) => {
            changed |= reflected_field(env, ui, "destructible_type", &mut s.destructible_type);
            changed |= reflected_field_narrow(env, ui, "coordinates", 140.0, &mut s.coordinates);
            changed |= reflected_field(env, ui, "depth", &mut s.depth);
            changed |= reflected_field_narrow(env, ui, "health", 80.0, &mut s.health);
            changed |= reflected_collapsing(env, ui, "authored_depths", &mut s.authored_depths);
        }
        StageSpawn::Pickup(s) => {
            changed |= reflected_field(env, ui, "pickup_type", &mut s.pickup_type);
            changed |= reflected_field_narrow(env, ui, "coordinates", 140.0, &mut s.coordinates);
            changed |= reflected_field(env, ui, "depth", &mut s.depth);
            changed |= reflected_collapsing(env, ui, "authored_depths", &mut s.authored_depths);
        }
        StageSpawn::Enemy(s) => {
            changed |= reflected_field(env, ui, "enemy_type", &mut s.enemy_type);
            changed |= reflected_field_narrow(env, ui, "coordinates", 140.0, &mut s.coordinates);
            changed |= reflected_field(env, ui, "depth", &mut s.depth);
            changed |= reflected_field_narrow(env, ui, "speed", 80.0, &mut s.speed);
            changed |= reflected_field_narrow(env, ui, "health", 80.0, &mut s.health);
            changed |= reflected_field_narrow(env, ui, "elapsed", 100.0, &mut s.elapsed);
            changed |= reflected_collapsing(env, ui, "authored_depths", &mut s.authored_depths);
            changed |= reflected_collapsing(env, ui, "steps", &mut s.steps);
        }
    }
    changed
}

/// Renders the steps list with per-step hybrid layout.
fn step_list(
    env: &mut InspectorUi,
    ui: &mut egui::Ui,
    steps: &mut Vec<carcinisation::stage::data::StageStep>,
    start_coordinates: bevy::math::Vec2,
) -> bool {
    use carcinisation::stage::components::{StopStageStep, TweenStageStep};
    use carcinisation::stage::data::StageStep;

    let mut changed = false;
    let mut action: Option<ListAction> = None;
    let len = steps.len();

    for (i, step) in steps.iter_mut().enumerate() {
        let label = match step {
            StageStep::Stop(_) => format!("[{i}] Stop"),
            StageStep::Tween(_) => format!("[{i}] Tween"),
            StageStep::Cinematic(_) => format!("[{i}] Cinematic"),
        };

        let id = ui.make_persistent_id(format!("step_{i}"));
        egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, true)
            .show_header(ui, |ui| {
                list_item_controls(ui, i, len, &mut action);
                ui.label(&label);
            })
            .body(|ui| {
                ui.push_id(i, |ui| {
                    changed |= step_fields(env, ui, step);
                });
            });
    }

    ui.horizontal(|ui| {
        if ui.small_button("+ add stop").clicked() {
            steps.push(StageStep::Stop(StopStageStep::new()));
            changed = true;
        }
        if ui.small_button("+ add tween").clicked() {
            let endpoint = last_step_endpoint(steps, start_coordinates);
            let mut tween = TweenStageStep::new();
            tween.coordinates = endpoint + bevy::math::Vec2::new(80.0, 0.0);
            steps.push(StageStep::Tween(tween));
            changed = true;
        }
    });

    if let Some(act) = action {
        apply_list_action(steps, act);
        changed = true;
    }

    changed
}

/// Returns the world position of the last tween endpoint, or `start` if no tweens exist.
fn last_step_endpoint(
    steps: &[carcinisation::stage::data::StageStep],
    start: bevy::math::Vec2,
) -> bevy::math::Vec2 {
    use carcinisation::stage::data::StageStep;
    for step in steps.iter().rev() {
        if let StageStep::Tween(t) = step {
            return t.coordinates;
        }
    }
    start
}

/// Renders a single step's fields with labels above each input.
fn step_fields(
    env: &mut InspectorUi,
    ui: &mut egui::Ui,
    step: &mut carcinisation::stage::data::StageStep,
) -> bool {
    use carcinisation::stage::data::StageStep;

    let mut changed = false;
    match step {
        StageStep::Stop(s) => {
            changed |= reflected_field_narrow(env, ui, "max_duration", 120.0, &mut s.max_duration);
            changed |= reflected_field(env, ui, "kill_all", &mut s.kill_all);
            changed |= reflected_field(env, ui, "kill_boss", &mut s.kill_boss);
            if s.spawns.is_empty() {
                field_label(ui, "spawns");
                ui.label("(empty)");
            } else {
                ui.add_space(4.0);
                field_label(ui, &format!("spawns ({})", s.spawns.len()));
                changed |= spawn_list(env, ui, &mut s.spawns);
            }
            changed |= reflected_collapsing(env, ui, "floor_depths", &mut s.floor_depths);
        }
        StageStep::Tween(s) => {
            changed |= reflected_field_narrow(env, ui, "coordinates", 140.0, &mut s.coordinates);
            changed |= reflected_field_narrow(env, ui, "base_speed", 80.0, &mut s.base_speed);
            if s.spawns.is_empty() {
                field_label(ui, "spawns");
                ui.label("(empty)");
            } else {
                ui.add_space(4.0);
                field_label(ui, &format!("spawns ({})", s.spawns.len()));
                changed |= spawn_list(env, ui, &mut s.spawns);
            }
            changed |= reflected_collapsing(env, ui, "floor_depths", &mut s.floor_depths);
        }
        StageStep::Cinematic(s) => {
            changed |= reflected_field(env, ui, "cinematic", s);
        }
    }
    changed
}

/// Renders a collapsing section with a label, containing a reflection-driven editor.
fn reflected_collapsing(
    env: &mut InspectorUi,
    ui: &mut egui::Ui,
    label: &str,
    value: &mut dyn Reflect,
) -> bool {
    let mut changed = false;
    egui::CollapsingHeader::new(
        egui::RichText::new(label)
            .font(egui::FontId::proportional(11.0))
            .color(crate::ui::style::LABEL_COLOR),
    )
    .default_open(false)
    .id_salt(label)
    .show(ui, |ui| {
        ui.push_id(label, |ui| {
            changed = env.ui_for_reflect(value.as_partial_reflect_mut(), ui);
        });
    });
    changed
}

// ─── List helpers ────────────────────────────────────────────────────────────

enum ListAction {
    MoveUp(usize),
    MoveDown(usize),
    Remove(usize),
}

/// Renders move-up, move-down, and delete buttons for a list item header.
fn list_item_controls(
    ui: &mut egui::Ui,
    index: usize,
    len: usize,
    action: &mut Option<ListAction>,
) {
    let up_enabled = index > 0;
    let down_enabled = index + 1 < len;

    if ui
        .add_enabled(up_enabled, egui::Button::new("^").small())
        .clicked()
    {
        *action = Some(ListAction::MoveUp(index));
    }
    if ui
        .add_enabled(down_enabled, egui::Button::new("v").small())
        .clicked()
    {
        *action = Some(ListAction::MoveDown(index));
    }
    if ui.small_button("x").clicked() {
        *action = Some(ListAction::Remove(index));
    }
}

/// Applies a list action (move/remove) to a `Vec`.
fn apply_list_action<T>(vec: &mut Vec<T>, action: ListAction) {
    match action {
        ListAction::MoveUp(i) if i > 0 => {
            vec.swap(i, i - 1);
        }
        ListAction::MoveDown(i) if i + 1 < vec.len() => {
            vec.swap(i, i + 1);
        }
        ListAction::Remove(i) => {
            vec.remove(i);
        }
        _ => {}
    }
}
