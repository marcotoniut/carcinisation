use crate::{
    globals::SCREEN_RESOLUTION,
    stage::{components::placement::Depth, enemy::entity::EnemyType, resources::StageTimeDomain},
    systems::camera::CameraPos,
};
use asset_pipeline::aseprite::{Animation, CompositionAtlas};
use bevy::{
    asset::Asset,
    image::Image,
    prelude::*,
    reflect::TypePath,
    sprite::{Anchor, Sprite},
};
use seldom_pixel::prelude::PxSubPosition;
use serde::Deserialize;

#[derive(Asset, Clone, Debug, Deserialize, TypePath)]
pub struct CompositionAtlasAsset {
    #[serde(flatten)]
    pub atlas: CompositionAtlas,
}

#[derive(Component, Clone, Debug)]
pub struct ComposedEnemyVisual {
    pub atlas: Handle<CompositionAtlasAsset>,
    pub image: Handle<Image>,
    pub requested_tag: String,
    active_tag: String,
    frame_index: usize,
    frame_started_at_ms: u64,
    ping_pong_forward: bool,
}

impl ComposedEnemyVisual {
    #[must_use]
    pub fn for_enemy(
        asset_server: &AssetServer,
        enemy_type: EnemyType,
        depth: Depth,
        initial_tag: &str,
    ) -> Self {
        let base_path = format!(
            "sprites/enemies/{}_{}/atlas",
            enemy_type.sprite_base_name(),
            depth.to_i8()
        );

        Self {
            atlas: asset_server.load(format!("{base_path}.json")),
            image: asset_server.load(format!("{base_path}.png")),
            requested_tag: initial_tag.to_string(),
            active_tag: String::new(),
            frame_index: 0,
            frame_started_at_ms: 0,
            ping_pong_forward: true,
        }
    }
}

#[derive(Component, Debug)]
pub struct ComposedEnemyVisualReady;

#[derive(Component, Clone, Debug)]
pub struct ComposedEnemyPart {
    pub part_id: String,
    pub draw_order: u32,
}

pub fn ensure_composed_enemy_parts(
    mut commands: Commands,
    atlas_assets: Res<Assets<CompositionAtlasAsset>>,
    query: Query<(Entity, &ComposedEnemyVisual), Without<ComposedEnemyVisualReady>>,
) {
    for (entity, visual) in &query {
        let Some(atlas_asset) = atlas_assets.get(&visual.atlas) else {
            continue;
        };

        commands.entity(entity).with_children(|parent| {
            for part in &atlas_asset.atlas.parts {
                parent.spawn((
                    Name::new(format!("ComposedEnemyPart<{}>", part.id)),
                    ComposedEnemyPart {
                        part_id: part.id.clone(),
                        draw_order: part.draw_order,
                    },
                    Sprite::from_image(visual.image.clone()),
                    Anchor::TOP_LEFT,
                    Transform::default(),
                    Visibility::Hidden,
                ));
            }
        });
        commands.entity(entity).insert(ComposedEnemyVisualReady);
    }
}

pub fn update_composed_enemy_visuals(
    atlas_assets: Res<Assets<CompositionAtlasAsset>>,
    camera_query: Query<&PxSubPosition, With<CameraPos>>,
    stage_time: Res<Time<StageTimeDomain>>,
    mut root_query: Query<
        (&mut ComposedEnemyVisual, &PxSubPosition, &Depth, &Children),
        With<ComposedEnemyVisualReady>,
    >,
    mut child_query: Query<(
        &ComposedEnemyPart,
        &mut Sprite,
        &mut Transform,
        &mut Visibility,
    )>,
) {
    let Ok(camera) = camera_query.single() else {
        return;
    };
    let now_ms = stage_time.elapsed().as_millis() as u64;

    for (mut visual, position, depth, children) in &mut root_query {
        let Some(atlas_asset) = atlas_assets.get(&visual.atlas) else {
            continue;
        };

        let Some(animation) = atlas_asset
            .atlas
            .animations
            .iter()
            .find(|animation| animation.tag == visual.requested_tag)
        else {
            continue;
        };

        if visual.active_tag != visual.requested_tag {
            visual.active_tag = visual.requested_tag.clone();
            visual.frame_index = initial_frame_index(animation);
            visual.frame_started_at_ms = now_ms;
            visual.ping_pong_forward = !matches!(animation.direction.as_str(), "ping_pong_reverse");
        } else {
            advance_animation_frame(&mut visual, animation, now_ms);
        }

        let frame = &animation.frames[visual.frame_index];
        let mut placements = std::collections::HashMap::new();
        for placement in &frame.parts {
            placements.insert(placement.part_id.as_str(), placement);
        }

        for child in children.iter() {
            let Ok((part, mut sprite, mut transform, mut visibility)) = child_query.get_mut(child)
            else {
                continue;
            };

            let Some(placement) = placements.get(part.part_id.as_str()) else {
                *visibility = Visibility::Hidden;
                continue;
            };
            let Some(atlas_sprite) = atlas_asset
                .atlas
                .sprites
                .iter()
                .find(|sprite| sprite.id == placement.sprite_id)
            else {
                *visibility = Visibility::Hidden;
                continue;
            };

            let screen_position = position.0 - camera.0
                + Vec2::new(placement.offset.x as f32, placement.offset.y as f32);
            let world_position = screen_to_world(screen_position);

            sprite.rect = Some(bevy::math::Rect {
                min: Vec2::new(atlas_sprite.rect.x as f32, atlas_sprite.rect.y as f32),
                max: Vec2::new(
                    (atlas_sprite.rect.x + atlas_sprite.rect.w) as f32,
                    (atlas_sprite.rect.y + atlas_sprite.rect.h) as f32,
                ),
            });
            sprite.custom_size = Some(Vec2::new(
                atlas_sprite.rect.w as f32,
                atlas_sprite.rect.h as f32,
            ));
            sprite.flip_x = placement.flip_x;
            sprite.flip_y = placement.flip_y;
            sprite.color = Color::WHITE.with_alpha(f32::from(placement.opacity) / 255.0);

            transform.translation = Vec3::new(
                world_position.x,
                world_position.y,
                composed_part_z(*depth, part.draw_order),
            );
            *visibility = Visibility::Visible;
        }
    }
}

fn advance_animation_frame(visual: &mut ComposedEnemyVisual, animation: &Animation, now_ms: u64) {
    if animation.frames.is_empty() {
        return;
    }

    loop {
        let frame_duration = u64::from(animation.frames[visual.frame_index].duration_ms.max(1));
        if now_ms.saturating_sub(visual.frame_started_at_ms) < frame_duration {
            break;
        }

        visual.frame_started_at_ms = visual.frame_started_at_ms.saturating_add(frame_duration);
        visual.frame_index = next_frame_index(
            animation.direction.as_str(),
            visual.frame_index,
            animation.frames.len(),
            &mut visual.ping_pong_forward,
        );
    }
}

fn initial_frame_index(animation: &Animation) -> usize {
    if animation.frames.is_empty() {
        return 0;
    }

    match animation.direction.as_str() {
        "reverse" | "ping_pong_reverse" => animation.frames.len() - 1,
        _ => 0,
    }
}

fn next_frame_index(
    direction: &str,
    current: usize,
    frame_count: usize,
    ping_pong_forward: &mut bool,
) -> usize {
    if frame_count <= 1 {
        return 0;
    }

    match direction {
        "reverse" => {
            if current == 0 {
                frame_count - 1
            } else {
                current - 1
            }
        }
        "ping_pong" | "ping_pong_reverse" => {
            if *ping_pong_forward {
                if current + 1 >= frame_count {
                    *ping_pong_forward = false;
                    current.saturating_sub(1)
                } else {
                    current + 1
                }
            } else if current == 0 {
                *ping_pong_forward = true;
                1
            } else {
                current - 1
            }
        }
        _ => {
            if current + 1 >= frame_count {
                0
            } else {
                current + 1
            }
        }
    }
}

fn screen_to_world(screen_position: Vec2) -> Vec2 {
    Vec2::new(
        screen_position.x - SCREEN_RESOLUTION.x as f32 / 2.0,
        SCREEN_RESOLUTION.y as f32 / 2.0 - screen_position.y,
    )
}

fn composed_part_z(depth: Depth, draw_order: u32) -> f32 {
    100.0 - depth.to_f32() + draw_order as f32 * 0.001
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{collections::HashSet, fs, path::PathBuf};

    fn load_exported_mosquiton() -> CompositionAtlasAsset {
        let asset_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/sprites/enemies/mosquiton_3/atlas.json");
        let body = fs::read_to_string(&asset_path)
            .expect("generated mosquiton atlas.json should exist under assets/");
        serde_json::from_str(&body).expect("generated mosquiton atlas.json should deserialize")
    }

    #[test]
    fn exported_mosquiton_manifest_deserializes() {
        let atlas = load_exported_mosquiton();

        assert_eq!(atlas.atlas.entity, "mosquiton");
        assert_eq!(atlas.atlas.depth, 3);
        assert_eq!(atlas.atlas.schema_version, 1);
        assert!(atlas.atlas.parts.len() >= 4);
        assert!(!atlas.atlas.sprites.is_empty());
    }

    #[test]
    fn exported_mosquiton_parts_have_unique_ids() {
        let atlas = load_exported_mosquiton();
        let ids: HashSet<_> = atlas
            .atlas
            .parts
            .iter()
            .map(|part| part.id.as_str())
            .collect();

        assert_eq!(ids.len(), atlas.atlas.parts.len());
        assert!(
            atlas
                .atlas
                .animations
                .iter()
                .any(|animation| animation.tag == "idle_stand"),
            "expected idle_stand tag in exported mosquiton atlas"
        );
        assert!(
            atlas
                .atlas
                .animations
                .iter()
                .any(|animation| animation.tag == "shoot_stand"),
            "expected shoot_stand tag in exported mosquiton atlas"
        );
    }
}
