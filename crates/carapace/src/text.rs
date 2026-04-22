use std::{error::Error, path::PathBuf};

use bevy_asset::{AssetLoader, LoadContext, io::Reader};
use bevy_image::{CompressedImageFormats, ImageLoader, ImageLoaderSettings};
use bevy_platform::collections::HashMap;
use bevy_reflect::TypePath;
#[cfg(feature = "headed")]
use bevy_render::{
    Extract, RenderApp,
    render_asset::{PrepareAssetError, RenderAsset, RenderAssetPlugin},
    sync_component::SyncComponentPlugin,
    sync_world::RenderEntity,
};
use serde::{Deserialize, Serialize};

use crate::{
    animation::AnimatedAssetComponent, image::CxImage, palette::Palette, position::CxLayer,
    position::DefaultLayer, prelude::*,
};

pub(crate) fn plug_core(app: &mut App, palette_path: PathBuf) {
    app.init_asset::<CxTypeface>()
        .register_asset_loader(CxTypefaceLoader::new(palette_path));
}

pub(crate) fn plug<L: CxLayer>(app: &mut App, palette_path: PathBuf) {
    #[cfg(feature = "headed")]
    app.add_plugins((
        RenderAssetPlugin::<CxTypeface>::default(),
        SyncComponentPlugin::<CxText>::default(),
    ));

    plug_core(app, palette_path);

    #[cfg(feature = "headed")]
    app.sub_app_mut(RenderApp)
        .add_systems(ExtractSchedule, extract_texts::<L>);
}

#[derive(Serialize, Deserialize)]
struct CxTypefaceLoaderSettings {
    default_frames: u32,
    characters: String,
    character_frames: HashMap<char, u32>,
    separator_widths: HashMap<char, u32>,
    image_loader_settings: ImageLoaderSettings,
}

impl Default for CxTypefaceLoaderSettings {
    fn default() -> Self {
        Self {
            default_frames: 1,
            characters: String::new(),
            character_frames: HashMap::new(),
            separator_widths: HashMap::new(),
            image_loader_settings: default(),
        }
    }
}

#[derive(TypePath)]
struct CxTypefaceLoader {
    palette_path: PathBuf,
}

impl CxTypefaceLoader {
    fn new(palette_path: PathBuf) -> Self {
        Self { palette_path }
    }
}

impl AssetLoader for CxTypefaceLoader {
    type Asset = CxTypeface;
    type Settings = CxTypefaceLoaderSettings;
    type Error = Box<dyn Error + Send + Sync>;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        settings: &CxTypefaceLoaderSettings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<CxTypeface, Self::Error> {
        let image = ImageLoader::new(CompressedImageFormats::NONE)
            .load(reader, &settings.image_loader_settings, load_context)
            .await?;
        let palette = load_context
            .loader()
            .immediate()
            .load::<Palette>(self.palette_path.clone())
            .await
            .map_err(|err| err.to_string())?;
        let palette = palette.get();
        let indices = CxImage::palette_indices(palette, &image).map_err(|err| err.to_string())?;
        let height = indices.height();
        let character_count = settings.characters.chars().count();

        let characters = if character_count == 0 {
            HashMap::new()
        } else {
            settings
                .characters
                .chars()
                .zip(indices.split_vert(height / character_count).into_iter())
                .map(|(character, mut image)| {
                    image.trim_right();
                    let image_width = image.width();
                    let image_area = image.area();
                    let frames = settings
                        .character_frames
                        .get(&character)
                        .copied()
                        .unwrap_or(settings.default_frames)
                        as usize;

                    (
                        character,
                        CxSpriteAsset {
                            data: CxImage::from_parts_vert(image.split_horz(image_width / frames))
                                .unwrap(),
                            frame_size: image_area / frames,
                        },
                    )
                })
                .collect::<HashMap<_, _>>()
        };

        let max_frame_count = characters.values().fold(0, |max, character| {
            if character.frame_size > max {
                character.frame_size
            } else {
                max
            }
        });

        Ok(CxTypeface {
            height: if image.texture_descriptor.size.height == 0 {
                0
            } else if settings.characters.is_empty() {
                return Err(format!(
                    "Typeface `{}` was assigned no characters. \
                        If no `.meta` file exists for that asset, create one. \
                        See `assets/typeface/` for examples.",
                    load_context.path()
                )
                .into());
            } else {
                image.texture_descriptor.size.height / character_count as u32
            },
            characters,
            separators: settings
                .separator_widths
                .iter()
                .map(|(&separator, &width)| (separator, CxSeparator { width }))
                .collect(),
            max_frame_count,
        })
    }

    fn extensions(&self) -> &[&str] {
        &["px_typeface.png"]
    }
}

#[derive(Clone, Debug, Reflect)]
pub(crate) struct CxSeparator {
    pub(crate) width: u32,
}

/// A typeface. Create a [`Handle<CxTypeface>`] through your asset wrapper
/// and provide an image file. The image file contains a column of characters, ordered from bottom to top.
/// For animated typefaces, add additional frames to the right of characters, marking the end
/// of an animation with a fully transparent character or the end of the image.
/// See the images in `assets/typeface/` for examples.
///
/// # Future: proportional font support
///
/// The layout pipeline already handles variable-width glyphs (auto-trimmed on
/// load), but two small additions would cover remaining edge cases:
///   - A per-typeface `spacing: u32` field (currently hardcoded to 1px in the
///     layout loop) to allow fonts with tighter or wider inter-character gaps.
///   - A per-glyph `bearing_x: i32` on `CxSpriteAsset` (default 0) to shift
///     individual glyphs without baking padding into the source image.
#[derive(Asset, Clone, Reflect, Debug)]
pub struct CxTypeface {
    pub(crate) height: u32,
    pub(crate) characters: HashMap<char, CxSpriteAsset>,
    pub(crate) separators: HashMap<char, CxSeparator>,
    pub(crate) max_frame_count: usize,
}

impl CxTypeface {
    /// Check whether the typeface contains the given character, including separators
    #[must_use]
    pub fn contains(&self, character: char) -> bool {
        self.characters.contains_key(&character) || self.separators.contains_key(&character)
    }
}

#[cfg(feature = "headed")]
impl RenderAsset for CxTypeface {
    type SourceAsset = Self;
    type Param = ();

    fn prepare_asset(
        source_asset: Self,
        _: AssetId<Self>,
        &mut (): &mut (),
        _: Option<&Self>,
    ) -> Result<Self, PrepareAssetError<Self>> {
        Ok(source_asset)
    }
}

/// Spawns text to be rendered on-screen
#[derive(Component, Default, Clone, Debug, Reflect)]
#[require(CxPosition, CxAnchor, DefaultLayer, CxRenderSpace)]
#[cfg_attr(feature = "headed", require(Visibility))]
pub struct CxText {
    /// The contents of the text
    pub value: String,
    /// The typeface
    pub typeface: Handle<CxTypeface>,
    /// The indices of characters after which a line break will be inserted. Should be strictly
    /// ascending. This is automatically computed for UI.
    pub line_breaks: Vec<u32>,
}

impl CxText {
    /// Creates a [`CxText`] with no line breaks
    pub fn new(value: impl Into<String>, typeface: Handle<CxTypeface>) -> Self {
        Self {
            value: value.into(),
            typeface,
            line_breaks: Vec::new(),
        }
    }
}

impl AnimatedAssetComponent for CxText {
    type Asset = CxTypeface;

    fn handle(&self) -> &Handle<Self::Asset> {
        &self.typeface
    }

    fn max_frame_count(typeface: &CxTypeface) -> usize {
        typeface.max_frame_count
    }
}

pub(crate) type TextComponents<L> = (
    &'static CxText,
    &'static CxPosition,
    &'static CxAnchor,
    &'static L,
    &'static CxRenderSpace,
    Option<&'static CxFrameView>,
    Option<&'static CxFilter>,
);

#[cfg(feature = "headed")]
fn extract_texts<L: CxLayer>(
    texts: Extract<Query<(TextComponents<L>, &InheritedVisibility, RenderEntity)>>,
    mut cmd: Commands,
) {
    for ((text, &pos, &alignment, layer, &canvas, frame, filter), visibility, id) in &texts {
        let mut entity = cmd.entity(id);

        if !visibility.get() {
            entity.remove::<L>();
            continue;
        }

        entity.insert((text.clone(), pos, alignment, layer.clone(), canvas));

        if let Some(frame) = frame {
            entity.insert(*frame);
        } else {
            entity.remove::<CxFrameView>();
        }

        if let Some(filter) = filter {
            entity.insert(filter.clone());
        } else {
            entity.remove::<CxFilter>();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{camera::CxCamera, frame::draw_spatial, image::CxImage, sprite::CxSpriteAsset};

    fn pixels(image: &CxImage) -> Vec<u8> {
        let size = image.size();
        let mut out = Vec::with_capacity((size.x * size.y) as usize);
        for y in 0..size.y as i32 {
            for x in 0..size.x as i32 {
                out.push(image.pixel(IVec2::new(x, y)));
            }
        }
        out
    }

    fn draw_text(
        image: &mut CxImage,
        text: &str,
        typeface: &CxTypeface,
        pos: CxPosition,
        alignment: CxAnchor,
    ) {
        let mut slice = image.slice_all_mut();
        let line_break_count = 0_u32;
        let mut size = uvec2(
            0,
            (line_break_count + 1) * typeface.height + line_break_count,
        );
        let mut x = 0;
        let y = 0;
        let mut chars = Vec::new();

        for char in text.chars() {
            if let Some(char) = typeface.characters.get(&char) {
                if x != 0 {
                    x += 1;
                }

                chars.push((x, y, char));
                x += char.data.size().x;

                if x > size.x {
                    size.x = x;
                }
            } else if let Some(separator) = typeface.separators.get(&char) {
                x += separator.width;
            }
        }

        let top_left = *pos - alignment.pos(size).as_ivec2() + ivec2(0, size.y as i32 - 1);

        for (x, y, char) in chars {
            draw_spatial(
                char,
                (),
                &mut slice,
                CxPosition(top_left + ivec2(x as i32, -y)),
                CxAnchor::TopLeft,
                CxRenderSpace::Camera,
                None,
                [],
                CxCamera::default(),
            );
        }
    }

    #[test]
    fn text_draws_characters_with_spacing() {
        let mut characters = HashMap::new();
        characters.insert(
            'A',
            CxSpriteAsset {
                data: CxImage::new(vec![2], 1),
                frame_size: 1,
            },
        );
        let typeface = CxTypeface {
            height: 1,
            characters,
            separators: HashMap::new(),
            max_frame_count: 1,
        };

        let mut image = CxImage::new(vec![1; 12], 4);
        draw_text(
            &mut image,
            "AA",
            &typeface,
            CxPosition(IVec2::new(0, 1)),
            CxAnchor::BottomLeft,
        );

        let expected = vec![1, 1, 1, 1, 1, 1, 1, 1, 2, 1, 2, 1];
        assert_eq!(pixels(&image), expected);
    }
}
