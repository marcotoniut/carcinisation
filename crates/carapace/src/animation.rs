//! Animation
//!
//! Optional animation systems that drive [`CxFrameControl`] updates.

use std::time::Duration;

use bevy_platform::time::Instant;

pub use crate::frame::{
    CxFrameControl, CxFrameCount, CxFrameSelector, CxFrameTransition, CxFrameView,
};
use bevy_asset::AssetEvent;

use crate::{prelude::*, set::CxSet};

/// Optional plugin that installs the default animation systems.
#[derive(Default)]
pub struct CxAnimationPlugin;

impl Plugin for CxAnimationPlugin {
    fn build(&self, app: &mut App) {
        plug(app);
    }
}

pub(crate) fn plug(app: &mut App) {
    app.add_systems(
        PostUpdate,
        (
            sync_frame_counts_on_component_change::<CxSprite>,
            sync_frame_counts_on_atlas_sprite_change,
            sync_frame_counts_on_component_change::<CxFilter>,
            sync_frame_counts_on_component_change::<CxText>,
            sync_frame_counts_on_component_change::<CxTilemap>,
            sync_frame_counts_on_asset_event::<CxSprite>,
            sync_frame_counts_on_atlas_asset_event,
            sync_frame_counts_on_asset_event::<CxFilter>,
            sync_frame_counts_on_asset_event::<CxText>,
            sync_frame_counts_on_asset_event::<CxTilemap>,
            update_animations,
        )
            .chain()
            .in_set(CxSet::FinishAnimations),
    );
}

/// Direction the animation plays.
#[derive(Clone, Copy, Debug, Default, Reflect)]
pub enum CxAnimationDirection {
    /// The animation plays forward.
    #[default]
    Forward,
    /// The animation plays backward.
    Backward,
}

impl CxAnimationDirection {
    /// Deprecated alias for [`Forward`](Self::Forward). Use `Forward` instead.
    #[deprecated(since = "0.9.0", note = "Typo — use `Forward` instead.")]
    pub const FOREWARD: Self = Self::Forward;
}

/// Animation duration.
#[derive(Clone, Copy, Debug, Reflect)]
pub enum CxAnimationDuration {
    /// Duration of the entire animation. When used on a tilemap, each tile's animation
    /// takes the same amount of time, but their frames may desync.
    PerAnimation(Duration),
    /// Duration of each frame. When used on a tilemap, each frame will take the same amount
    /// of time, but the tile's animations may desync.
    PerFrame(Duration),
}

impl Default for CxAnimationDuration {
    fn default() -> Self {
        Self::PerAnimation(Duration::from_secs(1))
    }
}

impl CxAnimationDuration {
    /// Creates a [`CxAnimationDuration::PerAnimation`] with the given number of milliseconds.
    #[must_use]
    pub fn millis_per_animation(millis: u64) -> Self {
        Self::PerAnimation(Duration::from_millis(millis))
    }

    /// Creates a [`CxAnimationDuration::PerFrame`] with the given number of milliseconds.
    #[must_use]
    pub fn millis_per_frame(millis: u64) -> Self {
        Self::PerFrame(Duration::from_millis(millis))
    }
}

/// Specifies what the animation does when it finishes.
#[derive(Clone, Copy, Debug, Default, Reflect)]
pub enum CxAnimationFinishBehavior {
    /// The entity is despawned when the animation finishes.
    #[default]
    Despawn,
    /// [`CxAnimationFinished`] is added to the entity when the animation finishes.
    Mark,
    /// Backward-compatible alias for [`CxAnimationFinishBehavior::Mark`].
    Done,
    /// The animation loops when it finishes.
    Loop,
}

/// Animates an entity. Works on sprites, atlas sprites, filters, text, tilemaps, rectangles, and
/// lines.
#[derive(Component, Clone, Copy, Debug)]
#[require(CxFrameView, CxFrameControl, CxFrameCount)]
pub struct CxAnimation {
    /// A [`CxAnimationDirection`].
    pub direction: CxAnimationDirection,
    /// A [`CxAnimationDuration`].
    pub duration: CxAnimationDuration,
    /// A [`CxAnimationFinishBehavior`].
    pub on_finish: CxAnimationFinishBehavior,
    /// Time when the animation started.
    pub start: Instant,
}

impl Default for CxAnimation {
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
/// with [`CxAnimationFinishBehavior::Mark`].
#[derive(Component, Debug, Reflect)]
pub struct CxAnimationFinished;

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
        &mut CxFrameControl,
        &CxAnimation,
        Has<CxAnimationFinished>,
        &CxFrameCount,
    )>,
) {
    for (id, mut control, animation, finished, frame_count) in &mut animations {
        let max_frame_count = frame_count.0;
        if max_frame_count == 0 {
            continue;
        }

        let elapsed = time.last_update().unwrap_or_else(|| time.startup()) - animation.start;
        let lifetime = match animation.duration {
            CxAnimationDuration::PerAnimation(duration) => duration,
            CxAnimationDuration::PerFrame(duration) => duration * max_frame_count as u32,
        };

        let ratio = elapsed.div_duration_f32(lifetime);
        let ratio = match animation.on_finish {
            CxAnimationFinishBehavior::Despawn
            | CxAnimationFinishBehavior::Mark
            | CxAnimationFinishBehavior::Done => ratio.min(1.),
            CxAnimationFinishBehavior::Loop => ratio.fract(),
        };
        let ratio = match animation.direction {
            CxAnimationDirection::Forward => ratio,
            CxAnimationDirection::Backward => 1. + -ratio,
        };

        match control.selector {
            CxFrameSelector::Index(ref mut index) => *index = max_frame_count as f32 * ratio,
            CxFrameSelector::Normalized(ref mut normalized) => *normalized = ratio,
        }

        if elapsed >= lifetime {
            match animation.on_finish {
                CxAnimationFinishBehavior::Despawn => {
                    cmd.entity(id).despawn();
                }
                CxAnimationFinishBehavior::Mark | CxAnimationFinishBehavior::Done => {
                    if !finished {
                        cmd.entity(id).insert(CxAnimationFinished);
                    }
                }
                CxAnimationFinishBehavior::Loop => (),
            }
        }
    }
}

fn sync_frame_counts_on_component_change<A: AnimatedAssetComponent>(
    assets: Res<Assets<A::Asset>>,
    mut query: Query<
        (&A, &mut CxFrameCount),
        (With<CxAnimation>, Or<(Changed<A>, Added<CxAnimation>)>),
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
    atlases: Res<Assets<CxSpriteAtlasAsset>>,
    mut query: Query<
        (&CxAtlasSprite, &mut CxFrameCount),
        (
            With<CxAnimation>,
            Or<(Changed<CxAtlasSprite>, Added<CxAnimation>)>,
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
    mut query: Query<(&A, &mut CxFrameCount), With<CxAnimation>>,
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
    atlases: Res<Assets<CxSpriteAtlasAsset>>,
    mut events: MessageReader<AssetEvent<CxSpriteAtlasAsset>>,
    mut query: Query<(&CxAtlasSprite, &mut CxFrameCount), With<CxAnimation>>,
) {
    if events.read().next().is_none() {
        return;
    }

    for (sprite, mut count) in &mut query {
        count.0 = atlas_region_frame_count(&atlases, sprite);
    }
}

fn atlas_region_frame_count(atlases: &Assets<CxSpriteAtlasAsset>, sprite: &CxAtlasSprite) -> usize {
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
    use crate::atlas::{AtlasRect, AtlasRegion, AtlasRegionId, CxSpriteAtlasAsset};
    use crate::image::CxImage;
    use crate::position::InsertDefaultLayer;
    use bevy_asset::{AssetId, uuid::Uuid};
    use bevy_ecs::{prelude::*, system::RunSystemOnce};
    use bevy_math::UVec2;
    use bevy_platform::collections::HashMap;

    fn make_atlas(frame_count: usize) -> CxSpriteAtlasAsset {
        let frames = (0..frame_count)
            .map(|i| AtlasRect {
                x: i as u32,
                y: 0,
                w: 1,
                h: 1,
            })
            .collect::<Vec<_>>();
        CxSpriteAtlasAsset {
            size: UVec2::new(frame_count as u32, 1),
            data: CxImage::new(vec![1u8; frame_count], frame_count),
            regions: vec![AtlasRegion {
                frame_size: UVec2::new(1, 1),
                frames,
            }],
            names: HashMap::default(),
            animations: HashMap::default(),
        }
    }

    fn atlas_with_handle(
        assets: &mut Assets<CxSpriteAtlasAsset>,
        frame_count: usize,
    ) -> Handle<CxSpriteAtlasAsset> {
        let uuid = Uuid::new_v4();
        assets
            .insert(AssetId::Uuid { uuid }, make_atlas(frame_count))
            .unwrap();
        Handle::Uuid(uuid, std::marker::PhantomData)
    }

    // atlas_region_frame_count: pure helper logic

    #[test]
    fn frame_count_returns_zero_for_missing_atlas() {
        let atlases = Assets::<CxSpriteAtlasAsset>::default();
        let sprite = CxAtlasSprite::new(Handle::default(), AtlasRegionId(0));
        assert_eq!(atlas_region_frame_count(&atlases, &sprite), 0);
    }

    #[test]
    fn frame_count_returns_zero_for_missing_region() {
        let mut atlases = Assets::<CxSpriteAtlasAsset>::default();
        let handle = atlas_with_handle(&mut atlases, 3);
        // Region index 99 does not exist.
        let sprite = CxAtlasSprite::new(handle, AtlasRegionId(99));
        assert_eq!(atlas_region_frame_count(&atlases, &sprite), 0);
    }

    #[test]
    fn frame_count_returns_region_frame_count() {
        let mut atlases = Assets::<CxSpriteAtlasAsset>::default();
        let handle = atlas_with_handle(&mut atlases, 5);
        let sprite = CxAtlasSprite::new(handle, AtlasRegionId(0));
        assert_eq!(atlas_region_frame_count(&atlases, &sprite), 5);
    }

    // sync_frame_counts_on_atlas_sprite_change via World::run_system_once.
    //
    // CxAtlasSprite has #[require(DefaultLayer)] whose on_add hook calls
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

        let mut atlases = Assets::<CxSpriteAtlasAsset>::default();
        let handle = atlas_with_handle(&mut atlases, 4);
        world.insert_resource(atlases);

        let entity = world
            .spawn((
                CxAtlasSprite::new(handle, AtlasRegionId(0)),
                CxFrameCount(0),
                CxAnimation::default(),
            ))
            .id();
        world.flush();

        world
            .run_system_once(sync_frame_counts_on_atlas_sprite_change)
            .unwrap();

        assert_eq!(world.get::<CxFrameCount>(entity).unwrap().0, 4);
    }

    #[test]
    fn sync_on_sprite_change_zeroes_count_when_atlas_missing() {
        let mut world = setup_world();
        world.insert_resource(Assets::<CxSpriteAtlasAsset>::default());

        let entity = world
            .spawn((
                CxAtlasSprite::new(Handle::default(), AtlasRegionId(0)),
                CxFrameCount(7),
                CxAnimation::default(),
            ))
            .id();
        world.flush();

        world
            .run_system_once(sync_frame_counts_on_atlas_sprite_change)
            .unwrap();

        assert_eq!(world.get::<CxFrameCount>(entity).unwrap().0, 0);
    }

    #[test]
    fn sync_on_asset_event_updates_frame_count() {
        let mut world = setup_world();

        let mut atlases = Assets::<CxSpriteAtlasAsset>::default();
        let handle = atlas_with_handle(&mut atlases, 3);
        world.insert_resource(atlases);
        world.init_resource::<Messages<AssetEvent<CxSpriteAtlasAsset>>>();

        let entity = world
            .spawn((
                CxAtlasSprite::new(handle, AtlasRegionId(0)),
                CxFrameCount(0),
                CxAnimation::default(),
            ))
            .id();
        world.flush();

        // Send a fake asset event so the system sees something to process.
        world
            .resource_mut::<Messages<AssetEvent<CxSpriteAtlasAsset>>>()
            .write(AssetEvent::LoadedWithDependencies {
                id: AssetId::default(),
            });

        world
            .run_system_once(sync_frame_counts_on_atlas_asset_event)
            .unwrap();

        assert_eq!(world.get::<CxFrameCount>(entity).unwrap().0, 3);
    }
}
