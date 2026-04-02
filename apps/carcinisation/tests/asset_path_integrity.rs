//! Asset path integrity validation.
//!
//! Validates that sprite files referenced by enemy data definitions exist on disk:
//! - Legacy atlas-strip mosquitoes (24 sprites across 6 depths × 4 animations)
//! - Legacy atlas-strip tardigrades (12 sprites across 3 depths × 4 animations)
//! - Composed mosquiton atlas assets (atlas.json + atlas.pxi)
//!
//! # Why These Tests Exist
//!
//! Asset path typos, file renames, or missing exports from art tools cause runtime
//! failures that are hard to debug. These deterministic checks catch missing assets
//! at build time before they crash gameplay.
//!
//! Unlike full Asset loading integration tests, these validate the data layer only:
//! - No async asset loading
//! - No Bevy app bootstrap
//! - Fast execution (< 0.01s)
//!
//! # Test Coverage
//!
//! **Mosquito sprites** (24 files):
//! - `sprites/enemies/mosquito_{animation}_{depth}.px_sprite.png`
//! - Depths: 3, 4, 5, 6, 7, 8
//! - Animations: death, fly, idle, melee_attack
//!
//! **Tardigrade sprites** (12 files):
//! - `sprites/enemies/tardigrade_{animation}_{depth}.png`
//! - Depths: 6, 7, 8
//! - Animations: attack, death, idle, sucking
//!
//! **Mosquiton composed assets**:
//! - `sprites/enemies/mosquiton_3/atlas.json`
//! - `sprites/enemies/mosquiton_3/atlas.pxi`

use carcinisation::stage::{
    components::placement::Depth,
    enemy::data::{mosquito::MOSQUITO_ANIMATIONS, tardigrade::TARDIGRADE_ANIMATIONS},
};
use std::{fs, path::PathBuf};

/// Converts asset-relative path to workspace-root-relative path for test validation.
///
/// Animation data stores paths as "sprites/enemies/foo.png" but tests run from
/// the carcinisation app directory. Assets are at workspace root, requiring
/// "../../assets/sprites/enemies/foo.png".
fn to_project_path(asset_path: &str) -> PathBuf {
    PathBuf::from("../../assets").join(asset_path)
}

/// Validates all mosquito sprite files exist at expected paths.
///
/// Mosquitoes use legacy atlas-strip sprites across 6 depth layers with
/// 4 animation states each (24 total files).
#[test]
fn all_mosquito_sprites_exist() {
    let mut missing = Vec::new();

    for (depth, animation_data) in &MOSQUITO_ANIMATIONS.death {
        if !to_project_path(&animation_data.sprite_path).exists() {
            missing.push(format!("mosquito death depth {}", depth.to_i8()));
        }
    }

    for (depth, animation_data) in &MOSQUITO_ANIMATIONS.fly {
        if !to_project_path(&animation_data.sprite_path).exists() {
            missing.push(format!("mosquito fly depth {}", depth.to_i8()));
        }
    }

    for (depth, animation_data) in &MOSQUITO_ANIMATIONS.idle {
        if !to_project_path(&animation_data.sprite_path).exists() {
            missing.push(format!("mosquito idle depth {}", depth.to_i8()));
        }
    }

    for (depth, animation_data) in &MOSQUITO_ANIMATIONS.melee_attack {
        if !to_project_path(&animation_data.sprite_path).exists() {
            missing.push(format!("mosquito melee_attack depth {}", depth.to_i8()));
        }
    }

    assert!(
        missing.is_empty(),
        "Missing mosquito sprites: {}",
        missing.join(", ")
    );
}

/// Validates all tardigrade sprite files exist at expected paths.
///
/// Tardigrades use legacy atlas-strip sprites across 3 depth layers with
/// 4 animation states each (12 total files).
#[test]
fn all_tardigrade_sprites_exist() {
    let mut missing = Vec::new();

    for (depth, animation_data) in &TARDIGRADE_ANIMATIONS.attack {
        if !to_project_path(&animation_data.sprite_path).exists() {
            missing.push(format!("tardigrade attack depth {}", depth.to_i8()));
        }
    }

    for (depth, animation_data) in &TARDIGRADE_ANIMATIONS.death {
        if !to_project_path(&animation_data.sprite_path).exists() {
            missing.push(format!("tardigrade death depth {}", depth.to_i8()));
        }
    }

    for (depth, animation_data) in &TARDIGRADE_ANIMATIONS.idle {
        if !to_project_path(&animation_data.sprite_path).exists() {
            missing.push(format!("tardigrade idle depth {}", depth.to_i8()));
        }
    }

    for (depth, animation_data) in &TARDIGRADE_ANIMATIONS.sucking {
        if !to_project_path(&animation_data.sprite_path).exists() {
            missing.push(format!("tardigrade sucking depth {}", depth.to_i8()));
        }
    }

    assert!(
        missing.is_empty(),
        "Missing tardigrade sprites: {}",
        missing.join(", ")
    );
}

/// Validates mosquiton composed atlas files exist.
///
/// Mosquiton uses composed sprite rendering with multi-part animations authored
/// in Aseprite. Requires atlas.composed.ron (runtime), atlas.json (debug),
/// source.png, atlas.pxi, and atlas.px_atlas.ron.
#[test]
fn mosquiton_composed_atlas_exists() {
    let base = "sprites/enemies/mosquiton_3";
    let required = [
        "atlas.composed.ron",
        "atlas.json",
        "source.png",
        "atlas.pxi",
        "atlas.px_atlas.ron",
    ];

    let mut missing = Vec::new();
    for file in &required {
        let path = to_project_path(&format!("{base}/{file}"));
        if !path.exists() {
            missing.push(format!("{base}/{file}"));
        }
    }

    assert!(
        missing.is_empty(),
        "Mosquiton atlas files missing: {}",
        missing.join(", ")
    );
}

/// Validates the mosquiton PXI file has a correct header and plausible size.
///
/// Decodes the header only — no Bevy or palette infrastructure needed.
/// Guards against truncated or corrupt exports.
#[test]
fn mosquiton_pxi_header_is_valid() {
    let pxi_path = to_project_path("sprites/enemies/mosquiton_3/atlas.pxi");
    let bytes = fs::read(&pxi_path).expect("atlas.pxi should be readable");

    assert!(
        bytes.len() >= 10,
        "PXI file too short: {} bytes",
        bytes.len()
    );
    assert_eq!(&bytes[0..4], b"PXAI", "bad magic");
    assert_eq!(bytes[4], 1, "unexpected version");
    // format: 0 = raw 4bpp, 1 = deflate 4bpp
    assert!(bytes[5] <= 1, "unexpected format byte: {}", bytes[5]);

    let width = u16::from_le_bytes([bytes[6], bytes[7]]) as usize;
    let height = u16::from_le_bytes([bytes[8], bytes[9]]) as usize;
    assert!(width > 0 && height > 0, "zero dimensions: {width}×{height}");

    let payload_len = bytes.len() - 10;
    let raw_packed_len = (width * height + 1) / 2;
    // Compressed payload should be smaller than raw packed size.
    // Raw payload should match exactly.
    if bytes[5] == 0 {
        assert_eq!(payload_len, raw_packed_len, "raw payload size mismatch");
    } else {
        assert!(
            payload_len < raw_packed_len,
            "compressed payload ({payload_len}) should be smaller than raw ({raw_packed_len})"
        );
    }
}

/// Validates mosquito sprite naming follows expected depth range convention.
///
/// Mosquitoes render across depths 3-8 (6 layers). Each animation state must
/// have sprites for all depths.
#[test]
fn mosquito_sprites_cover_full_depth_range() {
    let expected_depths = [
        Depth::Three,
        Depth::Four,
        Depth::Five,
        Depth::Six,
        Depth::Seven,
        Depth::Eight,
    ];

    assert_eq!(
        MOSQUITO_ANIMATIONS.death.len(),
        expected_depths.len(),
        "death animation should cover all mosquito depths"
    );
    assert_eq!(
        MOSQUITO_ANIMATIONS.fly.len(),
        expected_depths.len(),
        "fly animation should cover all mosquito depths"
    );
    assert_eq!(
        MOSQUITO_ANIMATIONS.idle.len(),
        expected_depths.len(),
        "idle animation should cover all mosquito depths"
    );
    assert_eq!(
        MOSQUITO_ANIMATIONS.melee_attack.len(),
        expected_depths.len(),
        "melee_attack animation should cover all mosquito depths"
    );

    for depth in expected_depths {
        assert!(
            MOSQUITO_ANIMATIONS.death.contains_key(&depth),
            "Missing mosquito death sprite for depth {}",
            depth.to_i8()
        );
        assert!(
            MOSQUITO_ANIMATIONS.fly.contains_key(&depth),
            "Missing mosquito fly sprite for depth {}",
            depth.to_i8()
        );
        assert!(
            MOSQUITO_ANIMATIONS.idle.contains_key(&depth),
            "Missing mosquito idle sprite for depth {}",
            depth.to_i8()
        );
        assert!(
            MOSQUITO_ANIMATIONS.melee_attack.contains_key(&depth),
            "Missing mosquito melee_attack sprite for depth {}",
            depth.to_i8()
        );
    }
}

/// Validates tardigrade sprite naming follows expected depth range convention.
///
/// Tardigrades render across depths 6-8 (3 layers, back layer only). Each
/// animation state must have sprites for all depths.
#[test]
fn tardigrade_sprites_cover_full_depth_range() {
    let expected_depths = [Depth::Six, Depth::Seven, Depth::Eight];

    assert_eq!(
        TARDIGRADE_ANIMATIONS.attack.len(),
        expected_depths.len(),
        "attack animation should cover all tardigrade depths"
    );
    assert_eq!(
        TARDIGRADE_ANIMATIONS.death.len(),
        expected_depths.len(),
        "death animation should cover all tardigrade depths"
    );
    assert_eq!(
        TARDIGRADE_ANIMATIONS.idle.len(),
        expected_depths.len(),
        "idle animation should cover all tardigrade depths"
    );
    assert_eq!(
        TARDIGRADE_ANIMATIONS.sucking.len(),
        expected_depths.len(),
        "sucking animation should cover all tardigrade depths"
    );

    for depth in expected_depths {
        assert!(
            TARDIGRADE_ANIMATIONS.attack.contains_key(&depth),
            "Missing tardigrade attack sprite for depth {}",
            depth.to_i8()
        );
        assert!(
            TARDIGRADE_ANIMATIONS.death.contains_key(&depth),
            "Missing tardigrade death sprite for depth {}",
            depth.to_i8()
        );
        assert!(
            TARDIGRADE_ANIMATIONS.idle.contains_key(&depth),
            "Missing tardigrade idle sprite for depth {}",
            depth.to_i8()
        );
        assert!(
            TARDIGRADE_ANIMATIONS.sucking.contains_key(&depth),
            "Missing tardigrade sucking sprite for depth {}",
            depth.to_i8()
        );
    }
}

/// Validates mosquito sprite paths follow .px_sprite.png convention.
///
/// Legacy mosquito sprites use carapace's .px_sprite.png suffix for
/// animated sprite sheets. Ensures no path typos use wrong extension.
#[test]
fn mosquito_sprites_use_px_sprite_extension() {
    for animation_data in MOSQUITO_ANIMATIONS.death.values() {
        assert!(
            animation_data.sprite_path.ends_with(".px_sprite.png"),
            "Mosquito death sprite should use .px_sprite.png: {}",
            animation_data.sprite_path
        );
    }

    for animation_data in MOSQUITO_ANIMATIONS.fly.values() {
        assert!(
            animation_data.sprite_path.ends_with(".px_sprite.png"),
            "Mosquito fly sprite should use .px_sprite.png: {}",
            animation_data.sprite_path
        );
    }

    for animation_data in MOSQUITO_ANIMATIONS.idle.values() {
        assert!(
            animation_data.sprite_path.ends_with(".px_sprite.png"),
            "Mosquito idle sprite should use .px_sprite.png: {}",
            animation_data.sprite_path
        );
    }

    for animation_data in MOSQUITO_ANIMATIONS.melee_attack.values() {
        assert!(
            animation_data.sprite_path.ends_with(".px_sprite.png"),
            "Mosquito melee_attack sprite should use .px_sprite.png: {}",
            animation_data.sprite_path
        );
    }
}

/// Validates tardigrade sprite paths follow .px_sprite.png convention.
///
/// Tardigrade sprites use .px_sprite.png extension (same as mosquitoes).
/// Ensures naming convention consistency.
#[test]
fn tardigrade_sprites_use_px_sprite_extension() {
    for animation_data in TARDIGRADE_ANIMATIONS.attack.values() {
        assert!(
            animation_data.sprite_path.ends_with(".px_sprite.png"),
            "Tardigrade attack sprite should use .px_sprite.png: {}",
            animation_data.sprite_path
        );
    }

    for animation_data in TARDIGRADE_ANIMATIONS.death.values() {
        assert!(
            animation_data.sprite_path.ends_with(".px_sprite.png"),
            "Tardigrade death sprite should use .px_sprite.png: {}",
            animation_data.sprite_path
        );
    }

    for animation_data in TARDIGRADE_ANIMATIONS.idle.values() {
        assert!(
            animation_data.sprite_path.ends_with(".px_sprite.png"),
            "Tardigrade idle sprite should use .px_sprite.png: {}",
            animation_data.sprite_path
        );
    }

    for animation_data in TARDIGRADE_ANIMATIONS.sucking.values() {
        assert!(
            animation_data.sprite_path.ends_with(".px_sprite.png"),
            "Tardigrade sucking sprite should use .px_sprite.png: {}",
            animation_data.sprite_path
        );
    }
}
