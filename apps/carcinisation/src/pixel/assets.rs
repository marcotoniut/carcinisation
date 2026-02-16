//! Sprite/typeface asset loading helpers and metadata writers for `seldom_pixel`.

use crate::components::GBColor;
use bevy::{
    asset::{AssetPath, AssetServer},
    ecs::system::SystemParam,
    prelude::{Handle, Res},
};
use seldom_pixel::{
    filter::{PxFilter, PxFilterAsset},
    prelude::{PxSprite, PxSpriteAsset, PxTypeface},
};
#[cfg(not(target_family = "wasm"))]
use std::fs;
use std::{collections::HashMap, marker::PhantomData, path::PathBuf};

pub type PxAsset<T> = T;
pub type PxSpriteData = PxSpriteAsset;
pub type PxFilterData = PxFilterAsset;
pub type PxTypefaceData = PxTypeface;

#[derive(SystemParam)]
pub struct PxAssets<'w, 's, T: 'static> {
    asset_server: Res<'w, AssetServer>,
    _marker: PhantomData<&'s T>,
}

fn into_asset_path(path: impl Into<String>) -> AssetPath<'static> {
    AssetPath::from(path.into())
}

impl<'w, 's> PxAssets<'w, 's, PxSprite> {
    pub fn load(&self, path: impl Into<String>) -> Handle<PxAsset<PxSpriteData>> {
        self.asset_server.load(into_asset_path(path))
    }

    pub fn load_animated(
        &self,
        path: impl Into<String>,
        frames: usize,
    ) -> Handle<PxAsset<PxSpriteData>> {
        let path = path.into();
        ensure_sprite_meta(&path, frames);
        self.asset_server.load(into_asset_path(path))
    }
}

impl<'w, 's> PxAssets<'w, 's, PxFilter> {
    pub fn load(&self, path: impl Into<String>) -> Handle<PxAsset<PxFilterData>> {
        self.asset_server.load(into_asset_path(path))
    }

    pub fn load_color(&self, color: GBColor) -> Handle<PxFilterAsset> {
        self.asset_server.load(color.get_filter_path())
    }
}

impl<'w, 's> PxAssets<'w, 's, PxTypeface> {
    pub fn load(
        &self,
        path: impl Into<String>,
        characters: &str,
        separators: impl IntoIterator<Item = (char, u32)>,
    ) -> Handle<PxAsset<PxTypefaceData>> {
        let characters = characters.to_string();
        let separator_map: HashMap<char, u32> = separators.into_iter().collect();
        let path = path.into();
        ensure_typeface_meta(&path, &characters, &separator_map);
        self.asset_server.load(into_asset_path(path))
    }
}

pub(crate) fn ensure_sprite_meta(path: &str, frames: usize) {
    if frames == 0 {
        return;
    }

    let meta_path = asset_meta_path(path);
    let contents = sprite_meta_contents(frames);
    write_meta_file(&meta_path, &contents);
}

pub(crate) fn ensure_typeface_meta(path: &str, characters: &str, separators: &HashMap<char, u32>) {
    let meta_path = asset_meta_path(path);
    let contents = typeface_meta_contents(characters, separators);
    write_meta_file(&meta_path, &contents);
}

fn asset_meta_path(path: &str) -> PathBuf {
    PathBuf::from("assets").join(format!("{path}.meta"))
}

#[cfg(not(target_family = "wasm"))]
fn write_meta_file(path: &PathBuf, contents: &str) {
    #[cfg(debug_assertions)]
    {
        if let Ok(existing) = fs::read_to_string(path) {
            if existing == contents {
                return;
            }
        }

        if let Some(parent) = path.parent() {
            if let Err(err) = fs::create_dir_all(parent) {
                panic!("Failed to create directories for {}: {err}", path.display());
            }
        }

        if let Err(err) = fs::write(path, contents) {
            panic!("Failed to write {}: {err}", path.display());
        }
    }

    #[cfg(not(debug_assertions))]
    {
        if !path.exists() {
            panic!(
                "Missing asset meta file {}. Run the game or the metadata generator in a debug build to create it.",
                path.display()
            );
        }
    }
}

#[cfg(target_family = "wasm")]
fn write_meta_file(_path: &PathBuf, _contents: &str) {}

fn sprite_meta_contents(frames: usize) -> String {
    format!(
        r#"(
    meta_format_version: "1.0",
    asset: Load(
        loader: "seldom_pixel::sprite::PxSpriteLoader",
        settings: (
            frame_count: {frames},
            image_loader_settings: (
                format: FromExtension,
                is_srgb: true,
                sampler: Default,
                asset_usage: RenderAssetUsages("RENDER_WORLD | MAIN_WORLD"),
            ),
        ),
    ),
)
"#
    )
}

fn typeface_meta_contents(characters: &str, separators: &HashMap<char, u32>) -> String {
    let separator_entries = if separators.is_empty() {
        String::from("{}")
    } else {
        let mut parts: Vec<String> = separators
            .iter()
            .map(|(ch, width)| format!("'{}': {}", escape_ron_char(*ch), width))
            .collect();
        parts.sort();
        format!("{{ {} }}", parts.join(", "))
    };

    // Typeface atlases generate characters from bottom to top, so reverse to match
    // the vertical order expected by the loader.
    let glyph_order: String = characters.chars().rev().collect();
    let escaped_chars = escape_ron_string(&glyph_order);
    format!(
        r#"(
    meta_format_version: "1.0",
    asset: Load(
        loader: "seldom_pixel::text::PxTypefaceLoader",
        settings: (
            default_frames: 1,
            characters: "{escaped_chars}",
            character_frames: {{}},
            separator_widths: {separator_entries},
            image_loader_settings: (
                format: FromExtension,
                is_srgb: true,
                sampler: Default,
                asset_usage: RenderAssetUsages("RENDER_WORLD | MAIN_WORLD"),
            ),
        ),
    ),
)
"#
    )
}

fn escape_ron_char(ch: char) -> String {
    match ch {
        '\'' => String::from("\\'"),
        '\\' => String::from("\\\\"),
        _ => ch.to_string(),
    }
}

fn escape_ron_string(value: &str) -> String {
    value.chars().flat_map(|c| c.escape_default()).collect()
}
