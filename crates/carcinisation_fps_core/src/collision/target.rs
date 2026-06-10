//! Per-part, per-facing target collision metadata.
//!
//! Provides the metadata layer between 2D collision primitives and gameplay
//! damage routing. Collision data is indexed by animation key, frame index,
//! and billboard facing — the facing is selected per attacker/query so two
//! players may hit-test the same enemy using different facings in the same
//! server tick.
//!
//! Geometry ([`PartCollider2d`]) is separate from gameplay metadata
//! ([`PartMetadata`]) so the collision kernel stays rendering-independent.

use std::collections::HashMap;

use bevy_math::Vec2;

use super::nearest;
use super::primitives::{Collider, HitResult};

// ---------------------------------------------------------------------------
// Lightweight identifiers
// ---------------------------------------------------------------------------

/// Opaque part identifier assigned by the authoring pipeline.
///
/// Numerically ordered for deterministic tie-breaking in nearest-hit queries.
///
/// This compact `u16` is the **hot-path and wire-safe** identity used by every
/// collision/damage query. Source assets (e.g. ORS compositions) identify parts
/// by `String` name; those names are mapped to `PartId` once, up front, via an
/// authored [`PartIdRegistry`] — never compared on the hot path or sent over the
/// network. See [`PartIdRegistry`] for the stability contract.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PartId(pub u16);

impl PartId {
    /// Sentinel part id used when a target has no authored collision frame and
    /// the query falls back to a single whole-body circle. Reserved: a
    /// [`PartIdRegistry`] rejects any attempt to map a name to this value.
    pub const FALLBACK: Self = Self(u16::MAX);
}

/// Opaque material identifier.
///
/// **Reserved / frozen (Phase 6 conclusion).** `MaterialId` is an FPS-local
/// concept with **no ORS source** — ORS has no material/reaction system (it
/// routes via part `tags` + `health_pool`). It is carried through
/// `PartMetadata`/`PartHitscanResult`/`FlamePartHit` as a placeholder but has
/// **no consumer yet**: do not expand it (no material tables, no reactions)
/// until an FPS-local material/reaction system actually reads it. It is
/// intentionally not produced by any future ORS importer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MaterialId(pub u16);

/// Lightweight animation identifier.
///
/// The caller maps from their domain-specific animation concept (e.g.
/// `EnemyPresentationState`) to this key. The collision system only stores
/// and looks up by key — it does not interpret the value.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AnimationKey(pub u16);

// ---------------------------------------------------------------------------
// 8-direction facing
// ---------------------------------------------------------------------------

/// Geometric 8-direction facing octant of an attacker relative to a target,
/// used to select per-facing collision frames.
///
/// # Semantics: geometric, not renderer asset keys
///
/// Each variant names the side of the target the attacker is on (and
/// therefore sees): `Front` means the attacker is in the direction the target
/// faces. In this engine's map space, "right" is **clockwise** from forward
/// (`Camera::plane`: facing east, right is south), so a **positive** relative
/// angle (counter-clockwise; `+Y` for a `+X`-facing target) is the target's
/// **left** side:
///
/// | relative angle | facing       |
/// |---------------:|--------------|
/// |   0°           | `Front`      |
/// |  +45°          | `FrontLeft`  |
/// |  +90°          | `Left`       |
/// | +135°          | `BackLeft`   |
/// |  180°          | `Back`       |
/// | +225°          | `BackRight`  |
/// | +270°          | `Right`      |
/// | +315°          | `FrontRight` |
///
/// Bucket boundaries and indices match the directional billboard renderer's
/// quantization exactly (same sector math, same index per angle). Only the
/// **names** differ for mirrored buckets: the renderer labels buckets by
/// *sprite-asset key*, where left/right side assets are derived via `flip_x`
/// (e.g. the renderer's +45° bucket is keyed `FrontRight`, this enum's
/// geometric `FrontLeft`). Never match the two by name — convert by bucket
/// index via `sprite_direction_for_facing` in `carcinisation_fps`, or reflect
/// with [`Self::mirrored`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum BillboardFacing8 {
    Front = 0,
    FrontLeft = 1,
    Left = 2,
    BackLeft = 3,
    Back = 4,
    BackRight = 5,
    Right = 6,
    FrontRight = 7,
}

impl BillboardFacing8 {
    /// All eight facings in quantization-bucket order starting from `Front`
    /// (relative angle increasing counter-clockwise, i.e. toward the target's
    /// left).
    pub const ALL: [Self; 8] = [
        Self::Front,
        Self::FrontLeft,
        Self::Left,
        Self::BackLeft,
        Self::Back,
        Self::BackRight,
        Self::Right,
        Self::FrontRight,
    ];

    /// Number of facing directions.
    pub const COUNT: usize = 8;

    /// Sector width in radians (τ/8 = π/4).
    const SECTOR: f32 = std::f32::consts::TAU / 8.0;

    /// Quantize a relative viewing angle into one of 8 facing buckets.
    ///
    /// `angle` is in radians counter-clockwise from the target's forward
    /// direction; positive angles are toward the target's **left** (see the
    /// type-level table). Bucket boundaries match the directional billboard
    /// renderer's `quantize_direction`. Values outside `[0, τ)` are wrapped.
    #[must_use]
    pub fn from_relative_angle(angle: f32) -> Self {
        let half = Self::SECTOR * 0.5;
        let tau = std::f32::consts::TAU;
        // Offset by half-sector so bucket centres align with enum values,
        // then wrap to [0, τ) and divide into sectors.
        let wrapped = (angle + half).rem_euclid(tau);
        let index = (wrapped / Self::SECTOR) as usize % Self::COUNT;
        Self::ALL[index]
    }

    /// Determine facing from world positions and target yaw.
    ///
    /// `target_pos` and `target_yaw` describe the target's 2D pose.
    /// `attacker_pos` is the observer/attacker position used to select
    /// the billboard facing.
    #[must_use]
    pub fn from_positions(target_pos: Vec2, target_yaw: f32, attacker_pos: Vec2) -> Self {
        let delta = attacker_pos - target_pos;
        if delta.length_squared() < f32::EPSILON * f32::EPSILON {
            return Self::Front;
        }
        let angle_to_attacker = delta.y.atan2(delta.x);
        Self::from_relative_angle(angle_to_attacker - target_yaw)
    }

    /// Convert to index `0..8`.
    #[must_use]
    pub const fn index(self) -> usize {
        self as usize
    }

    /// The left/right mirrored facing — reflection across the target's
    /// forward axis: `FrontLeft` ↔ `FrontRight`, `Left` ↔ `Right`,
    /// `BackLeft` ↔ `BackRight`; `Front` and `Back` map to themselves.
    ///
    /// Useful when collision geometry for one side is derived from the other
    /// (the collision analogue of the renderer's `flip_x` sprite mirroring).
    #[must_use]
    pub const fn mirrored(self) -> Self {
        match self {
            Self::Front => Self::Front,
            Self::FrontLeft => Self::FrontRight,
            Self::Left => Self::Right,
            Self::BackLeft => Self::BackRight,
            Self::Back => Self::Back,
            Self::BackRight => Self::BackLeft,
            Self::Right => Self::Left,
            Self::FrontRight => Self::FrontLeft,
        }
    }
}

// ---------------------------------------------------------------------------
// Query pose
// ---------------------------------------------------------------------------

/// 2D target pose for collision queries.
///
/// Contains the target's position and yaw — no visual pitch, no rendering
/// state. The facing for a specific attacker is computed on demand.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TargetQueryPose2d {
    pub position: Vec2,
    pub yaw: f32,
}

impl TargetQueryPose2d {
    #[must_use]
    pub const fn new(position: Vec2, yaw: f32) -> Self {
        Self { position, yaw }
    }

    /// Compute the billboard facing an attacker would see.
    #[must_use]
    pub fn facing_for_attacker(&self, attacker_pos: Vec2) -> BillboardFacing8 {
        BillboardFacing8::from_positions(self.position, self.yaw, attacker_pos)
    }
}

// ---------------------------------------------------------------------------
// Part collider (geometry)
// ---------------------------------------------------------------------------

/// A collision primitive tagged with a part identifier.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PartCollider2d {
    pub part_id: PartId,
    pub collider: Collider,
}

// ---------------------------------------------------------------------------
// Part metadata (gameplay, separate from geometry)
// ---------------------------------------------------------------------------

/// Static authored gameplay metadata for a collision part.
///
/// Kept separate from [`PartCollider2d`] so the collision kernel does not
/// depend on damage routing rules.
///
/// # Static only — no runtime state here
///
/// This is **immutable authored data**, stored once in a shared
/// [`TargetCollisionSet`] (typically `&'static`). Per-instance, mutable runtime
/// state — current durability, broken/disabled flags, exposed weak points —
/// must **not** live here; it belongs in [`PartRuntimeState`], keyed per
/// `(entity, PartId)` and owned server-authoritatively (mirroring the ORS split
/// of `CachedPartGameplay` vs `PartGameplayState`). When ORS-authored fields
/// (`targetable`, `armour`, `durability_max`, `breakable`, `health_pool`) are
/// adopted, the *static* ones extend this struct; the *mutable* ones do not.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PartMetadata {
    /// Reserved FPS-local material id — see [`MaterialId`] (no ORS source yet).
    pub material: MaterialId,
    /// Per-part damage multiplier applied via `scaled_damage`.
    pub damage_scale: f32,
}

// ---------------------------------------------------------------------------
// Runtime per-part state (placeholder — NOT wired into gameplay)
// ---------------------------------------------------------------------------

/// Mutable per-part runtime state for a live target instance.
///
/// **Placeholder for the static/runtime boundary (Phase 7); not wired into any
/// gameplay system yet.** It documents where future per-part mutable state will
/// live so it never leaks into the static [`PartMetadata`] / [`TargetCollisionSet`].
///
/// # Ownership contract (when adopted)
///
/// - **Server-authoritative.** The server owns the truth; clients never mutate it.
/// - **Keyed per `(entity, PartId)`** — e.g. a Bevy component holding a
///   `HashMap<PartId, PartRuntimeState>` per enemy entity (the ORS analogue is
///   `ComposedPartStates`).
/// - **Replicate only what visuals need** (e.g. a coarse `broken` flag), never
///   the full durability bookkeeping.
/// - Single-player owns its own copy locally (it is its own authority).
///
/// Defaults are safe and inert: full durability unknown (`None`), not broken,
/// collision enabled — i.e. behaves exactly like today's stateless parts.
/// `Default` is hand-written (not derived) because the derived `bool` default
/// would leave `collision_enabled = false`, which would disable the part.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PartRuntimeState {
    /// Remaining durability, or `None` when the part has no durability pool.
    pub current_durability: Option<u32>,
    /// Whether the part has broken off (becomes non-targetable when adopted).
    pub broken: bool,
    /// Whether this part currently participates in collision queries.
    pub collision_enabled: bool,
}

impl PartRuntimeState {
    /// Inert default: no durability tracking, not broken, collision enabled —
    /// equivalent to the current stateless behaviour.
    #[must_use]
    pub const fn fresh() -> Self {
        Self {
            current_durability: None,
            broken: false,
            collision_enabled: true,
        }
    }
}

impl Default for PartRuntimeState {
    fn default() -> Self {
        Self::fresh()
    }
}

// ---------------------------------------------------------------------------
// Collision frame
// ---------------------------------------------------------------------------

/// Collision data for a single animation frame at a specific facing.
///
/// Stores part colliders in a layout optimised for [`nearest_ray_hit_tagged`]
/// and [`nearest_segment_hit_tagged`] queries.
///
/// [`nearest_ray_hit_tagged`]: super::nearest::nearest_ray_hit_tagged
/// [`nearest_segment_hit_tagged`]: super::nearest::nearest_segment_hit_tagged
#[derive(Clone, Debug, Default)]
pub struct TargetCollisionFrame {
    tagged: Vec<(Collider, PartId)>,
}

impl TargetCollisionFrame {
    /// Build a frame from an iterator of part colliders.
    pub fn new(parts: impl IntoIterator<Item = PartCollider2d>) -> Self {
        Self {
            tagged: parts.into_iter().map(|p| (p.collider, p.part_id)).collect(),
        }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tagged.is_empty()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.tagged.len()
    }

    /// Iterate over the part colliders in this frame.
    pub fn parts(&self) -> impl Iterator<Item = PartCollider2d> + '_ {
        self.tagged.iter().map(|(c, id)| PartCollider2d {
            part_id: *id,
            collider: *c,
        })
    }

    /// Nearest ray hit among this frame's parts.
    ///
    /// Returns a [`HitResult`] tagged with the [`PartId`] of the hit
    /// collider. Equal-distance ties are broken by smaller `PartId`.
    #[must_use]
    pub fn nearest_ray_hit(&self, origin: Vec2, direction: Vec2) -> Option<HitResult<PartId>> {
        nearest::nearest_ray_hit_tagged(origin, direction, &self.tagged)
    }

    /// Nearest segment hit among this frame's parts.
    #[must_use]
    pub fn nearest_segment_hit(&self, start: Vec2, end: Vec2) -> Option<HitResult<PartId>> {
        nearest::nearest_segment_hit_tagged(start, end, &self.tagged)
    }

    /// Nearest ray hit for a target placed at `target` in world space.
    ///
    /// Part colliders are authored in target-local space where local `+X` is
    /// the target forward (yaw) direction and local `+Y` is 90° CCW from it.
    /// The world ray is transformed into that local frame, queried, and the
    /// resulting hit point/normal transformed back to world space. Distance is
    /// rotation-invariant, so it is directly comparable across targets and
    /// against wall obstruction.
    #[must_use]
    pub fn nearest_world_ray_hit(
        &self,
        target: TargetQueryPose2d,
        world_origin: Vec2,
        world_dir: Vec2,
    ) -> Option<HitResult<PartId>> {
        let (sin, cos) = target.yaw.sin_cos();
        // Rotate a world vector into local space by -yaw.
        let rot_in = |v: Vec2| Vec2::new(v.x * cos + v.y * sin, v.y * cos - v.x * sin);
        // Rotate a local vector back into world space by +yaw.
        let rot_out = |v: Vec2| Vec2::new(v.x * cos - v.y * sin, v.x * sin + v.y * cos);

        let local_origin = rot_in(world_origin - target.position);
        let local_dir = rot_in(world_dir);

        let hit = self.nearest_ray_hit(local_origin, local_dir)?;
        Some(HitResult {
            point: target.position + rot_out(hit.point),
            normal: rot_out(hit.normal),
            distance: hit.distance,
            id: hit.id,
        })
    }

    /// Nearest swept-circle hit for a target placed at `target` in world space.
    ///
    /// A circle of `sweep_radius` is swept along the world segment
    /// `world_start`–`world_end` (e.g. a flamethrower strip of half-width
    /// `sweep_radius`). The segment is transformed into the target-local frame,
    /// queried against the part colliders, and the resulting hit point/normal
    /// transformed back to world space. Distance is along the swept centre path
    /// and is comparable across targets.
    #[must_use]
    pub fn nearest_world_swept_hit(
        &self,
        target: TargetQueryPose2d,
        world_start: Vec2,
        world_end: Vec2,
        sweep_radius: f32,
    ) -> Option<HitResult<PartId>> {
        let (sin, cos) = target.yaw.sin_cos();
        let rot_in = |v: Vec2| Vec2::new(v.x * cos + v.y * sin, v.y * cos - v.x * sin);
        let rot_out = |v: Vec2| Vec2::new(v.x * cos - v.y * sin, v.x * sin + v.y * cos);

        let local_start = rot_in(world_start - target.position);
        let local_end = rot_in(world_end - target.position);

        let hit = nearest::nearest_swept_circle_hit_tagged(
            local_start,
            local_end,
            sweep_radius,
            &self.tagged,
        )?;
        Some(HitResult {
            point: target.position + rot_out(hit.point),
            normal: rot_out(hit.normal),
            distance: hit.distance,
            id: hit.id,
        })
    }
}

// ---------------------------------------------------------------------------
// Collision set (per target type)
// ---------------------------------------------------------------------------

/// Lookup key for a specific collision frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CollisionFrameKey {
    pub animation: AnimationKey,
    pub frame: u16,
    pub facing: BillboardFacing8,
}

/// Complete collision data for one target type (e.g. one enemy variant).
///
/// Indexed by ([`AnimationKey`], frame index, [`BillboardFacing8`]).
/// Part metadata is stored separately and looked up by [`PartId`].
///
/// **Static authored data only.** This set is built once and shared (typically
/// `&'static`); it holds geometry + [`PartMetadata`]. Mutable per-instance
/// runtime state lives in [`PartRuntimeState`] keyed per `(entity, PartId)`,
/// never here.
#[derive(Clone, Debug, Default)]
pub struct TargetCollisionSet {
    frames: HashMap<CollisionFrameKey, TargetCollisionFrame>,
    part_metadata: HashMap<PartId, PartMetadata>,
}

impl TargetCollisionSet {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert collision data for a specific frame/facing combination.
    pub fn insert_frame(&mut self, key: CollisionFrameKey, frame: TargetCollisionFrame) {
        self.frames.insert(key, frame);
    }

    /// Register gameplay metadata for a part.
    pub fn insert_part_metadata(&mut self, part_id: PartId, meta: PartMetadata) {
        self.part_metadata.insert(part_id, meta);
    }

    /// Look up a collision frame by composite key.
    #[must_use]
    pub fn frame(&self, key: &CollisionFrameKey) -> Option<&TargetCollisionFrame> {
        self.frames.get(key)
    }

    /// Look up a collision frame by individual components.
    #[must_use]
    pub fn lookup(
        &self,
        animation: AnimationKey,
        frame: u16,
        facing: BillboardFacing8,
    ) -> Option<&TargetCollisionFrame> {
        self.frame(&CollisionFrameKey {
            animation,
            frame,
            facing,
        })
    }

    /// Retrieve gameplay metadata for a part.
    #[must_use]
    pub fn part_metadata(&self, part_id: PartId) -> Option<&PartMetadata> {
        self.part_metadata.get(&part_id)
    }

    /// Number of registered frames.
    #[must_use]
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }
}

// ---------------------------------------------------------------------------
// Part id registry (source name <-> compact PartId)
// ---------------------------------------------------------------------------

/// Error returned when building a [`PartIdRegistry`] from an invalid table.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum PartIdRegistryError {
    /// Two entries share the same source name.
    #[error("duplicate part name in registry table: {0:?}")]
    DuplicateName(String),
    /// Two entries share the same [`PartId`].
    #[error("duplicate part id in registry table: {0:?}")]
    DuplicateId(PartId),
    /// A table entry uses the reserved [`PartId::FALLBACK`] sentinel.
    #[error("registry table uses the reserved PartId::FALLBACK sentinel")]
    ReservedId,
}

/// Bidirectional map between source part names (e.g. ORS `String` ids) and the
/// compact [`PartId`] used on hot paths.
///
/// Names are stored **owned**, so the registry accepts runtime/asset-loaded
/// strings (an ORS importer hands over `String` part ids directly) as well as
/// compile-time tables. Both lookup directions are off the hot path — gameplay
/// carries `PartId` only.
///
/// # Contract
///
/// - Built from an **explicit authored table** of `(name, PartId)` pairs — not
///   from hashing and not from insertion order — so ids stay **stable across
///   builds, renames, and reorders**. This is the wire/save-safe identity.
/// - The mapping is 1:1: duplicate names or ids are rejected at build time, as
///   is the reserved [`PartId::FALLBACK`] sentinel.
/// - Name → id is the import-time direction; the reverse (`name_of`) is for
///   debugging/logging only. Neither is required on the hot path — gameplay
///   uses `PartId` directly.
#[derive(Clone, Debug, Default)]
pub struct PartIdRegistry {
    by_name: HashMap<String, PartId>,
    by_id: HashMap<PartId, String>,
}

impl PartIdRegistry {
    /// Build a registry from `(name, id)` entries with any string-like name
    /// (`&str`, `String`, …) — the form an asset-loaded ORS import produces.
    ///
    /// # Errors
    ///
    /// Returns [`PartIdRegistryError`] if any name or id repeats, or if any
    /// entry uses the reserved [`PartId::FALLBACK`].
    pub fn from_entries<I, S>(entries: I) -> Result<Self, PartIdRegistryError>
    where
        I: IntoIterator<Item = (S, PartId)>,
        S: Into<String>,
    {
        let mut by_name: HashMap<String, PartId> = HashMap::new();
        let mut by_id: HashMap<PartId, String> = HashMap::new();
        for (name, id) in entries {
            let name = name.into();
            if id == PartId::FALLBACK {
                return Err(PartIdRegistryError::ReservedId);
            }
            if by_name.contains_key(&name) {
                return Err(PartIdRegistryError::DuplicateName(name));
            }
            if by_id.contains_key(&id) {
                return Err(PartIdRegistryError::DuplicateId(id));
            }
            by_id.insert(id, name.clone());
            by_name.insert(name, id);
        }
        Ok(Self { by_name, by_id })
    }

    /// Build a registry from an authored static `(name, id)` table.
    /// Convenience wrapper over [`Self::from_entries`].
    ///
    /// # Errors
    ///
    /// Returns [`PartIdRegistryError`] if any name or id repeats, or if any
    /// entry uses the reserved [`PartId::FALLBACK`].
    pub fn from_table(entries: &[(&str, PartId)]) -> Result<Self, PartIdRegistryError> {
        Self::from_entries(entries.iter().map(|&(name, id)| (name, id)))
    }

    /// Map a source part name to its [`PartId`], or `None` if unregistered.
    #[must_use]
    pub fn id_of(&self, name: &str) -> Option<PartId> {
        self.by_name.get(name).copied()
    }

    /// Reverse lookup: the source name for a [`PartId`] (debug/logging only).
    #[must_use]
    pub fn name_of(&self, id: PartId) -> Option<&str> {
        self.by_id.get(&id).map(String::as_str)
    }

    /// Number of registered parts.
    #[must_use]
    pub fn len(&self) -> usize {
        self.by_name.len()
    }

    /// Whether the registry has no entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_name.is_empty()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::collision::primitives::Circle;
    use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, PI, TAU};

    const EPS: f32 = 1e-4;

    fn approx(a: f32, b: f32) -> bool {
        (a - b).abs() < EPS
    }

    // --- BillboardFacing8 ---

    #[test]
    fn facing_sector_centres_are_deterministic() {
        // Geometric octants: positive relative angle (CCW) is the target's
        // LEFT side. Bucket indices (not names) match the renderer's
        // quantize_direction — see the parity test in directional_billboard.
        let expected = [
            (0.0, BillboardFacing8::Front),
            (FRAC_PI_4, BillboardFacing8::FrontLeft),
            (FRAC_PI_2, BillboardFacing8::Left),
            (FRAC_PI_2 + FRAC_PI_4, BillboardFacing8::BackLeft),
            (PI, BillboardFacing8::Back),
            (PI + FRAC_PI_4, BillboardFacing8::BackRight),
            (PI + FRAC_PI_2, BillboardFacing8::Right),
            (PI + FRAC_PI_2 + FRAC_PI_4, BillboardFacing8::FrontRight),
        ];
        for (angle, want) in &expected {
            let got = BillboardFacing8::from_relative_angle(*angle);
            assert_eq!(
                got, *want,
                "angle {angle:.4} should be {want:?}, got {got:?}"
            );
        }
    }

    #[test]
    fn facing_sector_boundaries_match_renderer_quantization() {
        assert_eq!(
            BillboardFacing8::from_relative_angle(PI / 8.0 - 0.001),
            BillboardFacing8::Front,
            "just before +22.5 degrees stays Front"
        );
        assert_eq!(
            BillboardFacing8::from_relative_angle(PI / 8.0),
            BillboardFacing8::FrontLeft,
            "exact +22.5 degree boundary rounds to FrontLeft (CCW = target's left)"
        );
        assert_eq!(
            BillboardFacing8::from_relative_angle(-PI / 8.0),
            BillboardFacing8::Front,
            "exact -22.5 degree boundary rounds to Front"
        );
        assert_eq!(
            BillboardFacing8::from_relative_angle(-PI / 8.0 - 0.001),
            BillboardFacing8::FrontRight,
            "just past -22.5 degrees rounds to FrontRight (CW = target's right)"
        );
    }

    #[test]
    fn facing_wraps_negative_angles() {
        assert_eq!(
            BillboardFacing8::from_relative_angle(-FRAC_PI_4),
            BillboardFacing8::FrontRight
        );
        assert_eq!(
            BillboardFacing8::from_relative_angle(-FRAC_PI_2),
            BillboardFacing8::Right
        );
    }

    #[test]
    fn facing_wraps_large_angles() {
        assert_eq!(
            BillboardFacing8::from_relative_angle(TAU + FRAC_PI_4),
            BillboardFacing8::FrontLeft
        );
    }

    #[test]
    fn back_diagonals_are_geometric_not_mirrored_asset_keys() {
        // The renderer's asset list also names its +135°/+225° buckets
        // BackLeft/BackRight, so these two are easy to silently mirror when
        // cross-referencing — pin the geometry explicitly.
        //
        // +135°: attacker is behind and to the target's LEFT (CCW = left).
        assert_eq!(
            BillboardFacing8::from_relative_angle(3.0 * FRAC_PI_4),
            BillboardFacing8::BackLeft,
            "+135 degrees is the back-left octant"
        );
        // +225°: attacker is behind and to the target's RIGHT.
        assert_eq!(
            BillboardFacing8::from_relative_angle(5.0 * FRAC_PI_4),
            BillboardFacing8::BackRight,
            "+225 degrees is the back-right octant"
        );

        // Positional double-check: target at origin facing east (+X). Its
        // left is north (+Y), its right is south (-Y).
        let nw = BillboardFacing8::from_positions(Vec2::ZERO, 0.0, Vec2::new(-5.0, 5.0));
        assert_eq!(nw, BillboardFacing8::BackLeft, "attacker NW of east-facer");
        let sw = BillboardFacing8::from_positions(Vec2::ZERO, 0.0, Vec2::new(-5.0, -5.0));
        assert_eq!(sw, BillboardFacing8::BackRight, "attacker SW of east-facer");
    }

    #[test]
    fn mirrored_reflects_across_forward_axis() {
        use BillboardFacing8 as F;
        let pairs = [
            (F::Front, F::Front),
            (F::FrontLeft, F::FrontRight),
            (F::Left, F::Right),
            (F::BackLeft, F::BackRight),
            (F::Back, F::Back),
        ];
        for (a, b) in pairs {
            assert_eq!(a.mirrored(), b);
            assert_eq!(b.mirrored(), a);
            assert_eq!(a.mirrored().mirrored(), a, "mirror is an involution");
        }
    }

    #[test]
    fn facing_from_positions_front() {
        // Target at origin facing east. Attacker directly east → Front.
        let facing = BillboardFacing8::from_positions(Vec2::ZERO, 0.0, Vec2::new(5.0, 0.0));
        assert_eq!(facing, BillboardFacing8::Front);
    }

    #[test]
    fn facing_from_positions_back() {
        // Target at origin facing east. Attacker directly west → Back.
        let facing = BillboardFacing8::from_positions(Vec2::ZERO, 0.0, Vec2::new(-5.0, 0.0));
        assert_eq!(facing, BillboardFacing8::Back);
    }

    #[test]
    fn facing_from_positions_north_is_targets_left() {
        // Target faces east (+X); its left is north (+Y) — map convention:
        // right is clockwise from forward (facing east, right is south).
        let facing = BillboardFacing8::from_positions(Vec2::ZERO, 0.0, Vec2::new(0.0, 5.0));
        assert_eq!(facing, BillboardFacing8::Left);
    }

    #[test]
    fn facing_from_positions_south_is_targets_right() {
        // Target faces east (+X); its right is south (-Y).
        let facing = BillboardFacing8::from_positions(Vec2::ZERO, 0.0, Vec2::new(0.0, -5.0));
        assert_eq!(facing, BillboardFacing8::Right);
    }

    #[test]
    fn facing_from_positions_respects_non_zero_target_yaw() {
        // Target faces north. Attacker north is now directly in front.
        let facing = BillboardFacing8::from_positions(Vec2::ZERO, FRAC_PI_2, Vec2::new(0.0, 5.0));
        assert_eq!(facing, BillboardFacing8::Front);

        // Same target yaw: west is 90° CCW of north → the target's left.
        let facing = BillboardFacing8::from_positions(Vec2::ZERO, FRAC_PI_2, Vec2::new(-5.0, 0.0));
        assert_eq!(facing, BillboardFacing8::Left);

        // Same target yaw: east is 90° CW of north → the target's right.
        let facing = BillboardFacing8::from_positions(Vec2::ZERO, FRAC_PI_2, Vec2::new(5.0, 0.0));
        assert_eq!(facing, BillboardFacing8::Right);
    }

    #[test]
    fn same_enemy_different_facings_for_two_attackers() {
        let pose = TargetQueryPose2d::new(Vec2::new(5.0, 5.0), 0.0);
        let attacker_a = Vec2::new(8.0, 5.0); // east → Front
        let attacker_b = Vec2::new(5.0, 8.0); // north → target's left

        let facing_a = pose.facing_for_attacker(attacker_a);
        let facing_b = pose.facing_for_attacker(attacker_b);

        assert_eq!(facing_a, BillboardFacing8::Front);
        assert_eq!(facing_b, BillboardFacing8::Left);
        assert_ne!(facing_a, facing_b);
    }

    #[test]
    fn query_pose_uses_target_yaw_for_facing() {
        let pose = TargetQueryPose2d::new(Vec2::ZERO, FRAC_PI_2);
        assert_eq!(
            pose.facing_for_attacker(Vec2::new(0.0, 5.0)),
            BillboardFacing8::Front
        );
        assert_eq!(
            pose.facing_for_attacker(Vec2::new(-5.0, 0.0)),
            BillboardFacing8::Left
        );
    }

    #[test]
    fn coincident_positions_default_to_front() {
        let facing = BillboardFacing8::from_positions(Vec2::ZERO, 0.0, Vec2::ZERO);
        assert_eq!(facing, BillboardFacing8::Front);
    }

    #[test]
    fn facing_index_matches_enum_value() {
        for (i, f) in BillboardFacing8::ALL.iter().enumerate() {
            assert_eq!(f.index(), i);
        }
    }

    // --- TargetCollisionSet lookup ---

    fn test_set() -> TargetCollisionSet {
        let mut set = TargetCollisionSet::new();
        let anim = AnimationKey(0);
        let frame = TargetCollisionFrame::new([PartCollider2d {
            part_id: PartId(1),
            collider: Collider::Circle(Circle::new(Vec2::ZERO, 0.5)),
        }]);
        set.insert_frame(
            CollisionFrameKey {
                animation: anim,
                frame: 0,
                facing: BillboardFacing8::Front,
            },
            frame,
        );
        set.insert_part_metadata(
            PartId(1),
            PartMetadata {
                material: MaterialId(10),
                damage_scale: 2.0,
            },
        );
        set
    }

    #[test]
    fn lookup_valid_frame() {
        let set = test_set();
        let frame = set.lookup(AnimationKey(0), 0, BillboardFacing8::Front);
        assert!(frame.is_some());
        assert_eq!(frame.unwrap().len(), 1);
    }

    #[test]
    fn lookup_missing_facing_returns_none() {
        let set = test_set();
        assert!(
            set.lookup(AnimationKey(0), 0, BillboardFacing8::Back)
                .is_none()
        );
    }

    #[test]
    fn lookup_missing_animation_returns_none() {
        let set = test_set();
        assert!(
            set.lookup(AnimationKey(99), 0, BillboardFacing8::Front)
                .is_none()
        );
    }

    #[test]
    fn lookup_missing_frame_index_returns_none() {
        let set = test_set();
        assert!(
            set.lookup(AnimationKey(0), 5, BillboardFacing8::Front)
                .is_none()
        );
    }

    #[test]
    fn part_metadata_lookup() {
        let set = test_set();
        let meta = set.part_metadata(PartId(1)).unwrap();
        assert_eq!(meta.material, MaterialId(10));
        assert!(approx(meta.damage_scale, 2.0));
    }

    #[test]
    fn missing_part_metadata_returns_none() {
        let set = test_set();
        assert!(set.part_metadata(PartId(99)).is_none());
    }

    // --- TargetCollisionFrame queries ---

    #[test]
    fn frame_nearest_ray_hit_returns_correct_part() {
        let frame = TargetCollisionFrame::new([
            PartCollider2d {
                part_id: PartId(10),
                collider: Collider::Circle(Circle::new(Vec2::new(3.0, 0.0), 0.5)),
            },
            PartCollider2d {
                part_id: PartId(20),
                collider: Collider::Circle(Circle::new(Vec2::new(6.0, 0.0), 0.5)),
            },
        ]);

        let hit = frame.nearest_ray_hit(Vec2::ZERO, Vec2::X).unwrap();
        assert_eq!(hit.id, PartId(10), "nearer part should be hit");
        assert!(approx(hit.distance, 2.5));
    }

    #[test]
    fn frame_nearest_ray_hit_tie_broken_by_part_id() {
        // Two parts at identical positions → same distance → smaller PartId wins.
        let frame = TargetCollisionFrame::new([
            PartCollider2d {
                part_id: PartId(30),
                collider: Collider::Circle(Circle::new(Vec2::new(5.0, 0.0), 1.0)),
            },
            PartCollider2d {
                part_id: PartId(10),
                collider: Collider::Circle(Circle::new(Vec2::new(5.0, 0.0), 1.0)),
            },
        ]);

        let hit = frame.nearest_ray_hit(Vec2::ZERO, Vec2::X).unwrap();
        assert_eq!(hit.id, PartId(10), "smaller PartId wins tie");
    }

    #[test]
    fn frame_nearest_segment_hit_returns_correct_part() {
        let frame = TargetCollisionFrame::new([
            PartCollider2d {
                part_id: PartId(1),
                collider: Collider::Circle(Circle::new(Vec2::new(3.0, 0.0), 0.5)),
            },
            PartCollider2d {
                part_id: PartId(2),
                collider: Collider::Circle(Circle::new(Vec2::new(6.0, 0.0), 0.5)),
            },
        ]);

        let hit = frame
            .nearest_segment_hit(Vec2::ZERO, Vec2::new(10.0, 0.0))
            .unwrap();
        assert_eq!(hit.id, PartId(1));
    }

    #[test]
    fn frame_segment_miss_when_too_short() {
        let frame = TargetCollisionFrame::new([PartCollider2d {
            part_id: PartId(1),
            collider: Collider::Circle(Circle::new(Vec2::new(5.0, 0.0), 0.5)),
        }]);

        assert!(
            frame
                .nearest_segment_hit(Vec2::ZERO, Vec2::new(2.0, 0.0))
                .is_none()
        );
    }

    #[test]
    fn empty_frame_returns_no_hit() {
        let frame = TargetCollisionFrame::default();
        assert!(frame.nearest_ray_hit(Vec2::ZERO, Vec2::X).is_none());
        assert!(
            frame
                .nearest_segment_hit(Vec2::ZERO, Vec2::new(10.0, 0.0))
                .is_none()
        );
    }

    #[test]
    fn frame_parts_iterator_round_trips() {
        let parts = vec![
            PartCollider2d {
                part_id: PartId(1),
                collider: Collider::Circle(Circle::new(Vec2::ZERO, 1.0)),
            },
            PartCollider2d {
                part_id: PartId(2),
                collider: Collider::Circle(Circle::new(Vec2::X, 0.5)),
            },
        ];
        let frame = TargetCollisionFrame::new(parts.clone());
        let round_tripped: Vec<_> = frame.parts().collect();
        assert_eq!(round_tripped.len(), 2);
        assert_eq!(round_tripped[0].part_id, PartId(1));
        assert_eq!(round_tripped[1].part_id, PartId(2));
    }

    // --- No visual pitch ---

    #[test]
    fn target_query_pose_has_no_pitch_field() {
        // Structural: TargetQueryPose2d only has position + yaw.
        let pose = TargetQueryPose2d::new(Vec2::ZERO, 1.0);
        let _ = pose.position;
        let _ = pose.yaw;
        // No pitch field exists — compile-time guarantee.
    }
}

#[cfg(test)]
mod phase7_tests {
    use super::*;

    const HEAD: PartId = PartId(2);
    const BODY: PartId = PartId(1);

    fn table() -> [(&'static str, PartId); 2] {
        [("body", BODY), ("head", HEAD)]
    }

    #[test]
    fn registry_maps_known_names_deterministically() {
        let reg = PartIdRegistry::from_table(&table()).unwrap();
        assert_eq!(reg.id_of("body"), Some(BODY));
        assert_eq!(reg.id_of("head"), Some(HEAD));
        assert_eq!(reg.len(), 2);
        // Order of the authored table must not change the mapping.
        let reg2 = PartIdRegistry::from_table(&[("head", HEAD), ("body", BODY)]).unwrap();
        assert_eq!(reg2.id_of("head"), reg.id_of("head"));
        assert_eq!(reg2.id_of("body"), reg.id_of("body"));
    }

    #[test]
    fn registry_unknown_name_is_none() {
        let reg = PartIdRegistry::from_table(&table()).unwrap();
        assert_eq!(reg.id_of("tail"), None);
    }

    #[test]
    fn registry_reverse_name_lookup() {
        let reg = PartIdRegistry::from_table(&table()).unwrap();
        assert_eq!(reg.name_of(HEAD), Some("head"));
        assert_eq!(reg.name_of(BODY), Some("body"));
        assert_eq!(reg.name_of(PartId(99)), None);
    }

    #[test]
    fn registry_rejects_duplicate_name() {
        let err = PartIdRegistry::from_table(&[("body", BODY), ("body", PartId(3))]).unwrap_err();
        assert_eq!(err, PartIdRegistryError::DuplicateName("body".to_owned()));
    }

    #[test]
    fn registry_accepts_runtime_owned_names() {
        // The shape an asset-loaded ORS import produces: owned Strings built
        // at runtime, not 'static literals.
        let entries: Vec<(String, PartId)> =
            (1..=3).map(|i| (format!("part_{i}"), PartId(i))).collect();
        let reg = PartIdRegistry::from_entries(entries).unwrap();
        assert_eq!(reg.id_of("part_2"), Some(PartId(2)));
        assert_eq!(reg.name_of(PartId(3)), Some("part_3"));
        assert_eq!(reg.len(), 3);
    }

    #[test]
    fn registry_duplicate_runtime_name_is_rejected() {
        let dup = String::from("he") + "ad";
        let err = PartIdRegistry::from_entries([("head".to_owned(), HEAD), (dup, PartId(9))])
            .unwrap_err();
        assert_eq!(err, PartIdRegistryError::DuplicateName("head".to_owned()));
    }

    #[test]
    fn registry_rejects_duplicate_id() {
        let err = PartIdRegistry::from_table(&[("body", BODY), ("torso", BODY)]).unwrap_err();
        assert_eq!(err, PartIdRegistryError::DuplicateId(BODY));
    }

    #[test]
    fn registry_rejects_reserved_fallback_id() {
        let err = PartIdRegistry::from_table(&[("whole", PartId::FALLBACK)]).unwrap_err();
        assert_eq!(err, PartIdRegistryError::ReservedId);
    }

    #[test]
    fn empty_registry_is_empty() {
        let reg = PartIdRegistry::from_table(&[]).unwrap();
        assert!(reg.is_empty());
        assert_eq!(reg.id_of("anything"), None);
    }

    #[test]
    fn part_runtime_state_default_is_safe_and_inert() {
        let s = PartRuntimeState::default();
        assert_eq!(s, PartRuntimeState::fresh());
        assert_eq!(s.current_durability, None);
        assert!(!s.broken);
        assert!(
            s.collision_enabled,
            "default must keep collision enabled (not the derived bool false)"
        );
    }
}
