//! Gizmo drawing helpers for visualising stage data during development.

use super::{
    DebugColor, DebugComposedDamageProbe, DebugComposedDamageProbeRequest,
    DebugComposedDamageProbeResult, DebugGodMode,
};
use crate::{
    stage::{
        attack::components::EnemyAttack,
        collision::{CollisionTarget, MaskCollisionAssets, visit_target_debug_collider},
        components::{interactive::ColliderData, placement::Depth},
        destructible::components::Destructible,
        enemy::{
            components::Enemy,
            composed::{
                ComposedAtlasBindings, ComposedCollisionState, ComposedHealthPools,
                ComposedResolvedParts,
            },
            mosquiton::entity::EnemyMosquiton,
        },
        floors::{ActiveFloors, Surface},
        messages::PartDamageMessage,
        player::components::PlayerAttack,
    },
    systems::camera::CameraPos,
};
use bevy::{prelude::*, window::PrimaryWindow};
use carapace::prelude::*;
use carcinisation_collision::{
    ColliderShape, WorldMaskInstance, extract_mask_boundary, extract_mask_boundary_closed,
    mask_edge_to_world_points,
};

pub const LINE_EXTENSION: f32 = 1000.;

const DEBUG_HEAD_DAMAGE: u32 = 3;
const DEBUG_BODY_DAMAGE: u32 = 5;
const DEBUG_ARM_DAMAGE: u32 = 4;
const MASK_BASE_HUE_DEG: f32 = 132.0;
const BOX_BASE_HUE_DEG: f32 = 190.0;
const RADIAL_BASE_HUE_DEG: f32 = 62.0;
const BASE_SATURATION: f32 = 0.9;
const BASE_LIGHTNESS: f32 = 0.58;
const COLLIDER_ALPHA: f32 = 0.7;
const ENEMY_HUE_OFFSET_PERCENT: f32 = 0.0;
const PROJECTILE_HUE_OFFSET_PERCENT: f32 = 0.0;
const DESTRUCTIBLE_HUE_OFFSET_PERCENT: f32 = -0.08;
const PLAYER_SHOT_HUE_DEG: f32 = 340.0;
const PLAYER_SHOT_SATURATION: f32 = 0.85;
const PLAYER_SHOT_LIGHTNESS: f32 = 0.65;
const ENEMY_SATURATION_DELTA: f32 = 0.0;
const PROJECTILE_SATURATION_DELTA: f32 = -0.15;
const DESTRUCTIBLE_SATURATION_DELTA: f32 = -0.18;
const ENEMY_LIGHTNESS_DELTA: f32 = 0.0;
const PROJECTILE_LIGHTNESS_DELTA: f32 = 0.20;
const DESTRUCTIBLE_LIGHTNESS_DELTA: f32 = -0.10;

#[derive(Clone, Copy)]
enum DebugColliderEntityClass {
    Enemy,
    Projectile,
    PlayerShot,
    Destructible,
}

#[derive(Clone, Copy)]
enum DebugColliderVisualKind {
    Mask,
    Box,
    Radial,
}

/// @system Toggles debug god mode with `Shift+G`.
///
/// The toggle lives in the debug plugin so it stays out of production input
/// mappings. Damage is still emitted normally; only the shared damage
/// application system consults this resource.
pub fn toggle_debug_god_mode(
    keys: Res<ButtonInput<KeyCode>>,
    god_mode: Option<ResMut<DebugGodMode>>,
) {
    let Some(mut god_mode) = god_mode else {
        return;
    };

    let modifier_held = keys.any_pressed([
        KeyCode::ShiftLeft,
        KeyCode::ShiftRight,
        KeyCode::SuperLeft,
        KeyCode::SuperRight,
        KeyCode::ControlLeft,
        KeyCode::ControlRight,
    ]);
    if modifier_held && keys.just_pressed(KeyCode::KeyG) {
        god_mode.enabled = !god_mode.enabled;
        info!(
            "Debug god mode {}",
            if god_mode.enabled {
                "enabled"
            } else {
                "disabled"
            }
        );
    }
}

#[must_use]
pub fn to_overlay_delta_x(x: f32, overlay_transform: CxOverlayViewportTransform) -> f32 {
    overlay_transform.delta_x(x)
}

#[must_use]
pub fn to_overlay_delta_y(y: f32, overlay_transform: CxOverlayViewportTransform) -> f32 {
    overlay_transform.delta_y(y)
}

#[must_use]
pub fn to_overlay_delta(v: Vec2, overlay_transform: CxOverlayViewportTransform) -> Vec2 {
    overlay_transform.delta(v)
}

#[must_use]
pub fn to_overlay_point_x(x: f32, overlay_transform: CxOverlayViewportTransform) -> f32 {
    overlay_transform.point_x(x)
}

#[must_use]
pub fn to_overlay_point_y(y: f32, overlay_transform: CxOverlayViewportTransform) -> f32 {
    overlay_transform.point_y(y)
}

#[must_use]
pub fn to_overlay_point(position: Vec2, overlay_transform: CxOverlayViewportTransform) -> Vec2 {
    overlay_transform.point(position)
}

/// @system Renders the configured floor heights as horizontal gizmo lines.
pub fn draw_floor_lines(
    mut gizmos: Gizmos,
    floors: Option<Res<ActiveFloors>>,
    screen: Res<CxScreen>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    let Some(floors) = floors else {
        return;
    };
    let Ok(window) = windows.single() else {
        return;
    };
    let overlay_transform = CxOverlayViewportTransform::from_screen(
        &screen,
        Vec2::new(window.width(), window.height()),
    );

    for (depth, surfaces) in &floors.by_depth {
        if depth == &Depth::Zero {
            continue;
        }
        for surface in surfaces {
            let Surface::Solid { y } = surface else {
                continue;
            };
            let floor_y = to_overlay_point_y(*y, overlay_transform);
            gizmos.line(
                Vec3::new(-LINE_EXTENSION, floor_y, 0.),
                Vec3::new(LINE_EXTENSION, floor_y, 0.),
                Color::YELLOW_GREEN,
            );
        }
    }
}

/// Toggle collider debug overlay with Cmd+O.
pub fn toggle_collider_overlay(
    keys: Res<ButtonInput<KeyCode>>,
    mut overlay: ResMut<DebugColliderOverlay>,
) {
    let modifier_held = keys.any_pressed([
        KeyCode::ControlLeft,
        KeyCode::ControlRight,
        KeyCode::SuperLeft,
        KeyCode::SuperRight,
    ]);
    if modifier_held && keys.just_pressed(KeyCode::KeyO) {
        overlay.enabled = !overlay.enabled;
        info!(
            "Collider debug overlay {}",
            if overlay.enabled {
                "enabled"
            } else {
                "disabled"
            }
        );
    }
}

/// Controls visibility of all collision debug gizmos:
/// - `ColliderData` circles/boxes for simple entities (Mosquito, Tardigrade, etc.)
/// - Pixel mask outlines for composed enemies (Mosquiton, Spidey)
///
/// Toggle at runtime with Cmd+O. Set `CARCINISATION_SHOW_COLLIDERS=true`
/// in `.env` to start enabled.
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct DebugColliderOverlay {
    pub enabled: bool,
}

impl DebugColliderOverlay {
    #[must_use]
    pub const fn new(enabled: bool) -> Self {
        Self { enabled }
    }
}

/// @system Visualises entity-level collider shapes (`ColliderData`).
///
/// Gated by [`DebugColliderOverlay`].
#[allow(clippy::too_many_lines)]
pub(crate) fn draw_colliders(
    mut gizmos: Gizmos,
    overlay: Res<DebugColliderOverlay>,
    camera_query: Query<&WorldPos, With<CameraPos>>,
    screen: Res<CxScreen>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut collision_assets: MaskCollisionAssets<'_, '_>,
    query: Query<
        (
            &CxPosition,
            &WorldPos,
            (&CxAnchor, &CxRenderSpace),
            (
                Option<&CxFrameView>,
                Option<&CxSprite>,
                Option<&CxAtlasSprite>,
                Option<&CxPresentationTransform>,
            ),
            Option<&ColliderData>,
            Option<&ComposedCollisionState>,
            Option<&ComposedResolvedParts>,
            Option<&ComposedAtlasBindings>,
            (
                Option<&Enemy>,
                Option<&EnemyAttack>,
                Option<&Destructible>,
                Option<&PlayerAttack>,
            ),
        ),
        Or<(With<ColliderData>, With<ComposedCollisionState>)>,
    >,
) {
    if !overlay.enabled {
        return;
    }
    let camera_pos = camera_query.single().unwrap();
    let camera_world = camera_pos.0.round().as_ivec2();
    let Ok(window) = windows.single() else {
        return;
    };
    let overlay_transform = CxOverlayViewportTransform::from_screen(
        &screen,
        Vec2::new(window.width(), window.height()),
    );
    collision_assets.refresh();

    for (
        position,
        world_pos,
        (anchor, canvas),
        (frame, sprite, atlas_sprite, presentation),
        collider_data,
        composed_collision_state,
        resolved_parts,
        bindings,
        (enemy, enemy_attack, destructible, player_attack),
    ) in query.iter()
    {
        let entity_class =
            debug_collider_entity_class(enemy, enemy_attack, destructible, player_attack);

        // Camera-canvas entities are already in screen space; world-canvas
        // entities need the camera offset subtracted for viewport mapping.
        let cam_offset = match *canvas {
            CxRenderSpace::Camera => Vec2::ZERO,
            CxRenderSpace::World => camera_pos.0,
        };

        // Player attacks with geometric colliders (Point cross, Radial circle)
        // must bypass the mask-first visitor — their visual sprite is not the
        // collision shape.
        let has_geometric_player_colliders = player_attack.is_some()
            && collider_data.is_some_and(|d| {
                d.0.iter().any(|c| {
                    !matches!(
                        c.shape,
                        ColliderShape::SpriteMask | ColliderShape::SpriteMaskClosed
                    )
                })
            });

        if has_geometric_player_colliders {
            if let Some(data) = collider_data {
                for collider in &data.0 {
                    match collider.shape {
                        ColliderShape::Circle(radius) => {
                            draw_circle_collider_2d(
                                &mut gizmos,
                                world_pos.0 + collider.offset,
                                radius,
                                debug_collider_color(entity_class, DebugColliderVisualKind::Radial),
                                |point| to_overlay_point(point - cam_offset, overlay_transform),
                                |x| to_overlay_delta_x(x, overlay_transform),
                            );
                        }
                        ColliderShape::Box(size) => {
                            draw_rect_collider_2d(
                                &mut gizmos,
                                world_pos.0 + collider.offset,
                                size,
                                debug_collider_color(entity_class, DebugColliderVisualKind::Box),
                                |point| to_overlay_point(point - cam_offset, overlay_transform),
                                |value| to_overlay_delta(value, overlay_transform),
                            );
                        }
                        ColliderShape::SpriteMask | ColliderShape::SpriteMaskClosed => {}
                    }
                }
            }
        } else {
            let target = CollisionTarget {
                position,
                world_pos,
                anchor,
                canvas,
                frame,
                sprite,
                atlas_sprite,
                presentation,
                collider_data,
                composed_collision_state,
                composed_resolved_parts: resolved_parts,
                composed_atlas_bindings: bindings,
                enemy,
                destructible,
            };

            visit_target_debug_collider(
                target,
                camera_world,
                &mut collision_assets,
                |_| {},
                |origin, collider| match collider.shape {
                    ColliderShape::Circle(radius) => {
                        draw_circle_collider_2d(
                            &mut gizmos,
                            origin + collider.offset,
                            radius,
                            debug_collider_color(entity_class, DebugColliderVisualKind::Radial),
                            |point| to_overlay_point(point - cam_offset, overlay_transform),
                            |x| to_overlay_delta_x(x, overlay_transform),
                        );
                    }
                    ColliderShape::Box(size) => {
                        draw_rect_collider_2d(
                            &mut gizmos,
                            origin + collider.offset,
                            size,
                            debug_collider_color(entity_class, DebugColliderVisualKind::Box),
                            |point| to_overlay_point(point - cam_offset, overlay_transform),
                            |value| to_overlay_delta(value, overlay_transform),
                        );
                    }
                    ColliderShape::SpriteMask | ColliderShape::SpriteMaskClosed => {}
                },
            );
        }
    }
}

// draw_composed_parts removed: composed enemies use pixel-mask collision
// exclusively. The pixel mask outline (Cmd+M) shows the real collision
// shape. Analytic shapes on composed enemies are only for legacy
// compatibility and have no gameplay effect.

/// @system In debug builds, triggers deterministic Mosquiton part damage probes.
///
/// The probe resolves through the same composed collision state used by live
/// gameplay before emitting `PartDamageMessage`, so its success/failure path is
/// a runtime proof of semantic targeting rather than a separate shortcut.
#[allow(clippy::too_many_lines)]
pub fn debug_damage_composed_parts(
    keys: Res<ButtonInput<KeyCode>>,
    mut debug_probe: ResMut<DebugComposedDamageProbe>,
    mut messages: MessageWriter<PartDamageMessage>,
    query: Query<
        (
            Entity,
            &Name,
            &ComposedCollisionState,
            &ComposedResolvedParts,
            &ComposedHealthPools,
        ),
        With<EnemyMosquiton>,
    >,
) {
    let Some(request) = take_debug_probe_request(&keys, &mut debug_probe) else {
        return;
    };
    let part_id = request.part_id.clone();
    let damage = request.damage;

    let Ok((entity, name, collision_state, resolved_parts, health_pools)) = query.single() else {
        debug_probe.last_result = Some(DebugComposedDamageProbeResult {
            requested_part_id: part_id,
            resolved_part_id: None,
            damage,
            pool_id: None,
            pool_before: None,
            probe_point: None,
            dispatched: false,
            error: Some(
                "debug composed-damage probe requires exactly one live Mosquiton target"
                    .to_string(),
            ),
        });
        error!("Debug composed-damage probe requested but no single Mosquiton target is available");
        return;
    };

    let Some(part) = resolved_parts
        .parts()
        .iter()
        .find(|part| part.part_id == part_id.as_str())
    else {
        debug_probe.last_result = Some(DebugComposedDamageProbeResult {
            requested_part_id: part_id.clone(),
            resolved_part_id: None,
            damage,
            pool_id: None,
            pool_before: None,
            probe_point: None,
            dispatched: false,
            error: Some(format!(
                "semantic part '{part_id}' is not active as a resolved visual part on {name}"
            )),
        });
        error!(
            "Debug composed-damage probe requested unresolved semantic part '{}' on {}",
            part_id, name
        );
        return;
    };

    let Some(collision) = collision_state
        .collisions()
        .iter()
        .find(|collision| collision.part_id == part_id.as_str())
    else {
        debug_probe.last_result = Some(DebugComposedDamageProbeResult {
            requested_part_id: part_id.clone(),
            resolved_part_id: None,
            damage,
            pool_id: part.health_pool.clone(),
            pool_before: part
                .health_pool
                .as_deref()
                .and_then(|pool_id| health_pools.pools().get(pool_id).copied()),
            probe_point: None,
            dispatched: false,
            error: Some(format!(
                "semantic part '{part_id}' is not collidable on {name}"
            )),
        });
        error!(
            "Debug composed-damage probe requested non-collidable part '{}' on {}",
            part_id, name
        );
        return;
    };

    let probe_point = collision.pivot_position + collision.collider.offset;
    let Some(resolved_collision) = collision_state.point_collides(probe_point) else {
        debug_probe.last_result = Some(DebugComposedDamageProbeResult {
            requested_part_id: part_id.clone(),
            resolved_part_id: None,
            damage,
            pool_id: part.health_pool.clone(),
            pool_before: part
                .health_pool
                .as_deref()
                .and_then(|pool_id| health_pools.pools().get(pool_id).copied()),
            probe_point: Some(probe_point),
            dispatched: false,
            error: Some(format!(
                "probe point for semantic part '{part_id}' did not collide on {name}"
            )),
        });
        error!(
            "Debug composed-damage probe for '{}' did not resolve any collision on {}",
            part_id, name
        );
        return;
    };
    if resolved_collision.part_id != part_id {
        debug_probe.last_result = Some(DebugComposedDamageProbeResult {
            requested_part_id: part_id.clone(),
            resolved_part_id: Some(resolved_collision.part_id.clone()),
            damage,
            pool_id: part.health_pool.clone(),
            pool_before: part
                .health_pool
                .as_deref()
                .and_then(|pool_id| health_pools.pools().get(pool_id).copied()),
            probe_point: Some(probe_point),
            dispatched: false,
            error: Some(format!(
                "probe point for semantic part '{part_id}' resolved to '{}'",
                resolved_collision.part_id
            )),
        });
        error!(
            "Debug composed-damage probe expected '{}' but collision resolved to '{}' on {}",
            part_id, resolved_collision.part_id, name
        );
        return;
    }

    info!(
        "Debug composed probe {:?} {} -> part '{}' at {:?}, pool {:?}, pools {:?}",
        entity, name, part.part_id, probe_point, part.health_pool, health_pools
    );
    debug_probe.last_result = Some(DebugComposedDamageProbeResult {
        requested_part_id: part_id,
        resolved_part_id: Some(resolved_collision.part_id.clone()),
        damage,
        pool_id: part.health_pool.clone(),
        pool_before: part
            .health_pool
            .as_deref()
            .and_then(|pool_id| health_pools.pools().get(pool_id).copied()),
        probe_point: Some(probe_point),
        dispatched: true,
        error: None,
    });
    messages.write(PartDamageMessage::new(entity, part.part_id.clone(), damage));
}

/// @system Logs composed pool mutations so semantic damage routing is visible without inference.
pub fn log_composed_health_pool_changes(
    query: Query<(&Name, &ComposedHealthPools), Changed<ComposedHealthPools>>,
) {
    for (name, pools) in &query {
        info!("Composed health pools updated for {}: {:?}", name, pools);
    }
}

fn take_debug_probe_request(
    keys: &ButtonInput<KeyCode>,
    debug_probe: &mut DebugComposedDamageProbe,
) -> Option<DebugComposedDamageProbeRequest> {
    if let Some(request) = debug_probe.request.take() {
        return Some(request);
    }

    if keys.just_pressed(KeyCode::KeyH) {
        Some(DebugComposedDamageProbeRequest {
            part_id: "head".to_string(),
            damage: DEBUG_HEAD_DAMAGE,
        })
    } else if keys.just_pressed(KeyCode::KeyB) {
        Some(DebugComposedDamageProbeRequest {
            part_id: "body".to_string(),
            damage: DEBUG_BODY_DAMAGE,
        })
    } else if keys.just_pressed(KeyCode::KeyR) {
        Some(DebugComposedDamageProbeRequest {
            part_id: "arm_r".to_string(),
            damage: DEBUG_ARM_DAMAGE,
        })
    } else {
        None
    }
}

// ── Pixel mask outline debug drawing ──────────────────────────────

/// @system Draws pixel mask outlines for simple and composed mask-driven targets.
///
/// Uses the same shared mask placement as gameplay hit detection so debug
/// outlines reflect the real authoritative collision shape.
pub(crate) fn draw_pixel_mask_outlines(
    mut gizmos: Gizmos,
    overlay: Res<DebugColliderOverlay>,
    camera_query: Query<&WorldPos, With<CameraPos>>,
    screen: Res<CxScreen>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut collision_assets: MaskCollisionAssets<'_, '_>,
    query: Query<
        (
            &WorldPos,
            Option<&ColliderData>,
            Option<&ComposedCollisionState>,
            &CxPosition,
            (&CxAnchor, &CxRenderSpace),
            (
                Option<&CxFrameView>,
                Option<&CxSprite>,
                Option<&CxAtlasSprite>,
                Option<&CxPresentationTransform>,
            ),
            Option<&ComposedResolvedParts>,
            Option<&ComposedAtlasBindings>,
            (
                Option<&Enemy>,
                Option<&EnemyAttack>,
                Option<&Destructible>,
                Option<&PlayerAttack>,
            ),
        ),
        Or<(With<ColliderData>, With<ComposedCollisionState>)>,
    >,
) {
    if !overlay.enabled {
        return;
    }
    let camera_pos = camera_query.single().unwrap();
    let camera_world = camera_pos.0.round().as_ivec2();
    let Ok(window) = windows.single() else {
        return;
    };
    let overlay_transform = CxOverlayViewportTransform::from_screen(
        &screen,
        Vec2::new(window.width(), window.height()),
    );
    collision_assets.refresh();

    for (
        world_pos,
        collider_data,
        composed_collision_state,
        position,
        (anchor, canvas),
        (frame, sprite, atlas_sprite, presentation),
        resolved_parts,
        bindings,
        (enemy, enemy_attack, destructible, player_attack),
    ) in query.iter()
    {
        // Player attacks with geometric colliders have no mask to outline.
        let has_geometric_player_colliders = player_attack.is_some()
            && collider_data.is_some_and(|d| {
                d.0.iter().any(|c| {
                    !matches!(
                        c.shape,
                        ColliderShape::SpriteMask | ColliderShape::SpriteMaskClosed
                    )
                })
            });
        if has_geometric_player_colliders {
            continue;
        }

        let entity_class =
            debug_collider_entity_class(enemy, enemy_attack, destructible, player_attack);
        let target = CollisionTarget {
            position,
            world_pos,
            anchor,
            canvas,
            frame,
            sprite,
            atlas_sprite,
            presentation,
            collider_data,
            composed_collision_state,
            composed_resolved_parts: resolved_parts,
            composed_atlas_bindings: bindings,
            enemy,
            destructible,
        };

        visit_target_debug_collider(
            target,
            camera_world,
            &mut collision_assets,
            |mask| {
                draw_mask_outline_instance(
                    &mut gizmos,
                    camera_pos.0,
                    overlay_transform,
                    debug_collider_color(entity_class, DebugColliderVisualKind::Mask),
                    mask,
                );
            },
            |_, _| {},
        );
    }
}

fn draw_mask_outline_instance(
    gizmos: &mut Gizmos<'_, '_>,
    camera_pos: Vec2,
    overlay_transform: CxOverlayViewportTransform,
    color: Color,
    mask: WorldMaskInstance<'_>,
) {
    let source_size = mask.source.frame_size();
    let boundary = if mask.closed {
        extract_mask_boundary_closed(mask.source, mask.frame)
    } else {
        extract_mask_boundary(mask.source, mask.frame)
    };
    let segments = boundary
        .into_iter()
        .map(|edge| mask_edge_to_world_points(mask.world, source_size, edge));
    draw_world_mask_outline_2d(gizmos, segments, color, |point| {
        to_overlay_point(point - camera_pos, overlay_transform)
    });
}

fn debug_collider_entity_class(
    enemy: Option<&Enemy>,
    enemy_attack: Option<&EnemyAttack>,
    destructible: Option<&Destructible>,
    player_attack: Option<&PlayerAttack>,
) -> DebugColliderEntityClass {
    if player_attack.is_some() {
        DebugColliderEntityClass::PlayerShot
    } else if enemy_attack.is_some() {
        DebugColliderEntityClass::Projectile
    } else if destructible.is_some() {
        DebugColliderEntityClass::Destructible
    } else {
        let _ = enemy;
        DebugColliderEntityClass::Enemy
    }
}

fn debug_collider_color(
    entity_class: DebugColliderEntityClass,
    kind: DebugColliderVisualKind,
) -> Color {
    let base_hue = match kind {
        DebugColliderVisualKind::Mask => MASK_BASE_HUE_DEG,
        DebugColliderVisualKind::Box => BOX_BASE_HUE_DEG,
        DebugColliderVisualKind::Radial => RADIAL_BASE_HUE_DEG,
    };
    if matches!(entity_class, DebugColliderEntityClass::PlayerShot) {
        return Color::hsla(
            PLAYER_SHOT_HUE_DEG,
            PLAYER_SHOT_SATURATION,
            PLAYER_SHOT_LIGHTNESS,
            COLLIDER_ALPHA,
        );
    }

    let (hue_offset_percent, saturation_delta, lightness_delta) = match entity_class {
        DebugColliderEntityClass::Enemy => (
            ENEMY_HUE_OFFSET_PERCENT,
            ENEMY_SATURATION_DELTA,
            ENEMY_LIGHTNESS_DELTA,
        ),
        DebugColliderEntityClass::Projectile => (
            PROJECTILE_HUE_OFFSET_PERCENT,
            PROJECTILE_SATURATION_DELTA,
            PROJECTILE_LIGHTNESS_DELTA,
        ),
        DebugColliderEntityClass::Destructible => (
            DESTRUCTIBLE_HUE_OFFSET_PERCENT,
            DESTRUCTIBLE_SATURATION_DELTA,
            DESTRUCTIBLE_LIGHTNESS_DELTA,
        ),
        DebugColliderEntityClass::PlayerShot => unreachable!(),
    };

    let hue = (base_hue + hue_offset_percent * 360.0).rem_euclid(360.0);
    let saturation = (BASE_SATURATION + saturation_delta).clamp(0.0, 1.0);
    let lightness = (BASE_LIGHTNESS + lightness_delta).clamp(0.0, 1.0);
    Color::hsla(hue, saturation, lightness, COLLIDER_ALPHA)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shift_g_toggles_debug_god_mode() {
        let mut app = App::new();
        app.insert_resource(ButtonInput::<KeyCode>::default());
        app.insert_resource(DebugGodMode::new(true));
        app.add_systems(Update, toggle_debug_god_mode);

        {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.press(KeyCode::ShiftLeft);
            keys.press(KeyCode::KeyG);
        }
        app.update();
        assert!(!app.world().resource::<DebugGodMode>().enabled);
    }
}
