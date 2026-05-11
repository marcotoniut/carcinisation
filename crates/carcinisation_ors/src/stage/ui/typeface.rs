//! Typeface loading helpers for stage UI.

use bevy::prelude::*;

const TYPEFACE_INVERTED_PATH: &str =
    assert_assets_path::assert_assets_path!("typeface/pixeboy-inverted.px_typeface.png");
const TYPEFACE_CHARACTERS: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*()-_=+[{]}\\|;:'\",<.>/?";

/// Loads the standard inverted typeface used by all stage UI overlays.
#[must_use]
pub fn load_inverted_typeface(
    assets: &crate::assets::CxAssets<'_, '_, carapace::prelude::CxTypeface>,
) -> Handle<carapace::prelude::CxTypeface> {
    assets.load(TYPEFACE_INVERTED_PATH, TYPEFACE_CHARACTERS, [(' ', 4)])
}
