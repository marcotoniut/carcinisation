//! Gallery systems: startup, egui panel, character spawning, and animation override.

use crate::{
    data::AnimationData,
    gallery::{
        components::{GalleryAnimationOverride, GalleryDisplayCharacter, GalleryEntity},
        messages::GalleryStartupEvent,
        resources::{GalleryCharacter, GalleryState, all_depths},
    },
    game::GameProgressState,
    globals::{HUD_HEIGHT, SCREEN_RESOLUTION},
    layer::Layer,
    pixel::{CxAssets, CxFilterRectBundle},
    stage::{
        components::placement::{Depth, InView},
        enemy::{
            bundles::make_enemy_animation_bundle,
            composed::{ComposedAnimationState, ComposedEnemyVisual},
            data::{
                mosquito::MOSQUITO_ANIMATIONS,
                mosquiton::{
                    GALLERY_ACTION_TAGS as MOSQUITON_GALLERY_ACTION_TAGS,
                    apply_mosquiton_animation_state,
                },
                spidey::{
                    GALLERY_ACTION_TAGS as SPIDEY_GALLERY_ACTION_TAGS, apply_spidey_animation_state,
                },
                tardigrade::TARDIGRADE_ANIMATIONS,
            },
            entity::EnemyType,
        },
        player::messages::PlayerStartupEvent,
        ui::hud::{HudPlugin, spawn::spawn_hud},
    },
};
use activable::activate;
use bevy::prelude::*;
use bevy_inspector_egui::bevy_egui::{EguiContext, PrimaryEguiContext, egui};
use carapace::prelude::{
    CxAnchor, CxFilter, CxFilterLayers, CxFilterRect, CxRenderSpace, CxSprite, CxTypeface, WorldPos,
};
use carapace::{
    animation::CxAnimation,
    frame::{CxFrameControl, CxFrameCount, CxFrameSelector},
};
use carcinisation_core::components::{DespawnMark, GBColor};

use super::GalleryPlugin;

/// Center of the playable area (above the HUD bar).
const GALLERY_DISPLAY_POSITION: Vec2 = Vec2::new(
    SCREEN_RESOLUTION.x as f32 / 2.0,
    HUD_HEIGHT as f32 + (SCREEN_RESOLUTION.y as f32 - HUD_HEIGHT as f32) / 2.0,
);

/// Resolves animation data for a given character, animation name, and depth.
fn resolve_animation_data(
    character: GalleryCharacter,
    animation_name: &str,
    depth: Depth,
) -> Option<&'static AnimationData> {
    match character {
        GalleryCharacter::Mosquito => {
            let anims = &*MOSQUITO_ANIMATIONS;
            match animation_name {
                "idle" => &anims.idle,
                "melee_attack" => &anims.melee_attack,
                "death" => &anims.death,
                _ => &anims.fly,
            }
        }
        GalleryCharacter::Tardigrade => {
            let anims = &*TARDIGRADE_ANIMATIONS;
            match animation_name {
                "attack" => &anims.attack,
                "sucking" => &anims.sucking,
                "death" => &anims.death,
                _ => &anims.idle,
            }
        }
        _ => return None,
    }
    .get(&depth)
}

/// Observer: bootstraps the gallery scene.
pub fn on_gallery_startup(
    _trigger: On<GalleryStartupEvent>,
    mut commands: Commands,
    mut next_game_state: ResMut<NextState<GameProgressState>>,
    mut typefaces: CxAssets<CxTypeface>,
    mut assets_sprite: CxAssets<CxSprite>,
    mut filters: CxAssets<CxFilter>,
) {
    activate::<GalleryPlugin>(&mut commands);
    activate::<HudPlugin>(&mut commands);
    next_game_state.set(GameProgressState::Running);
    commands.trigger(PlayerStartupEvent);

    // Spawn a GB-white background covering the playable area above the HUD.
    commands.spawn((
        GalleryEntity,
        CxFilterRectBundle::<Layer> {
            anchor: CxAnchor::BottomLeft,
            canvas: CxRenderSpace::Camera,
            filter: CxFilter(filters.load_color(GBColor::White)),
            layers: CxFilterLayers::single_over(Layer::Skybox),
            position: IVec2::new(0, HUD_HEIGHT as i32).into(),
            rect: CxFilterRect(UVec2::new(
                SCREEN_RESOLUTION.x,
                SCREEN_RESOLUTION.y - HUD_HEIGHT,
            )),
            visibility: Visibility::Visible,
        },
        Name::new("GalleryBackground"),
    ));

    spawn_hud(
        &mut commands,
        &mut typefaces,
        &mut assets_sprite,
        &mut filters,
    );
}

/// Egui side panel for character and animation selection.
#[allow(clippy::too_many_lines)]
pub fn gallery_panel_ui(world: &mut World) {
    let Ok(egui_context) = world
        .query_filtered::<&mut EguiContext, With<PrimaryEguiContext>>()
        .single(world)
    else {
        return;
    };

    let mut egui_context = egui_context.clone();
    let ctx = egui_context.get_mut();

    let state = world.resource::<GalleryState>();
    let mut selected_character = state.selected_character;
    let mut selected_animation = state.selected_animation.clone();
    let mut selected_depth = state.selected_depth;
    let mut paused = state.paused;
    let mut selected_frame = state.selected_frame;
    let frame_count = state.frame_count;

    let available_depths = selected_character.available_depths();

    egui::SidePanel::right("gallery_panel")
        .default_width(200.0)
        .resizable(false)
        .show(ctx, |ui| {
            ui.heading("Gallery");
            ui.separator();

            ui.label("Character:");
            egui::ComboBox::from_id_salt("character_select")
                .selected_text(selected_character.display_name())
                .show_ui(ui, |ui| {
                    for &character in GalleryCharacter::all() {
                        ui.selectable_value(
                            &mut selected_character,
                            character,
                            character.display_name(),
                        );
                    }
                });

            ui.separator();

            ui.label("Depth:");
            let all = all_depths();
            for depth in &all {
                let has_assets = available_depths.contains(depth);
                let label = if has_assets {
                    format!("{}", depth.to_i8())
                } else {
                    format!("{} (N/A)", depth.to_i8())
                };
                let response = ui.add_enabled(
                    has_assets,
                    egui::RadioButton::new(selected_depth == *depth, label),
                );
                if response.clicked() {
                    selected_depth = *depth;
                }
            }

            ui.separator();

            let available = selected_character.available_animations();
            if available.is_empty() {
                ui.label("(no animations)");
            } else {
                ui.label("Animation:");
                for anim_name in &available {
                    ui.radio_value(
                        &mut selected_animation,
                        anim_name.clone(),
                        anim_name.as_str(),
                    );
                }
            }

            if matches!(
                selected_character,
                GalleryCharacter::Mosquiton | GalleryCharacter::Spidey
            ) {
                let action_tags: &[&str] = match selected_character {
                    GalleryCharacter::Mosquiton => MOSQUITON_GALLERY_ACTION_TAGS,
                    GalleryCharacter::Spidey => SPIDEY_GALLERY_ACTION_TAGS,
                    _ => &[],
                };
                ui.separator();
                ui.label("Verification actions:");
                ui.horizontal_wrapped(|ui| {
                    for action in action_tags {
                        if ui.button(*action).clicked() {
                            selected_animation.clear();
                            selected_animation.push_str(action);
                            paused = false;
                            selected_frame = 0;
                        }
                    }
                });
            }

            if frame_count > 0 {
                ui.separator();

                ui.horizontal(|ui| {
                    if ui
                        .button(if paused { "\u{25b6}" } else { "\u{23f8}" })
                        .clicked()
                    {
                        paused = !paused;
                    }
                    ui.label(format!("Frame: {} / {}", selected_frame + 1, frame_count));
                });

                if paused {
                    let mut frame_f32 = selected_frame as f32;
                    let max = (frame_count - 1) as f32;
                    if ui
                        .add(egui::Slider::new(&mut frame_f32, 0.0..=max).step_by(1.0))
                        .changed()
                    {
                        selected_frame = frame_f32 as usize;
                    }
                }
            }
        });

    let mut state = world.resource_mut::<GalleryState>();
    state.selected_character = selected_character;
    state.selected_animation = selected_animation;
    state.selected_depth = selected_depth;
    state.paused = paused;
    state.selected_frame = selected_frame;
}

/// Detects selection changes and respawns the display character.
pub fn react_to_gallery_selection_changed(
    mut commands: Commands,
    mut state: ResMut<GalleryState>,
    display_query: Query<Entity, With<GalleryDisplayCharacter>>,
    asset_server: Res<AssetServer>,
) {
    let char_changed = state.prev_character != Some(state.selected_character);
    let anim_changed = state.prev_animation.as_deref() != Some(&state.selected_animation);
    let depth_changed = state.prev_depth != Some(state.selected_depth);

    if !char_changed && !anim_changed && !depth_changed {
        return;
    }

    for entity in &display_query {
        commands.entity(entity).insert(DespawnMark);
    }

    let character = state.selected_character;
    let animation = state.selected_animation.clone();

    // Reset animation to first available when character changed and current anim is invalid.
    let available = character.available_animations();
    let animation = if !available.is_empty() && !available.contains(&animation) {
        available[0].clone()
    } else {
        animation
    };
    state.selected_animation.clone_from(&animation);

    // Reset depth to default when character changed and current depth has no assets.
    let available_depths = character.available_depths();
    let depth = if char_changed && !available_depths.contains(&state.selected_depth) {
        let d = character.default_depth();
        state.selected_depth = d;
        d
    } else {
        state.selected_depth
    };

    state.prev_character = Some(character);
    state.prev_animation = Some(animation.clone());
    state.prev_depth = Some(depth);
    state.paused = false;
    state.selected_frame = 0;
    state.frame_count = 0;

    if !character.has_assets() || available.is_empty() {
        return;
    }

    let common = (
        GalleryEntity,
        GalleryDisplayCharacter,
        WorldPos::from(GALLERY_DISPLAY_POSITION),
        depth,
        InView,
    );

    match character {
        GalleryCharacter::Mosquito | GalleryCharacter::Tardigrade => {
            commands.spawn((
                common,
                GalleryAnimationOverride {
                    animation_name: animation,
                },
                Name::new(format!("Gallery<{}>", character.display_name())),
            ));
        }
        GalleryCharacter::Mosquiton => {
            // Gallery preview path only. It is an asset-preview authority model,
            // not the gameplay spawn pipeline, and must not be used as a
            // reference for spawn-time presentation priming guarantees.
            commands.spawn((
                common,
                ComposedAnimationState::new(&animation),
                ComposedEnemyVisual::for_enemy(&asset_server, EnemyType::Mosquiton, depth),
                Name::new("Gallery<Mosquiton>"),
            ));
        }
        GalleryCharacter::Spidey => {
            // Gallery preview path only. It is an asset-preview authority model,
            // not the gameplay spawn pipeline, and must not be used as a
            // reference for spawn-time presentation priming guarantees.
            commands.spawn((
                common,
                ComposedAnimationState::new(&animation),
                ComposedEnemyVisual::for_enemy(&asset_server, EnemyType::Spidey, depth),
                Name::new("Gallery<Spidey>"),
            ));
        }
        GalleryCharacter::Marauder | GalleryCharacter::Spidomonsta | GalleryCharacter::Kyle => {}
    }
}

/// Assigns `CxSprite` animation bundles to gallery entities with `GalleryAnimationOverride`.
pub fn apply_gallery_animation(
    mut commands: Commands,
    state: Res<GalleryState>,
    query: Query<(Entity, &GalleryAnimationOverride, &Depth), Added<GalleryAnimationOverride>>,
    mut assets_sprite: CxAssets<CxSprite>,
) {
    for (entity, override_comp, depth) in &query {
        let Some(data) = resolve_animation_data(
            state.selected_character,
            &override_comp.animation_name,
            *depth,
        ) else {
            continue;
        };

        let (sprite_bundle, animation_bundle) =
            make_enemy_animation_bundle(&mut assets_sprite, data, depth);
        commands.entity(entity).insert((
            WorldPos::from(GALLERY_DISPLAY_POSITION),
            sprite_bundle,
            animation_bundle,
        ));
    }
}

/// Controls animation playback: pauses by removing `CxAnimation`, resumes by re-inserting it,
/// and sets the frame index when paused.
pub fn apply_gallery_playback_control(
    mut commands: Commands,
    mut state: ResMut<GalleryState>,
    mut query: Query<
        (
            Entity,
            Option<&CxAnimation>,
            &mut CxFrameControl,
            &CxFrameCount,
        ),
        With<GalleryDisplayCharacter>,
    >,
) {
    for (entity, animation, mut frame_control, frame_count) in &mut query {
        state.frame_count = frame_count.0;

        if state.paused {
            // Remove CxAnimation so update_animations doesn't overwrite our frame.
            if animation.is_some() {
                commands.entity(entity).remove::<CxAnimation>();
            }
            state.selected_frame = state.selected_frame.min(frame_count.0.saturating_sub(1));
            frame_control.selector = CxFrameSelector::Index(state.selected_frame as f32);
        } else if animation.is_none() {
            // Re-insert CxAnimation to resume playback.
            commands.entity(entity).insert(CxAnimation::default());
        }

        // Track current frame for the UI.
        if !state.paused {
            if let CxFrameSelector::Index(idx) = frame_control.selector {
                state.selected_frame = idx as usize;
            } else if let CxFrameSelector::Normalized(n) = frame_control.selector {
                state.selected_frame =
                    (n * frame_count.0.saturating_sub(1) as f32).round() as usize;
            }
        }
    }
}

/// Updates the composed animation tag when the gallery selection changes for composed characters.
pub fn update_gallery_composed_animation(
    state: Res<GalleryState>,
    mut query: Query<&mut ComposedAnimationState, With<GalleryDisplayCharacter>>,
) {
    match state.selected_character {
        GalleryCharacter::Mosquiton => {
            for mut anim_state in &mut query {
                apply_mosquiton_animation_state(&mut anim_state, &state.selected_animation);
            }
        }
        GalleryCharacter::Spidey => {
            for mut anim_state in &mut query {
                apply_spidey_animation_state(&mut anim_state, &state.selected_animation);
            }
        }
        _ => {}
    }
}

/// Cleans up all gallery entities when the plugin is deactivated.
pub fn cleanup_gallery(mut commands: Commands, query: Query<Entity, With<GalleryEntity>>) {
    for entity in &query {
        commands.entity(entity).insert(DespawnMark);
    }
}
