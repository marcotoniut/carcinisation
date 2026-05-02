use crate::{
    assets::CxAssets,
    stage::{
        collision::MaskCollisionAssets,
        components::{
            StageEntity,
            damage::{DamageFlicker, InvertFilter},
            interactive::{BurningCorpse, ColliderData, Health, Hittable},
            placement::{Airborne, Depth},
        },
        enemy::{
            components::{
                CircleAround, LinearTween,
                behavior::{EnemyBehaviors, EnemyCurrentBehavior, GroundedEnemyFall, JumpTween},
            },
            composed::{ComposedAtlasBindings, ComposedResolvedParts},
            mosquito::entity::{EnemyMosquito, EnemyMosquitoAnimation, EnemyMosquitoAttacking},
            mosquiton::entity::{EnemyMosquiton, EnemyMosquitonAnimation, FallingState},
            spidey::entity::{EnemySpidey, EnemySpideyAnimation, EnemySpideyAttacking},
            tardigrade::entity::{EnemyTardigrade, EnemyTardigradeAnimation},
        },
        player::flamethrower::FlamethrowerConfig,
        resources::StageTimeDomain,
    },
    stubs::Score,
};
use bevy::prelude::*;
use carapace::prelude::{
    CxAnchor, CxAnimation, CxAnimationBundle, CxAnimationDirection, CxAnimationDuration,
    CxAnimationFinishBehavior, CxAtlasSprite, CxFilter, CxFrameTransition, CxFrameView, CxPosition,
    CxPresentationTransform, CxRenderSpace, CxSprite, CxSpriteAtlasAsset, WorldPos,
};
use carcinisation_base::fire_death::{FireDeathConfig, PerimeterFlame, perimeter_flames_from_mask};
use carcinisation_base::layer::{FlameDepth, Layer, OrsLayer};
use carcinisation_collision::{
    AtlasMaskFrames, PixelMaskSource, WorldMaskInstance, WorldMaskRect, world_mask_contains_point,
    world_mask_rect_from_top_left,
};
use carcinisation_core::components::{DespawnMark, GBColor};
use cween::linear::components::{TargetingValueX, TargetingValueY, TargetingValueZ};

#[derive(Component, Clone, Copy, Debug)]
pub struct BurningCorpseFlame {
    pub corpse: Entity,
}

#[derive(Component, Clone, Copy, Debug)]
pub struct BurningCorpsePrepared;

#[derive(Component, Clone, Copy, Debug)]
pub struct BurningCorpseFlamesSpawned;

pub fn start_burning_corpses(
    mut commands: Commands,
    config: Res<FlamethrowerConfig>,
    mut score: ResMut<Score>,
    asset_server: Res<AssetServer>,
    atlas_assets: Res<Assets<CxSpriteAtlasAsset>>,
    mut collision_assets: MaskCollisionAssets<'_, '_>,
    filters: CxAssets<CxFilter>,
    query: Query<
        (
            Entity,
            &BurningCorpse,
            &WorldPos,
            &Depth,
            Option<&CxFrameView>,
            Option<&CxSprite>,
            Option<&CxAtlasSprite>,
            Option<&CxPresentationTransform>,
            Option<&ComposedResolvedParts>,
            Option<&ComposedAtlasBindings>,
            Option<&BurningCorpsePrepared>,
            Option<&EnemyMosquito>,
            Option<&EnemyMosquiton>,
            Option<&EnemySpidey>,
            Option<&EnemyTardigrade>,
        ),
        (With<BurningCorpse>, Without<BurningCorpseFlamesSpawned>),
    >,
) {
    collision_assets.refresh();
    let char_filter = filters.load_color(GBColor::Black);
    let flame_atlas: Handle<CxSpriteAtlasAsset> = asset_server.load(config.atlas_path.clone());
    let region_id = atlas_assets
        .get(&flame_atlas)
        .and_then(|atlas| atlas.region_id(&config.animation_tag))
        .unwrap_or_default();
    let anim_duration_ms = atlas_assets
        .get(&flame_atlas)
        .and_then(|atlas| atlas.animation(&config.animation_tag))
        .map_or(400, |animation| animation.duration_ms);

    for (
        entity,
        burning,
        position,
        depth,
        frame,
        sprite,
        atlas_sprite,
        presentation,
        resolved_parts,
        atlas_bindings,
        prepared,
        mosquito,
        mosquiton,
        spidey,
        tardigrade,
    ) in &query
    {
        if prepared.is_none() {
            prepare_burning_corpse(
                &mut commands,
                entity,
                char_filter.clone(),
                &mut score,
                burning_corpse_score(mosquito, mosquiton, spidey, tardigrade),
            );
        }

        let Some(mask) = burning_corpse_flame_mask(
            burning.seed,
            position.0,
            frame,
            sprite,
            atlas_sprite,
            presentation,
            resolved_parts,
            atlas_bindings,
            &mut collision_assets,
            &config.fire_death_config(),
        ) else {
            warn!(
                "Burning corpse {:?} has no usable pixel mask yet; will retry corpse flames",
                entity
            );
            continue;
        };

        spawn_burning_corpse_flames(
            &mut commands,
            entity,
            position.0,
            *depth,
            &flame_atlas,
            region_id,
            anim_duration_ms,
            mask,
        );
    }
}

fn prepare_burning_corpse(
    commands: &mut Commands,
    entity: Entity,
    char_filter: Handle<carapace::prelude::CxFilterAsset>,
    score: &mut Score,
    kill_score: u32,
) {
    score.add_u(kill_score);
    commands
        .entity(entity)
        .remove::<Hittable>()
        .remove::<ColliderData>()
        .remove::<DamageFlicker>()
        .remove::<InvertFilter>()
        .remove::<Health>()
        .remove::<CxAnimation>()
        .remove::<EnemyCurrentBehavior>()
        .remove::<EnemyBehaviors>()
        .remove::<CircleAround>()
        .remove::<LinearTween>()
        .remove::<JumpTween>()
        .remove::<GroundedEnemyFall>()
        .remove::<Airborne>()
        .remove::<FallingState>()
        .remove::<TargetingValueX>()
        .remove::<TargetingValueY>()
        .remove::<TargetingValueZ>()
        .remove::<EnemyMosquitoAttacking>()
        .remove::<EnemySpideyAttacking>()
        .remove::<EnemyMosquitoAnimation>()
        .remove::<EnemyMosquitonAnimation>()
        .remove::<EnemySpideyAnimation>()
        .remove::<EnemyTardigradeAnimation>()
        .insert((
            CxFilter(char_filter),
            Layer::Ors(OrsLayer::Front),
            BurningCorpsePrepared,
        ));
}

#[allow(clippy::too_many_arguments)]
fn spawn_burning_corpse_flames(
    commands: &mut Commands,
    corpse: Entity,
    corpse_position: Vec2,
    depth: Depth,
    flame_atlas: &Handle<CxSpriteAtlasAsset>,
    region_id: carapace::prelude::AtlasRegionId,
    anim_duration_ms: u64,
    mask: BurningCorpseFlameMask,
) {
    commands.entity(corpse).insert(BurningCorpseFlamesSpawned);

    for flame in mask.flames {
        let flame_pos = corpse_position + mask.center_offset_px + flame.offset_px;
        commands.spawn((
            BurningCorpseFlame { corpse },
            Name::new("Burning corpse flame"),
            WorldPos::from(flame_pos),
            CxPosition::from(flame_pos.round().as_ivec2()),
            CxAtlasSprite::new(flame_atlas.clone(), region_id),
            CxAnimationBundle::from_parts(
                CxAnimationDirection::Forward,
                CxAnimationDuration::millis_per_animation(anim_duration_ms),
                CxAnimationFinishBehavior::Loop,
                CxFrameTransition::None,
            ),
            CxAnchor::Center,
            CxRenderSpace::World,
            Layer::Ors(OrsLayer::FlameSegment(FlameDepth(8))),
            CxPresentationTransform {
                scale: Vec2::splat(flame.scale),
                ..default()
            },
            depth,
            StageEntity,
        ));
    }
}

struct BurningCorpseFlameMask {
    center_offset_px: Vec2,
    flames: Vec<PerimeterFlame>,
}

#[allow(clippy::too_many_arguments)]
fn burning_corpse_flame_mask(
    seed: u32,
    corpse_position: Vec2,
    frame: Option<&CxFrameView>,
    sprite: Option<&CxSprite>,
    atlas_sprite: Option<&CxAtlasSprite>,
    presentation: Option<&CxPresentationTransform>,
    resolved_parts: Option<&ComposedResolvedParts>,
    atlas_bindings: Option<&ComposedAtlasBindings>,
    collision_assets: &mut MaskCollisionAssets<'_, '_>,
    config: &FireDeathConfig,
) -> Option<BurningCorpseFlameMask> {
    if let (Some(resolved_parts), Some(atlas_bindings)) = (resolved_parts, atlas_bindings)
        && let Some(mask) = composed_corpse_flame_mask(
            seed,
            corpse_position,
            resolved_parts,
            atlas_bindings,
            collision_assets,
            config,
        )
    {
        return Some(mask);
    }

    if let Some(sprite) = sprite
        && let Some(sprite_pixels) = collision_assets.sprite_pixels(&sprite.0)
    {
        let source = PixelMaskSource::Sprite(sprite_pixels.as_ref());
        return Some(flames_from_mask_source(
            seed,
            source,
            frame.copied(),
            presentation,
            config,
        ));
    }

    if let Some(atlas_sprite) = atlas_sprite {
        let region = collision_assets
            .atlas_sprite_region(atlas_sprite)
            .cloned()?;
        let atlas_pixels = collision_assets.atlas_pixels(&atlas_sprite.atlas)?;
        let source = PixelMaskSource::Atlas {
            atlas: atlas_pixels.as_ref(),
            frames: AtlasMaskFrames::Region(&region),
        };
        return Some(flames_from_mask_source(
            seed,
            source,
            frame.copied(),
            presentation,
            config,
        ));
    }

    None
}

fn flames_from_mask_source(
    seed: u32,
    source: PixelMaskSource<'_>,
    frame: Option<CxFrameView>,
    presentation: Option<&CxPresentationTransform>,
    config: &FireDeathConfig,
) -> BurningCorpseFlameMask {
    let source_size = source.frame_size();
    let display_size = presentation_scaled_size(source_size, presentation);
    let scale = presentation.map_or(Vec2::ONE, |presentation| presentation.scale);
    let world = WorldMaskRect {
        rect: IRect {
            min: IVec2::ZERO,
            max: display_size.as_ivec2(),
        },
        flip_x: scale.x.is_sign_negative(),
        flip_y: scale.y.is_sign_negative(),
    };
    let mask = WorldMaskInstance {
        source,
        frame,
        world,
        closed: false,
    };
    let height = display_size.y as i32;

    BurningCorpseFlameMask {
        center_offset_px: presentation
            .map_or(Vec2::ZERO, |presentation| presentation.visual_offset),
        flames: perimeter_flames_from_mask(
            seed,
            display_size.x as usize,
            display_size.y as usize,
            |x, y| world_mask_contains_point(mask, IVec2::new(x as i32, height - 1 - y as i32)),
            config,
        ),
    }
}

fn composed_corpse_flame_mask(
    seed: u32,
    corpse_position: Vec2,
    resolved_parts: &ComposedResolvedParts,
    atlas_bindings: &ComposedAtlasBindings,
    collision_assets: &mut MaskCollisionAssets<'_, '_>,
    config: &FireDeathConfig,
) -> Option<BurningCorpseFlameMask> {
    let atlas_pixels = collision_assets.atlas_pixels(atlas_bindings.atlas_handle())?;
    let mut masks = Vec::new();
    let mut min = IVec2::splat(i32::MAX);
    let mut max = IVec2::splat(i32::MIN);

    for fragment in resolved_parts.fragments() {
        let Some(rect) = atlas_bindings.sprite_rect(fragment.sprite_id.as_str()) else {
            continue;
        };
        let Some(world) = world_mask_rect_from_top_left(
            fragment.visual_top_left_position,
            fragment.frame_size,
            fragment.flip_x,
            fragment.flip_y,
        ) else {
            continue;
        };
        min = min.min(world.rect.min);
        max = max.max(world.rect.max);
        masks.push(WorldMaskInstance {
            source: PixelMaskSource::Atlas {
                atlas: atlas_pixels.as_ref(),
                frames: AtlasMaskFrames::Single(rect),
            },
            frame: None,
            world,
            closed: false,
        });
    }

    if masks.is_empty() || min.x >= max.x || min.y >= max.y {
        return None;
    }

    let size = max - min;
    let width = usize::try_from(size.x).ok()?;
    let height = usize::try_from(size.y).ok()?;
    let center = (min.as_vec2() + max.as_vec2()) * 0.5;

    Some(BurningCorpseFlameMask {
        center_offset_px: center - corpse_position,
        flames: perimeter_flames_from_mask(
            seed,
            width,
            height,
            |x, y| {
                let point = IVec2::new(min.x + x as i32, max.y - 1 - y as i32);
                masks
                    .iter()
                    .any(|mask| world_mask_contains_point(*mask, point))
            },
            config,
        ),
    })
}

fn presentation_scaled_size(
    source_size: UVec2,
    presentation: Option<&CxPresentationTransform>,
) -> UVec2 {
    let scale = presentation.map_or(Vec2::ONE, |presentation| presentation.scale.abs());
    UVec2::new(
        scaled_dimension(source_size.x, scale.x),
        scaled_dimension(source_size.y, scale.y),
    )
}

fn scaled_dimension(size: u32, scale: f32) -> u32 {
    if size == 0 {
        1
    } else {
        ((size as f32 * scale.max(0.01)).round() as u32).max(1)
    }
}

pub fn tick_burning_corpses(
    mut commands: Commands,
    stage_time: Res<Time<StageTimeDomain>>,
    corpse_query: Query<(Entity, &BurningCorpse)>,
    flame_query: Query<(Entity, &BurningCorpseFlame)>,
) {
    for (corpse, burning) in &corpse_query {
        if stage_time.elapsed().saturating_sub(burning.started) < burning.duration {
            continue;
        }
        commands.entity(corpse).insert(DespawnMark);
        for (flame, owner) in &flame_query {
            if owner.corpse == corpse {
                commands.entity(flame).insert(DespawnMark);
            }
        }
    }
}

fn burning_corpse_score(
    mosquito: Option<&EnemyMosquito>,
    mosquiton: Option<&EnemyMosquiton>,
    spidey: Option<&EnemySpidey>,
    tardigrade: Option<&EnemyTardigrade>,
) -> u32 {
    if let Some(mosquiton) = mosquiton {
        mosquiton.kill_score()
    } else if let Some(mosquito) = mosquito {
        mosquito.kill_score()
    } else if let Some(spidey) = spidey {
        spidey.kill_score()
    } else if let Some(tardigrade) = tardigrade {
        tardigrade.kill_score()
    } else {
        0
    }
}
