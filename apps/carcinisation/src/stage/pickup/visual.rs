//! Composed pickup visual assembly: loads atlas assets and builds
//! [`CxCompositeSprite`] from part-filtered poses.

use crate::stage::enemy::composed::CompositionAtlasAsset;
use asset_pipeline::composed_ron::CompactComposedAtlas;
use bevy::prelude::*;
use carapace::prelude::{
    AtlasRegionId, CxAuthoritativeCompositeMetrics, CxCompositePart, CxCompositeSprite,
    CxRenderSpace, CxSpriteAtlasAsset,
};

/// Pending pickup visual waiting for atlas assets to load.
#[derive(Component, Clone, Debug)]
pub struct PendingPickupVisual {
    pub atlas_manifest: Handle<CompositionAtlasAsset>,
    pub sprite_atlas: Handle<CxSpriteAtlasAsset>,
    pub visible_parts: &'static [&'static str],
    /// Optional render space override (e.g. Camera for HUD icons).
    pub render_space: Option<CxRenderSpace>,
}

/// Marker inserted once the composed sprite has been assembled.
#[derive(Component, Clone, Debug, Default)]
pub struct PickupVisualReady;

const PICKUP_ASSET_ROOT: &str = "sprites/pickups";
const PICKUP_ATLAS_BASENAME: &str = "atlas";

/// Loads the composed RON and px_atlas assets for the pickup_3 atlas.
#[must_use]
pub fn load_pickup_visual(
    asset_server: &AssetServer,
    visible_parts: &'static [&'static str],
) -> PendingPickupVisual {
    let base_path = format!("{}/pickup_3/{}", PICKUP_ASSET_ROOT, PICKUP_ATLAS_BASENAME);
    PendingPickupVisual {
        atlas_manifest: asset_server.load(format!("{base_path}.composed.ron")),
        sprite_atlas: asset_server.load(format!("{base_path}.px_atlas.ron")),
        visible_parts,
        render_space: None,
    }
}

/// @system Assembles pickup visuals once their atlas assets have loaded.
pub fn assemble_pickup_visuals(
    mut commands: Commands,
    atlas_assets: Res<Assets<CompositionAtlasAsset>>,
    query: Query<
        (Entity, &PendingPickupVisual),
        (Without<PickupVisualReady>, Without<CxCompositeSprite>),
    >,
) {
    for (entity, pending) in &query {
        let Some(manifest) = atlas_assets.get(&pending.atlas_manifest) else {
            continue;
        };

        let Some(composite) = build_pickup_composite(
            &manifest.atlas,
            &pending.sprite_atlas,
            pending.visible_parts,
        ) else {
            continue;
        };

        let mut entity_commands = commands.entity(entity);
        entity_commands.insert((
            composite,
            CxAuthoritativeCompositeMetrics,
            Visibility::Inherited,
            PickupVisualReady,
        ));
        if let Some(render_space) = pending.render_space {
            entity_commands.insert(render_space);
        }
    }
}

/// Builds a [`CxCompositeSprite`] from the compact composed atlas, filtering
/// only the parts whose names appear in `visible_parts`.
///
/// Uses the first frame of the "idle" animation. Poses with top-left offsets
/// (Y-down) are converted to bottom-left (carapace convention).
fn build_pickup_composite(
    atlas: &CompactComposedAtlas,
    sprite_atlas_handle: &Handle<CxSpriteAtlasAsset>,
    visible_parts: &[&str],
) -> Option<CxCompositeSprite> {
    let idle_anim = atlas.animations.iter().find(|a| a.tag == "idle")?;
    let frame = idle_anim.frames.first()?;

    let canvas_h = atlas.canvas.1 as i32;
    let origin = IVec2::new(atlas.origin.0 as i32, atlas.origin.1 as i32);

    let mut parts = Vec::new();
    let mut min = IVec2::MAX;
    let mut max = IVec2::MIN;

    for pose in &frame.poses {
        let part_idx = pose.p as usize;
        let part = atlas.parts.get(part_idx)?;
        if !part.visual {
            continue;
        }

        let part_name_idx = part.id as usize;
        let part_name = atlas.part_names.get(part_name_idx)?;
        if !visible_parts.contains(&part_name.as_str()) {
            continue;
        }

        let sprite_idx = pose.s as usize;
        let (sw, sh) = atlas.sprite_sizes.get(sprite_idx)?;
        let size = IVec2::new(*sw as i32, *sh as i32);

        // Pose offset is top-left, Y-down relative to composition origin.
        let top_left = IVec2::new(pose.o.0 as i32, pose.o.1 as i32);

        // Convert to bottom-left, Y-up (carapace convention):
        // bottom_left.x = origin.x + top_left.x
        // bottom_left.y = (canvas_h - origin.y) - (top_left.y + height)
        let bl_x = origin.x + top_left.x;
        let bl_y = (canvas_h - origin.y) - (top_left.y + size.y);
        let offset = IVec2::new(bl_x, bl_y);

        min = min.min(offset);
        max = max.max(offset + size);

        let mut part_entry = CxCompositePart::atlas_region(
            sprite_atlas_handle.clone(),
            AtlasRegionId(sprite_idx as u32),
        );
        part_entry.offset = offset;
        part_entry.flip_x = pose.fx;
        part_entry.flip_y = pose.fy;
        parts.push(part_entry);
    }

    if parts.is_empty() {
        return None;
    }

    let composite_size = (max - min).max(IVec2::ONE);
    Some(CxCompositeSprite {
        parts,
        size: UVec2::new(composite_size.x as u32, composite_size.y as u32),
        origin: min,
        render_size: UVec2::new(composite_size.x as u32, composite_size.y as u32),
        render_origin: min,
        frame_count: 1,
    })
}
