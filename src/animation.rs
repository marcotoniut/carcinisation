//! Animation
//!
//! Optional animation systems that drive [`PxFrameControl`] updates.

use std::time::Duration;

use bevy_platform::time::Instant;

pub use crate::frame::{
    PxFrame, PxFrameControl, PxFrameCount, PxFrameSelector, PxFrameTransition, PxFrameView,
};
use bevy_asset::AssetEvent;

use crate::{prelude::*, set::PxSet};

/// Optional plugin that installs the default animation systems.
#[derive(Default)]
pub struct PxAnimationPlugin;

impl Plugin for PxAnimationPlugin {
    fn build(&self, app: &mut App) {
        plug(app);
    }
}

pub(crate) fn plug(app: &mut App) {
    app.add_systems(
        PostUpdate,
        (
            sync_frame_counts_on_component_change::<PxSprite>,
            sync_frame_counts_on_atlas_sprite_change,
            sync_frame_counts_on_component_change::<PxFilter>,
            sync_frame_counts_on_component_change::<PxText>,
            sync_frame_counts_on_component_change::<PxMap>,
            sync_frame_counts_on_asset_event::<PxSprite>,
            sync_frame_counts_on_atlas_asset_event,
            sync_frame_counts_on_asset_event::<PxFilter>,
            sync_frame_counts_on_asset_event::<PxText>,
            sync_frame_counts_on_asset_event::<PxMap>,
            update_animations,
        )
            .chain()
            .in_set(PxSet::FinishAnimations),
    );
}

/// Direction the animation plays.
#[derive(Clone, Copy, Debug, Default)]
pub enum PxAnimationDirection {
    /// The animation plays foreward.
    #[default]
    Foreward,
    /// The animation plays backward.
    Backward,
}

/// Animation duration.
#[derive(Clone, Copy, Debug)]
pub enum PxAnimationDuration {
    /// Duration of the entire animation. When used on a tilemap, each tile's animation
    /// takes the same amount of time, but their frames may desync.
    PerAnimation(Duration),
    /// Duration of each frame. When used on a tilemap, each frame will take the same amount
    /// of time, but the tile's animations may desync.
    PerFrame(Duration),
}

impl Default for PxAnimationDuration {
    fn default() -> Self {
        Self::PerAnimation(Duration::from_secs(1))
    }
}

impl PxAnimationDuration {
    /// Creates a [`PxAnimationDuration::PerAnimation`] with the given number of milliseconds.
    #[must_use]
    pub fn millis_per_animation(millis: u64) -> Self {
        Self::PerAnimation(Duration::from_millis(millis))
    }

    /// Creates a [`PxAnimationDuration::PerFrame`] with the given number of milliseconds.
    #[must_use]
    pub fn millis_per_frame(millis: u64) -> Self {
        Self::PerFrame(Duration::from_millis(millis))
    }
}

/// Specifies what the animation does when it finishes.
#[derive(Clone, Copy, Debug, Default)]
pub enum PxAnimationFinishBehavior {
    /// The entity is despawned when the animation finishes.
    #[default]
    Despawn,
    /// [`PxAnimationFinished`] is added to the entity when the animation finishes.
    Mark,
    /// Backward-compatible alias for [`PxAnimationFinishBehavior::Mark`].
    Done,
    /// The animation loops when it finishes.
    Loop,
}

/// Animates an entity. Works on sprites, atlas sprites, filters, text, tilemaps, rectangles, and
/// lines.
#[derive(Component, Clone, Copy, Debug)]
#[require(PxFrameView, PxFrameControl, PxFrameCount)]
pub struct PxAnimation {
    /// A [`PxAnimationDirection`].
    pub direction: PxAnimationDirection,
    /// A [`PxAnimationDuration`].
    pub duration: PxAnimationDuration,
    /// A [`PxAnimationFinishBehavior`].
    pub on_finish: PxAnimationFinishBehavior,
    /// Time when the animation started.
    pub start: Instant,
}

impl Default for PxAnimation {
    fn default() -> Self {
        Self {
            direction: default(),
            duration: default(),
            on_finish: default(),
            start: Instant::now(),
        }
    }
}

/// Marks an animation that has finished. Automatically added to animations
/// with [`PxAnimationFinishBehavior::Mark`].
#[derive(Component, Debug)]
pub struct PxAnimationFinished;

pub(crate) trait AnimatedAssetComponent: Component {
    type Asset: Asset;

    fn handle(&self) -> &Handle<Self::Asset>;
    fn max_frame_count(asset: &Self::Asset) -> usize;
}

fn update_animations(
    mut cmd: Commands,
    time: Res<Time<Real>>,
    mut animations: Query<(
        Entity,
        &mut PxFrameControl,
        &PxAnimation,
        Has<PxAnimationFinished>,
        &PxFrameCount,
    )>,
) {
    for (id, mut control, animation, finished, frame_count) in &mut animations {
        let max_frame_count = frame_count.0;
        if max_frame_count == 0 {
            continue;
        }

        let elapsed = time.last_update().unwrap_or_else(|| time.startup()) - animation.start;
        let lifetime = match animation.duration {
            PxAnimationDuration::PerAnimation(duration) => duration,
            PxAnimationDuration::PerFrame(duration) => duration * max_frame_count as u32,
        };

        let ratio = elapsed.div_duration_f32(lifetime);
        let ratio = match animation.on_finish {
            PxAnimationFinishBehavior::Despawn
            | PxAnimationFinishBehavior::Mark
            | PxAnimationFinishBehavior::Done => ratio.min(1.),
            PxAnimationFinishBehavior::Loop => ratio.fract(),
        };
        let ratio = match animation.direction {
            PxAnimationDirection::Foreward => ratio,
            PxAnimationDirection::Backward => 1. + -ratio,
        };

        match control.selector {
            PxFrameSelector::Index(ref mut index) => *index = max_frame_count as f32 * ratio,
            PxFrameSelector::Normalized(ref mut normalized) => *normalized = ratio,
        }

        if elapsed >= lifetime {
            match animation.on_finish {
                PxAnimationFinishBehavior::Despawn => {
                    cmd.entity(id).despawn();
                }
                PxAnimationFinishBehavior::Mark | PxAnimationFinishBehavior::Done => {
                    if !finished {
                        cmd.entity(id).insert(PxAnimationFinished);
                    }
                }
                PxAnimationFinishBehavior::Loop => (),
            }
        }
    }
}

fn sync_frame_counts_on_component_change<A: AnimatedAssetComponent>(
    assets: Res<Assets<A::Asset>>,
    mut query: Query<
        (&A, &mut PxFrameCount),
        (With<PxAnimation>, Or<(Changed<A>, Added<PxAnimation>)>),
    >,
) {
    for (component, mut count) in &mut query {
        let Some(asset) = assets.get(component.handle()) else {
            count.0 = 0;
            continue;
        };
        count.0 = A::max_frame_count(asset);
    }
}

fn sync_frame_counts_on_atlas_sprite_change(
    atlases: Res<Assets<PxSpriteAtlasAsset>>,
    mut query: Query<
        (&PxAtlasSprite, &mut PxFrameCount),
        (
            With<PxAnimation>,
            Or<(Changed<PxAtlasSprite>, Added<PxAnimation>)>,
        ),
    >,
) {
    for (sprite, mut count) in &mut query {
        count.0 = atlas_region_frame_count(&atlases, sprite);
    }
}

fn sync_frame_counts_on_asset_event<A: AnimatedAssetComponent>(
    assets: Res<Assets<A::Asset>>,
    mut events: MessageReader<AssetEvent<A::Asset>>,
    mut query: Query<(&A, &mut PxFrameCount), With<PxAnimation>>,
) {
    if events.read().next().is_none() {
        return;
    }

    for (component, mut count) in &mut query {
        let Some(asset) = assets.get(component.handle()) else {
            count.0 = 0;
            continue;
        };
        count.0 = A::max_frame_count(asset);
    }
}

fn sync_frame_counts_on_atlas_asset_event(
    atlases: Res<Assets<PxSpriteAtlasAsset>>,
    mut events: MessageReader<AssetEvent<PxSpriteAtlasAsset>>,
    mut query: Query<(&PxAtlasSprite, &mut PxFrameCount), With<PxAnimation>>,
) {
    if events.read().next().is_none() {
        return;
    }

    for (sprite, mut count) in &mut query {
        count.0 = atlas_region_frame_count(&atlases, sprite);
    }
}

fn atlas_region_frame_count(atlases: &Assets<PxSpriteAtlasAsset>, sprite: &PxAtlasSprite) -> usize {
    let Some(atlas) = atlases.get(&sprite.atlas) else {
        return 0;
    };
    atlas
        .region(sprite.region)
        .map_or(0, super::atlas::AtlasRegion::frame_count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atlas::{AtlasRect, AtlasRegion, AtlasRegionId, PxSpriteAtlasAsset};
    use crate::image::PxImage;
    use crate::position::InsertDefaultLayer;
    use bevy_asset::{AssetId, uuid::Uuid};
    use bevy_ecs::{prelude::*, system::RunSystemOnce};
    use bevy_math::UVec2;
    use bevy_platform::collections::HashMap;

    fn make_atlas(frame_count: usize) -> PxSpriteAtlasAsset {
        let frames = (0..frame_count)
            .map(|i| AtlasRect {
                x: i as u32,
                y: 0,
                w: 1,
                h: 1,
            })
            .collect::<Vec<_>>();
        PxSpriteAtlasAsset {
            size: UVec2::new(frame_count as u32, 1),
            data: PxImage::new(vec![1u8; frame_count], frame_count),
            regions: vec![AtlasRegion {
                frame_size: UVec2::new(1, 1),
                frames,
            }],
            names: HashMap::default(),
        }
    }

    fn atlas_with_handle(
        assets: &mut Assets<PxSpriteAtlasAsset>,
        frame_count: usize,
    ) -> Handle<PxSpriteAtlasAsset> {
        let uuid = Uuid::new_v4();
        assets
            .insert(AssetId::Uuid { uuid }, make_atlas(frame_count))
            .unwrap();
        Handle::Uuid(uuid, std::marker::PhantomData)
    }

    // atlas_region_frame_count: pure helper logic

    #[test]
    fn frame_count_returns_zero_for_missing_atlas() {
        let atlases = Assets::<PxSpriteAtlasAsset>::default();
        let sprite = PxAtlasSprite::new(Handle::default(), AtlasRegionId(0));
        assert_eq!(atlas_region_frame_count(&atlases, &sprite), 0);
    }

    #[test]
    fn frame_count_returns_zero_for_missing_region() {
        let mut atlases = Assets::<PxSpriteAtlasAsset>::default();
        let handle = atlas_with_handle(&mut atlases, 3);
        // Region index 99 does not exist.
        let sprite = PxAtlasSprite::new(handle, AtlasRegionId(99));
        assert_eq!(atlas_region_frame_count(&atlases, &sprite), 0);
    }

    #[test]
    fn frame_count_returns_region_frame_count() {
        let mut atlases = Assets::<PxSpriteAtlasAsset>::default();
        let handle = atlas_with_handle(&mut atlases, 5);
        let sprite = PxAtlasSprite::new(handle, AtlasRegionId(0));
        assert_eq!(atlas_region_frame_count(&atlases, &sprite), 5);
    }

    // sync_frame_counts_on_atlas_sprite_change via World::run_system_once.
    //
    // PxAtlasSprite has #[require(DefaultLayer)] whose on_add hook calls
    // world.remove_resource::<InsertDefaultLayer>().unwrap(). We satisfy this by inserting
    // a no-op InsertDefaultLayer before spawning and flushing commands afterwards.

    fn setup_world() -> World {
        let mut world = World::new();
        // Satisfy the DefaultLayer component hook (it remove/re-inserts this resource).
        world.insert_resource(InsertDefaultLayer::noop());
        world
    }

    #[test]
    fn sync_on_sprite_change_sets_frame_count() {
        let mut world = setup_world();

        let mut atlases = Assets::<PxSpriteAtlasAsset>::default();
        let handle = atlas_with_handle(&mut atlases, 4);
        world.insert_resource(atlases);

        let entity = world
            .spawn((
                PxAtlasSprite::new(handle, AtlasRegionId(0)),
                PxFrameCount(0),
                PxAnimation::default(),
            ))
            .id();
        world.flush();

        world
            .run_system_once(sync_frame_counts_on_atlas_sprite_change)
            .unwrap();

        assert_eq!(world.get::<PxFrameCount>(entity).unwrap().0, 4);
    }

    #[test]
    fn sync_on_sprite_change_zeroes_count_when_atlas_missing() {
        let mut world = setup_world();
        world.insert_resource(Assets::<PxSpriteAtlasAsset>::default());

        let entity = world
            .spawn((
                PxAtlasSprite::new(Handle::default(), AtlasRegionId(0)),
                PxFrameCount(7),
                PxAnimation::default(),
            ))
            .id();
        world.flush();

        world
            .run_system_once(sync_frame_counts_on_atlas_sprite_change)
            .unwrap();

        assert_eq!(world.get::<PxFrameCount>(entity).unwrap().0, 0);
    }

    #[test]
    fn sync_on_asset_event_updates_frame_count() {
        let mut world = setup_world();

        let mut atlases = Assets::<PxSpriteAtlasAsset>::default();
        let handle = atlas_with_handle(&mut atlases, 3);
        world.insert_resource(atlases);
        world.init_resource::<Messages<AssetEvent<PxSpriteAtlasAsset>>>();

        let entity = world
            .spawn((
                PxAtlasSprite::new(handle, AtlasRegionId(0)),
                PxFrameCount(0),
                PxAnimation::default(),
            ))
            .id();
        world.flush();

        // Send a fake asset event so the system sees something to process.
        world
            .resource_mut::<Messages<AssetEvent<PxSpriteAtlasAsset>>>()
            .write(AssetEvent::LoadedWithDependencies {
                id: AssetId::default(),
            });

        world
            .run_system_once(sync_frame_counts_on_atlas_asset_event)
            .unwrap();

        assert_eq!(world.get::<PxFrameCount>(entity).unwrap().0, 3);
    }
}
