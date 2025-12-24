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
            sync_frame_counts_on_component_change::<PxFilter>,
            sync_frame_counts_on_component_change::<PxText>,
            sync_frame_counts_on_component_change::<PxMap>,
            sync_frame_counts_on_asset_event::<PxSprite>,
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
    pub fn millis_per_animation(millis: u64) -> Self {
        Self::PerAnimation(Duration::from_millis(millis))
    }

    /// Creates a [`PxAnimationDuration::PerFrame`] with the given number of milliseconds.
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
    /// A successful [`Done`] is added to the entity when the animation finishes.
    #[cfg(feature = "state")]
    Done,
    /// The animation loops when it finishes.
    Loop,
}

/// Animates an entity. Works on sprites, filters, text, tilemaps, rectangles, and lines.
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
            PxAnimationFinishBehavior::Despawn | PxAnimationFinishBehavior::Mark => ratio.min(1.),
            #[cfg(feature = "state")]
            PxAnimationFinishBehavior::Done => ratio.min(1.),
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
                PxAnimationFinishBehavior::Mark => {
                    if !finished {
                        cmd.entity(id).insert(PxAnimationFinished);
                    }
                }
                #[cfg(feature = "state")]
                PxAnimationFinishBehavior::Done => {
                    cmd.entity(id).insert(Done::Success);
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
