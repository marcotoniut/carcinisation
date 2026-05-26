//! Avatar palette permutation infrastructure.
//!
//! Defines which palette indices are protected (always identity) and which
//! form the three remappable colour groups (A, B, C). Provides
//! [`AvatarPaletteRemap`] which expands a [`AvatarPaletteVariant`] into a
//! 16-entry lookup table for draw-time pixel remapping.
//!
//! # Palette layout
//!
//! The protected/group split is loaded from `assets/config/palette.ron` so
//! it can be tuned without recompiling in dev mode. The defaults match the
//! player billboard atlas composited from `assets/sprites/player/player_3/`;
//! all player source parts (body, head, legs, arms, weapon) share the global
//! `.palette.png`.
//!
//! If the palette or atlas sprites change, update the `.ron` and run the
//! validation test suite.

use std::sync::LazyLock;

#[cfg(test)]
use carapace::palette::TRANSPARENT_INDEX;
use carcinisation_net::AvatarPaletteVariant;

// ---------------------------------------------------------------------------
// Runtime config — loaded from .ron, served via LazyLock
// ---------------------------------------------------------------------------

/// Hot-reloadable palette index classification for avatar colour remapping.
///
/// Loaded from `assets/config/palette.ron`.
/// Used by both singleplayer and multiplayer (client-side rendering).
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename = "AvatarPaletteConfig")]
pub struct AvatarPaletteConfig {
    /// Palette indices that are never remapped (always identity).
    pub protected_indices: Vec<u8>,
    /// Three remappable colour groups (A, B, C).
    pub colour_groups: [u8; 3],
}

impl AvatarPaletteConfig {
    fn load() -> Self {
        carcinisation_core::ron_config!("assets/config/palette.ron")
    }
}

impl Default for AvatarPaletteConfig {
    fn default() -> Self {
        Self {
            protected_indices: vec![0, 2],
            colour_groups: [1, 3, 4],
        }
    }
}

static CONFIG: LazyLock<AvatarPaletteConfig> = LazyLock::new(AvatarPaletteConfig::load);

/// Return the config's protected palette indices.
#[inline]
pub(crate) fn protected_indices() -> &'static [u8] {
    &CONFIG.protected_indices
}

/// Return the config's three colour-group palette indices.
#[inline]
pub(crate) fn colour_groups() -> &'static [u8; 3] {
    &CONFIG.colour_groups
}

// ---------------------------------------------------------------------------
// Remap type
// ---------------------------------------------------------------------------

/// 16-entry palette-index remap table.
///
/// `table[src]` gives the remapped palette index to use in place of `src`.
/// `Default` yields the identity mapping: every index maps to itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AvatarPaletteRemap {
    table: [u8; 16],
}

impl AvatarPaletteRemap {
    /// Identity remap — every index maps to itself.
    fn identity() -> Self {
        let mut table = [0u8; 16];
        for i in 0..16u8 {
            table[i as usize] = i;
        }
        Self { table }
    }

    /// Build a remap table from a server-assigned variant.
    #[must_use]
    pub fn from_variant(variant: AvatarPaletteVariant) -> Self {
        let mut table = Self::identity().table;

        let [a, b, c] = *colour_groups();

        match variant {
            AvatarPaletteVariant::Abc => { /* identity — already set */ }
            AvatarPaletteVariant::Acb => {
                table[b as usize] = c;
                table[c as usize] = b;
            }
            AvatarPaletteVariant::Bac => {
                table[a as usize] = b;
                table[b as usize] = a;
            }
            AvatarPaletteVariant::Bca => {
                table[a as usize] = c;
                table[b as usize] = a;
                table[c as usize] = b;
            }
            AvatarPaletteVariant::Cab => {
                table[a as usize] = b;
                table[b as usize] = c;
                table[c as usize] = a;
            }
            AvatarPaletteVariant::Cba => {
                table[a as usize] = c;
                table[c as usize] = a;
            }
        }

        for &idx in protected_indices() {
            table[idx as usize] = idx;
        }

        Self { table }
    }

    /// Apply the remap to a single palette index.
    #[inline]
    #[must_use]
    pub const fn apply(&self, pixel: u8) -> u8 {
        self.table[pixel as usize]
    }
}

impl Default for AvatarPaletteRemap {
    fn default() -> Self {
        Self::identity()
    }
}

// ---------------------------------------------------------------------------
// Startup validation
// ---------------------------------------------------------------------------

/// Validate that a sample sprite frame uses palette indices consistent with
/// the active config.
///
/// Call during atlas initialisation so palette/sprite mismatches fail early.
///
/// # Errors
/// Returns the first palette index that violates expectations.
#[cfg(test)]
pub(crate) fn validate_sprite_palette(sprite: &carapace::image::CxImage) -> Result<(), String> {
    let data = sprite.data();
    if data.is_empty() {
        return Ok(());
    }

    let mut used = [false; 16];
    for &pixel in data {
        if pixel as usize >= 16 {
            return Err(format!(
                "sprite uses palette index {pixel} which is outside the 16-entry remap range",
            ));
        }
        if pixel != TRANSPARENT_INDEX {
            used[pixel as usize] = true;
        }
    }

    let groups = colour_groups();
    let group_used = groups.iter().any(|&g| used[g as usize]);
    if !group_used {
        return Err(format!(
            "none of the colour-group indices {groups:?} appear in sprite — \
             palette may have changed",
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use carcinisation_net::AvatarPaletteVariant;

    // -----------------------------------------------------------------------
    // Remap table correctness
    // -----------------------------------------------------------------------

    #[test]
    fn avatar_palette_config_loads() {
        let _ = AvatarPaletteConfig::load();
    }

    #[test]
    fn identity_maps_all_indices_to_self() {
        let remap = AvatarPaletteRemap::identity();
        for i in 0..16u8 {
            assert_eq!(remap.apply(i), i, "index {i} should map to itself");
        }
    }

    #[test]
    fn default_is_identity() {
        assert_eq!(
            AvatarPaletteRemap::default(),
            AvatarPaletteRemap::identity()
        );
    }

    #[test]
    fn transparent_index_always_zero() {
        for variant in &[
            AvatarPaletteVariant::Abc,
            AvatarPaletteVariant::Acb,
            AvatarPaletteVariant::Bac,
            AvatarPaletteVariant::Bca,
            AvatarPaletteVariant::Cab,
            AvatarPaletteVariant::Cba,
        ] {
            let remap = AvatarPaletteRemap::from_variant(*variant);
            assert_eq!(
                remap.apply(0),
                0,
                "variant {variant:?} must preserve transparent index"
            );
        }
    }

    #[test]
    fn protected_indices_never_remap() {
        for variant in &[
            AvatarPaletteVariant::Abc,
            AvatarPaletteVariant::Acb,
            AvatarPaletteVariant::Bac,
            AvatarPaletteVariant::Bca,
            AvatarPaletteVariant::Cab,
            AvatarPaletteVariant::Cba,
        ] {
            let remap = AvatarPaletteRemap::from_variant(*variant);
            for &idx in protected_indices() {
                assert_eq!(
                    remap.apply(idx),
                    idx,
                    "variant {variant:?} must preserve protected index {idx}"
                );
            }
        }
    }

    #[test]
    fn only_colour_groups_change() {
        let prot = protected_indices();
        let groups = colour_groups();
        for variant in &[
            AvatarPaletteVariant::Abc,
            AvatarPaletteVariant::Acb,
            AvatarPaletteVariant::Bac,
            AvatarPaletteVariant::Bca,
            AvatarPaletteVariant::Cab,
            AvatarPaletteVariant::Cba,
        ] {
            let remap = AvatarPaletteRemap::from_variant(*variant);
            for i in 0..16u8 {
                let is_protected = prot.contains(&i);
                let is_group = groups.contains(&i);
                if is_protected {
                    assert_eq!(
                        remap.apply(i),
                        i,
                        "protected index {i} must be identity for {variant:?}"
                    );
                } else if !is_group {
                    assert_eq!(
                        remap.apply(i),
                        i,
                        "non-group non-protected index {i} must be identity for {variant:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn abc_is_identity() {
        let remap = AvatarPaletteRemap::from_variant(AvatarPaletteVariant::Abc);
        for i in 0..16u8 {
            assert_eq!(remap.apply(i), i, "Abc should be identity for all indices");
        }
    }

    #[test]
    fn acb_swaps_b_and_c() {
        let remap = AvatarPaletteRemap::from_variant(AvatarPaletteVariant::Acb);
        let [a, b, c] = *colour_groups();
        assert_eq!(remap.apply(a), a, "A should stay A");
        assert_eq!(remap.apply(b), c, "B should become C");
        assert_eq!(remap.apply(c), b, "C should become B");
    }

    #[test]
    fn bac_swaps_a_and_b() {
        let remap = AvatarPaletteRemap::from_variant(AvatarPaletteVariant::Bac);
        let [a, b, _c] = *colour_groups();
        assert_eq!(remap.apply(a), b, "A should become B");
        assert_eq!(remap.apply(b), a, "B should become A");
    }

    #[test]
    fn bca_cycles() {
        let remap = AvatarPaletteRemap::from_variant(AvatarPaletteVariant::Bca);
        let [a, b, c] = *colour_groups();
        assert_eq!(remap.apply(a), c, "A should become C");
        assert_eq!(remap.apply(b), a, "B should become A");
        assert_eq!(remap.apply(c), b, "C should become B");
    }

    #[test]
    fn cab_cycles() {
        let remap = AvatarPaletteRemap::from_variant(AvatarPaletteVariant::Cab);
        let [a, b, c] = *colour_groups();
        assert_eq!(remap.apply(a), b, "A should become B");
        assert_eq!(remap.apply(b), c, "B should become C");
        assert_eq!(remap.apply(c), a, "C should become A");
    }

    #[test]
    fn cba_swaps_a_and_c() {
        let remap = AvatarPaletteRemap::from_variant(AvatarPaletteVariant::Cba);
        let [a, b, c] = *colour_groups();
        assert_eq!(remap.apply(a), c, "A should become C");
        assert_eq!(remap.apply(b), b, "B should stay B");
        assert_eq!(remap.apply(c), a, "C should become A");
    }

    // -----------------------------------------------------------------------
    // Palette validation
    // -----------------------------------------------------------------------

    #[test]
    fn validate_empty_sprite_skips() {
        let sprite = carapace::image::CxImage::new(vec![], 1);
        assert!(validate_sprite_palette(&sprite).is_ok());
    }

    #[test]
    fn validate_sprite_missing_colour_groups_fails() {
        let mut data = [0u8; 16];
        data[0] = TRANSPARENT_INDEX;
        for px in data.iter_mut().skip(1) {
            *px = 9;
        }
        let sprite = carapace::image::CxImage::new(data.to_vec(), 4);
        assert!(
            validate_sprite_palette(&sprite).is_err(),
            "sprite without any colour-group indices should fail validation"
        );
    }
}
