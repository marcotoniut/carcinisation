//! Gizmo drawing helpers for visualising stage data during development.

use super::{
    DebugColor, DebugComposedDamageProbe, DebugComposedDamageProbeRequest,
    DebugComposedDamageProbeResult, DebugGodMode,
};
use crate::{
    globals::{SCREEN_RESOLUTION, VIEWPORT_MULTIPLIER, VIEWPORT_RESOLUTION_OFFSET},
    stage::{
        components::{
            interactive::{ColliderData, ColliderShape},
            placement::{Depth, Floor},
        },
        enemy::{
            composed::{ComposedCollisionState, ComposedHealthPools, ComposedResolvedParts},
            mosquiton::entity::EnemyMosquiton,
        },
        messages::PartDamageMessage,
    },
    systems::camera::CameraPos,
};
use bevy::prelude::*;
use seldom_pixel::prelude::PxSubPosition;

pub const LINE_EXTENSION: f32 = 1000.;

const SCREEN_X: f32 = SCREEN_RESOLUTION.x as f32;
const SCREEN_Y: f32 = SCREEN_RESOLUTION.y as f32;
const DEBUG_HEAD_DAMAGE: u32 = 3;
const DEBUG_BODY_DAMAGE: u32 = 5;
const DEBUG_ARM_DAMAGE: u32 = 4;

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

    let shift_held = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    if shift_held && keys.just_pressed(KeyCode::KeyG) {
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

pub fn to_viewport_ratio_x(x: f32) -> f32 {
    VIEWPORT_MULTIPLIER * x
}

pub fn to_viewport_ratio_y(y: f32) -> f32 {
    VIEWPORT_MULTIPLIER * y
}

pub fn to_viewport_ratio(v: Vec2) -> Vec2 {
    Vec2::new(to_viewport_ratio_x(v.x), to_viewport_ratio_y(v.y))
}

pub fn to_viewport_coordinate_x(x: f32) -> f32 {
    VIEWPORT_RESOLUTION_OFFSET.x + VIEWPORT_MULTIPLIER * (x - SCREEN_X * 0.5)
}

pub fn to_viewport_coordinate_y(y: f32) -> f32 {
    VIEWPORT_RESOLUTION_OFFSET.y + VIEWPORT_MULTIPLIER * (y - SCREEN_Y * 0.5)
}

pub fn to_viewport_coordinates(position: Vec2) -> Vec2 {
    Vec2::new(
        to_viewport_coordinate_x(position.x),
        to_viewport_coordinate_y(position.y),
    )
}

/// @system Renders the configured floor heights as horizontal gizmo lines.
pub fn draw_floor_lines(mut gizmos: Gizmos, query: Query<(&Depth, &Floor)>) {
    for (_, floor) in query.iter() {
        let floor_y = to_viewport_coordinate_y(floor.0);
        // TODO calculate position in the real camera SCREEN_RES vs the virtual one
        gizmos.line(
            Vec3::new(-LINE_EXTENSION, floor_y, 0.),
            Vec3::new(LINE_EXTENSION, floor_y, 0.),
            Color::YELLOW_GREEN,
        );
    }
}

/// @system Visualises collider shapes in the viewport-relative coordinate space.
pub fn draw_colliders(
    mut gizmos: Gizmos,
    camera_query: Query<&PxSubPosition, With<CameraPos>>,
    query: Query<(&ColliderData, &PxSubPosition)>,
) {
    let camera_pos = camera_query.single().unwrap();

    for (data, position) in query.iter() {
        let absolute_position = position.0 - camera_pos.0;
        for data in &data.0 {
            match data.shape {
                ColliderShape::Circle(radius) => {
                    gizmos.circle_2d(
                        to_viewport_coordinates(absolute_position + data.offset),
                        to_viewport_ratio_x(radius),
                        Color::ALICE_BLUE,
                    );
                }
                ColliderShape::Box(size) => {
                    gizmos.rect_2d(
                        to_viewport_coordinates(absolute_position + data.offset),
                        to_viewport_ratio(size),
                        Color::FUCHSIA,
                    );
                }
            }
        }
    }
}

/// @system Visualises resolved composed-part pivots and collision shapes in world space.
pub fn draw_composed_parts(
    mut gizmos: Gizmos,
    camera_query: Query<&PxSubPosition, With<CameraPos>>,
    query: Query<(&ComposedCollisionState, &ComposedResolvedParts)>,
) {
    let camera_pos = camera_query.single().unwrap();

    for (collisions, resolved_parts) in query.iter() {
        for part in resolved_parts.parts() {
            gizmos.circle_2d(
                to_viewport_coordinates(part.world_pivot_position - camera_pos.0),
                to_viewport_ratio_x(1.5),
                Color::YELLOW_GREEN,
            );
        }

        for collision in collisions.collisions() {
            let center = collision.pivot_position + collision.collider.offset - camera_pos.0;
            match collision.collider.shape {
                ColliderShape::Circle(radius) => {
                    gizmos.circle_2d(
                        to_viewport_coordinates(center),
                        to_viewport_ratio_x(radius),
                        Color::srgb(1.0, 0.55, 0.0),
                    );
                }
                ColliderShape::Box(size) => {
                    gizmos.rect_2d(
                        to_viewport_coordinates(center),
                        to_viewport_ratio(size),
                        Color::srgb(0.1, 0.9, 0.9),
                    );
                }
            }
        }
    }
}

/// @system In debug builds, triggers deterministic Mosquiton part damage probes.
///
/// The probe resolves through the same composed collision state used by live
/// gameplay before emitting `PartDamageMessage`, so its success/failure path is
/// a runtime proof of semantic targeting rather than a separate shortcut.
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
