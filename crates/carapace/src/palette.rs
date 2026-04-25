//! Color palettes
//!
//! Asset loading uses a single global palette; runtime palette swaps only affect rendering.
//! This keeps assets palette-indexed but couples loaders to a shared, immutable palette.

use std::{error::Error, path::PathBuf};

use bevy_asset::{AssetLoader, LoadContext, io::Reader};
use bevy_derive::{Deref, DerefMut};
use bevy_image::{CompressedImageFormats, ImageLoader, ImageLoaderSettings};
use bevy_platform::collections::HashMap;
use bevy_reflect::TypePath;
use bevy_render::render_resource::TextureFormat;

use crate::prelude::*;

pub(crate) fn plug(palette_path: PathBuf) -> impl Fn(&mut App) {
    move |app| {
        app.init_asset::<Palette>()
            .init_asset_loader::<PaletteLoader>()
            .add_systems(Startup, init_palette(palette_path.clone()));
    }
}

#[derive(Default, TypePath)]
struct PaletteLoader;

impl AssetLoader for PaletteLoader {
    type Asset = Palette;
    type Settings = ImageLoaderSettings;
    type Error = Box<dyn Error + Send + Sync>;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        settings: &ImageLoaderSettings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Palette, Self::Error> {
        Palette::new(
            &ImageLoader::new(CompressedImageFormats::NONE)
                .load(reader, settings, load_context)
                .await?,
        )
        .map_err(|err| err.to_string().into())
    }

    fn extensions(&self) -> &[&str] {
        &["palette.png"]
    }
}

/// A palette. Palettes are loaded from images containing pixels
/// that represent what colors the game may display. You may use up to 255 colors.
/// The top-left pixel in the palette is used as the background color.
#[derive(Asset, Clone, TypePath, Debug)]
pub struct Palette {
    pub(crate) size: UVec2,
    // TODO This could be a `[[u8; 3]; 255]`
    pub(crate) colors: Vec<[u8; 3]>,
    pub(crate) indices: HashMap<[u8; 3], u8>,
}

/// Resource containing the game's palette. Set this resource
/// to a new palette to change the game's palette. The replacement palette's pixels
/// must be laid out the same as the original. You cannot change the palette that is used
/// to load assets.
#[derive(Resource, Deref, DerefMut)]
pub struct PaletteHandle(pub Handle<Palette>);

/// Palette index reserved for transparency.  Pixels with this index are
/// never drawn — they let the layer below show through.  The palette
/// image's top-left pixel must be transparent (alpha = 0).
pub const TRANSPARENT_INDEX: u8 = 0;

impl Palette {
    /// Create a palette from an [`Image`]
    ///
    /// # Errors
    ///
    /// Returns an error if the image cannot be converted, is uninitialized, or the top-left pixel
    /// is not transparent.
    pub fn new(image: &Image) -> Result<Palette> {
        let image = image
            .convert(TextureFormat::Rgba8UnormSrgb)
            .ok_or("could not convert palette image to `Rgba8UnormSrgb`")?;
        let data = image.data.ok_or("image is uninitialized")?;

        if data.get(3) != Some(&0) {
            return Err("palette's top left pixel should be transparent".into());
        }

        let (colors, _, _) = data
            .iter()
            .skip(4)
            .copied()
            // TODO Should use chunks here
            .fold(
                (vec![[0, 0, 0]], [0, 0, 0], 0),
                |(mut colors, mut color, i), value| {
                    if i == 3 {
                        if value != 0 {
                            colors.push(color);
                        }
                        (colors, [0, 0, 0], 0)
                    } else {
                        color[i] = value;
                        (colors, color, i + 1)
                    }
                },
            );

        if colors.len() > 256 {
            return Err("palette contains more than 255 colors".into());
        }

        Ok(Palette {
            size: UVec2::new(
                image.texture_descriptor.size.width,
                image.texture_descriptor.size.height,
            ),
            indices: colors
                .iter()
                .enumerate()
                .skip(1)
                .map(|(i, color)| (*color, i as u8))
                .collect(),
            colors,
        })
    }
}

fn init_palette(path: PathBuf) -> impl Fn(Commands, Res<AssetServer>) {
    move |mut commands, assets| {
        let palette = assets.load(path.clone());
        commands.insert_resource(PaletteHandle(palette));
    }
}
