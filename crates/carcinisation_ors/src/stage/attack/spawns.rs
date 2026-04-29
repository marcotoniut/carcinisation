pub mod blood_shot;
pub mod boulder_throw;
pub mod spider_shot;

use bevy::prelude::*;
use carapace::prelude::CxPresentationTransform;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ProjectileSpawnSourceBasis {
    /// The available muzzle point is still in world space, before the source's
    /// collision-affecting presentation offset is applied.
    WorldSpace,
    /// The available muzzle point already matches the source's collision-
    /// affecting presented muzzle and must be used directly.
    Presented,
}

/// Projectile policy: flight stays in plain world space with no in-flight
/// parallax. At fire time only, we may shift the projectile's world-space
/// origin onto the source's collision-affecting presented muzzle point so the
/// first visible frame originates from what the player sees.
///
/// In this pipeline a presented point is `world + collision_offset`. A
/// no-parallax projectile therefore stores the presented muzzle point directly
/// into `WorldPos` at spawn, because its render path has no offset after
/// spawn. Subtracting the source offset here would place the projectile back in
/// the source's world-space muzzle, not at the visible muzzle.
///
/// This is a one-time spawn-space reconstruction only. Ongoing motion remains
/// ordinary world-space simulation after spawn.
#[must_use]
pub(crate) fn projectile_spawn_world_pos_from_source(
    source_muzzle_point: Vec2,
    source_presentation: Option<&CxPresentationTransform>,
    basis: ProjectileSpawnSourceBasis,
) -> Vec2 {
    match basis {
        ProjectileSpawnSourceBasis::WorldSpace => {
            if let Some(presentation) = source_presentation
                && presentation.collision_offset != Vec2::ZERO
            {
                source_muzzle_point + presentation.collision_offset
            } else {
                source_muzzle_point
            }
        }
        ProjectileSpawnSourceBasis::Presented => source_muzzle_point,
    }
}

#[cfg(test)]
mod tests {
    use super::{ProjectileSpawnSourceBasis, projectile_spawn_world_pos_from_source};
    use bevy::prelude::*;
    use carapace::prelude::CxPresentationTransform;

    #[test]
    fn projectile_spawn_origin_stays_in_world_space_for_non_parallaxed_source() {
        let muzzle_world = Vec2::new(10.0, 20.0);

        assert_eq!(
            projectile_spawn_world_pos_from_source(
                muzzle_world,
                None,
                ProjectileSpawnSourceBasis::WorldSpace
            ),
            muzzle_world
        );
    }

    #[test]
    fn projectile_spawn_origin_uses_collision_affecting_presented_muzzle_for_parallaxed_source() {
        let muzzle_world = Vec2::new(10.0, 20.0);
        let presentation = CxPresentationTransform {
            collision_offset: Vec2::new(-6.5, 0.0),
            visual_offset: Vec2::new(-6.5, 3.0),
            ..Default::default()
        };

        assert_eq!(
            projectile_spawn_world_pos_from_source(
                muzzle_world,
                Some(&presentation),
                ProjectileSpawnSourceBasis::WorldSpace
            ),
            Vec2::new(3.5, 20.0)
        );
    }

    #[test]
    fn projectile_spawn_origin_uses_presented_muzzle_directly_when_basis_is_presented() {
        let presented_muzzle = Vec2::new(3.5, 20.0);
        let presentation = CxPresentationTransform {
            collision_offset: Vec2::new(-6.5, 0.0),
            ..Default::default()
        };

        assert_eq!(
            projectile_spawn_world_pos_from_source(
                presented_muzzle,
                Some(&presentation),
                ProjectileSpawnSourceBasis::Presented
            ),
            presented_muzzle
        );
    }
}
