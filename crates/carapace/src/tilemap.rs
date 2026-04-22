use std::{error::Error, mem::replace, path::PathBuf};

use bevy_asset::{AssetLoader, LoadContext, io::Reader};
use bevy_ecs::entity::EntityHashMap;
use bevy_image::{CompressedImageFormats, ImageLoader, ImageLoaderSettings};
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
    animation::AnimatedAssetComponent,
    image::CxImage,
    palette::Palette,
    position::{CxLayer, DefaultLayer, Spatial},
    prelude::*,
    sprite::CxSpriteAsset,
};

pub(crate) fn plug_core(app: &mut App, palette_path: PathBuf) {
    app.init_asset::<CxTileset>()
        .register_asset_loader(CxTilesetLoader::new(palette_path));
}

pub(crate) fn plug<L: CxLayer>(app: &mut App, palette_path: PathBuf) {
    #[cfg(feature = "headed")]
    app.add_plugins((
        RenderAssetPlugin::<CxTileset>::default(),
        SyncComponentPlugin::<CxTilemap>::default(),
        SyncComponentPlugin::<CxTile>::default(),
    ));

    plug_core(app, palette_path);

    #[cfg(feature = "headed")]
    app.sub_app_mut(RenderApp)
        .add_systems(ExtractSchedule, (extract_maps::<L>, extract_tiles));
}

#[derive(Serialize, Deserialize)]
struct CxTilesetLoaderSettings {
    tile_size: UVec2,
    image_loader_settings: ImageLoaderSettings,
}

impl Default for CxTilesetLoaderSettings {
    fn default() -> Self {
        Self {
            tile_size: UVec2::ONE,
            image_loader_settings: default(),
        }
    }
}

#[derive(TypePath)]
struct CxTilesetLoader {
    palette_path: PathBuf,
}

impl CxTilesetLoader {
    fn new(palette_path: PathBuf) -> Self {
        Self { palette_path }
    }
}

fn tileset_pixel_pos(
    tile_index: u32,
    tile_pos: u32,
    tile_tileset_width: u32,
    tile_size: UVec2,
) -> UVec2 {
    UVec2::new(
        tile_index % tile_tileset_width,
        tile_index / tile_tileset_width,
    ) * tile_size
        + UVec2::new(tile_pos % tile_size.x, tile_pos / tile_size.x)
}

fn remap_map_tiles(
    map: &CxTilemap,
    mut resolve: impl FnMut(Entity) -> Option<Entity>,
) -> CxTilemap {
    let mut map = map.clone();
    for opt_tile in &mut map.tiles.tiles {
        if let Some(tile) = *opt_tile {
            *opt_tile = resolve(tile);
        }
    }

    map.tiles.tile_poses.clear();
    for (index, opt_tile) in map.tiles.tiles.iter().copied().enumerate() {
        if let Some(tile) = opt_tile {
            map.tiles.tile_poses.insert(tile, index);
        }
    }

    map
}

impl AssetLoader for CxTilesetLoader {
    type Asset = CxTileset;
    type Settings = CxTilesetLoaderSettings;
    type Error = Box<dyn Error + Send + Sync>;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        settings: &CxTilesetLoaderSettings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<CxTileset, Self::Error> {
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
        let tile_size = settings.tile_size;
        let tile_area = tile_size.x * tile_size.y;
        let mut tileset = Vec::default();
        let mut tile = Vec::with_capacity(tile_area as usize);
        let tileset_width = image.texture_descriptor.size.width;
        let tile_tileset_width = tileset_width / tile_size.x;
        let mut max_frame_count = 0;

        for i in 0..indices.area() {
            let tile_index = i as u32 / tile_area;
            let tile_pos = i as u32 % tile_area;

            tile.push(indices.pixel(
                tileset_pixel_pos(tile_index, tile_pos, tile_tileset_width, tile_size).as_ivec2(),
            ));

            if tile_pos == tile_area - 1
                && tile_index % tile_tileset_width == tile_tileset_width - 1
            {
                while tile.len() > tile_area as usize
                    && tile[tile.len() - tile_area as usize..tile.len()]
                        .iter()
                        .all(|&pixel| pixel == 0)
                {
                    tile.truncate(tile.len() - tile_area as usize);
                }

                let frame_count = tile.len() / tile_area as usize;
                if max_frame_count < frame_count {
                    max_frame_count = frame_count;
                }

                tileset.push(CxSpriteAsset {
                    data: CxImage::new(
                        replace(&mut tile, Vec::with_capacity(tile_area as usize)),
                        tile_size.x as usize,
                    ),
                    frame_size: tile_area as usize,
                });
            }
        }

        Ok(CxTileset {
            tileset,
            tile_size,
            max_frame_count,
        })
    }

    fn extensions(&self) -> &[&str] {
        &["px_tileset.png"]
    }
}

/// A tileset for a tilemap. Create a [`Handle<CxTileset>`] through your asset wrapper
/// and provide an image file. The image file contains a column of tiles, ordered from bottom to top.
/// For animated tilesets, add additional frames to the right of tiles, marking the end
/// of an animation with a fully transparent tile or the end of the image.
/// See `assets/tileset/tileset.png` for an example.
#[derive(Asset, Clone, Reflect, Debug)]
pub struct CxTileset {
    pub(crate) tileset: Vec<CxSpriteAsset>,
    tile_size: UVec2,
    max_frame_count: usize,
}

#[cfg(feature = "headed")]
impl RenderAsset for CxTileset {
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

impl CxTileset {
    /// The size of tiles in the tileset
    #[must_use]
    pub fn tile_size(&self) -> UVec2 {
        self.tile_size
    }
}

/// The tiles in a tilemap
#[derive(Clone, Debug)]
pub struct CxTiles {
    tiles: Vec<Option<Entity>>,
    tile_poses: EntityHashMap<usize>,
    width: usize,
}

impl CxTiles {
    /// Creates a [`CxTilemap`]
    #[must_use]
    pub fn new(size: UVec2) -> Self {
        Self {
            tiles: vec![None; (size.x * size.y) as usize],
            tile_poses: EntityHashMap::new(),
            width: size.x as usize,
        }
    }

    fn index(&self, at: UVec2) -> Option<usize> {
        let x = at.x as usize;
        if x >= self.width {
            return None;
        }

        Some(x + at.y as usize * self.width)
    }

    /// Gets a tile. Returns `None` if there is no tile at the given position or if the position is
    /// out of bounds.
    #[must_use]
    pub fn get(&self, at: UVec2) -> Option<Entity> {
        self.tiles.get(self.index(at)?).copied()?
    }

    /// Sets a tile and returns the previous tile at the position. If there was no tile, returns
    /// `None`. If the position is out of bounds, returns `None` and there is no effect.
    pub fn set(&mut self, tile: Option<Entity>, at: UVec2) -> Option<Entity> {
        let index = self.index(at)?;
        let target = self.tiles.get_mut(index)?;
        let old = *target;

        if let Some(old) = old {
            self.tile_poses.remove(&old);
        }

        *target = tile;
        if let Some(tile) = tile {
            self.tile_poses.insert(tile, index);
        }

        old
    }

    /// Gets the size of the map
    #[must_use]
    pub fn size(&self) -> UVec2 {
        let width = self.width as u32;
        UVec2::new(width, self.tiles.len() as u32 / width)
    }

    /// Gets the position of a tile
    #[must_use]
    pub fn pos(&self, id: Entity) -> Option<UVec2> {
        let &index = self.tile_poses.get(&id)?;
        Some(uvec2(
            (index % self.width) as u32,
            (index / self.width) as u32,
        ))
    }
}

impl Default for CxTiles {
    fn default() -> Self {
        Self::new(UVec2::ONE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{camera::CxCamera, frame::draw_spatial, image::CxImage, sprite::CxSpriteAsset};
    use bevy_ecs::entity::Entity;
    #[cfg(feature = "headed")]
    use bevy_ecs::schedule::Schedule;
    use bevy_platform::collections::HashMap;
    #[cfg(feature = "headed")]
    use bevy_render::MainWorld;
    #[cfg(feature = "headed")]
    use bevy_render::sync_world::RenderEntity;

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

    #[test]
    fn map_draws_tiles_in_order() {
        let tileset = CxTileset {
            tileset: vec![
                CxSpriteAsset {
                    data: CxImage::new(vec![2], 1),
                    frame_size: 1,
                },
                CxSpriteAsset {
                    data: CxImage::new(vec![3], 1),
                    frame_size: 1,
                },
            ],
            tile_size: UVec2::ONE,
            max_frame_count: 1,
        };

        let mut tiles = CxTiles::new(UVec2::new(2, 1));
        let tile_a = Entity::from_raw_u32(1).unwrap();
        let tile_b = Entity::from_raw_u32(2).unwrap();
        tiles.set(Some(tile_a), UVec2::new(0, 0));
        tiles.set(Some(tile_b), UVec2::new(1, 0));

        let mut tile_components = HashMap::new();
        tile_components.insert(tile_a, CxTile { texture: 0 });
        tile_components.insert(tile_b, CxTile { texture: 1 });

        let mut image = CxImage::new(vec![1; 2], 2);
        let mut slice = image.slice_all_mut();

        for x in 0..2 {
            let pos = UVec2::new(x, 0);
            let Some(tile_entity) = tiles.get(pos) else {
                continue;
            };
            let tile = tile_components.get(&tile_entity).unwrap();
            let tile_asset = &tileset.tileset[tile.texture as usize];
            draw_spatial(
                tile_asset,
                (),
                &mut slice,
                CxPosition(pos.as_ivec2()),
                CxAnchor::BottomLeft,
                CxRenderSpace::Camera,
                None,
                [],
                CxCamera::default(),
            );
        }

        let expected = vec![2, 3];
        assert_eq!(pixels(&image), expected);
    }

    #[test]
    fn tileset_pixel_pos_uses_tile_width_for_local_row_stride() {
        let tile_size = UVec2::new(2, 3);
        let tile_index = 0;
        let tile_tileset_width = 1;

        // Local index 4 in a 2x3 tile should map to (0, 2), not (0, 1).
        let pos = tileset_pixel_pos(tile_index, 4, tile_tileset_width, tile_size);

        assert_eq!(pos, UVec2::new(0, 2));
    }

    #[test]
    fn remap_map_tiles_updates_entities_and_tile_positions() {
        let src_a = Entity::from_raw_u32(10).unwrap();
        let src_b = Entity::from_raw_u32(11).unwrap();
        let dst_a = Entity::from_raw_u32(20).unwrap();

        let mut tiles = CxTiles::new(UVec2::new(2, 1));
        tiles.set(Some(src_a), UVec2::new(0, 0));
        tiles.set(Some(src_b), UVec2::new(1, 0));

        let map = CxTilemap {
            tiles,
            tileset: default(),
        };

        let remapped = remap_map_tiles(&map, |entity| (entity == src_a).then_some(dst_a));

        assert_eq!(map.tiles.get(UVec2::new(0, 0)), Some(src_a));
        assert_eq!(map.tiles.get(UVec2::new(1, 0)), Some(src_b));

        assert_eq!(remapped.tiles.get(UVec2::new(0, 0)), Some(dst_a));
        assert_eq!(remapped.tiles.get(UVec2::new(1, 0)), None);
        assert_eq!(remapped.tiles.pos(dst_a), Some(UVec2::new(0, 0)));
        assert_eq!(remapped.tiles.pos(src_a), None);
    }

    #[cfg(feature = "headed")]
    #[derive(
        bevy_render::extract_component::ExtractComponent,
        Component,
        next::Next,
        Ord,
        PartialOrd,
        Eq,
        PartialEq,
        Clone,
        Default,
        Debug,
    )]
    #[next(path = next::Next)]
    enum TestLayer {
        #[default]
        Test,
    }

    #[cfg(feature = "headed")]
    #[test]
    fn extract_maps_remaps_when_only_tile_render_entity_changes() {
        let mut render_world = World::new();
        render_world.init_resource::<MainWorld>();
        render_world.insert_resource(crate::position::InsertDefaultLayer::noop());

        let render_map = render_world.spawn_empty().id();
        let render_tile_old = render_world.spawn_empty().id();
        let render_tile_new = render_world.spawn_empty().id();

        let src_tile = {
            let mut main_world = render_world.resource_mut::<MainWorld>();
            main_world.insert_resource(crate::position::InsertDefaultLayer::noop());
            main_world
                .spawn((CxTile { texture: 0 }, RenderEntity::from(render_tile_old)))
                .id()
        };

        let mut tiles = CxTiles::new(UVec2::ONE);
        tiles.set(Some(src_tile), UVec2::ZERO);

        {
            let mut main_world = render_world.resource_mut::<MainWorld>();
            main_world.spawn((
                CxTilemap {
                    tiles,
                    tileset: default(),
                },
                CxPosition::default(),
                TestLayer::default(),
                CxRenderSpace::Camera,
                InheritedVisibility::VISIBLE,
                RenderEntity::from(render_map),
            ));
        }

        let mut schedule = Schedule::default();
        schedule.add_systems(extract_maps::<TestLayer>);
        schedule.run(&mut render_world);

        let map_after_first_extract = render_world.get::<CxTilemap>(render_map).unwrap();
        assert_eq!(
            map_after_first_extract.tiles.get(UVec2::ZERO),
            Some(render_tile_old)
        );

        {
            let mut main_world = render_world.resource_mut::<MainWorld>();
            main_world.clear_trackers();
            main_world
                .entity_mut(src_tile)
                .insert(RenderEntity::from(render_tile_new));
        }

        schedule.run(&mut render_world);

        let map_after_second_extract = render_world.get::<CxTilemap>(render_map).unwrap();
        assert_eq!(
            map_after_second_extract.tiles.get(UVec2::ZERO),
            Some(render_tile_new)
        );
    }

    #[cfg(feature = "headed")]
    #[test]
    fn extract_maps_updates_when_map_render_entity_changes() {
        let mut render_world = World::new();
        render_world.init_resource::<MainWorld>();
        render_world.insert_resource(crate::position::InsertDefaultLayer::noop());

        let render_map_old = render_world.spawn_empty().id();
        let render_map_new = render_world.spawn_empty().id();
        let render_tile = render_world.spawn_empty().id();

        let src_tile = {
            let mut main_world = render_world.resource_mut::<MainWorld>();
            main_world.insert_resource(crate::position::InsertDefaultLayer::noop());
            main_world
                .spawn((CxTile { texture: 0 }, RenderEntity::from(render_tile)))
                .id()
        };

        let mut tiles = CxTiles::new(UVec2::ONE);
        tiles.set(Some(src_tile), UVec2::ZERO);

        let map_entity = {
            let mut main_world = render_world.resource_mut::<MainWorld>();
            main_world
                .spawn((
                    CxTilemap {
                        tiles,
                        tileset: default(),
                    },
                    CxPosition::default(),
                    TestLayer::default(),
                    CxRenderSpace::Camera,
                    InheritedVisibility::VISIBLE,
                    RenderEntity::from(render_map_old),
                ))
                .id()
        };

        let mut schedule = Schedule::default();
        schedule.add_systems(extract_maps::<TestLayer>);
        schedule.run(&mut render_world);

        assert!(render_world.get::<CxTilemap>(render_map_old).is_some());

        {
            let mut main_world = render_world.resource_mut::<MainWorld>();
            main_world.clear_trackers();
            main_world
                .entity_mut(map_entity)
                .insert(RenderEntity::from(render_map_new));
        }

        schedule.run(&mut render_world);

        assert!(
            render_world.get::<CxTilemap>(render_map_new).is_some(),
            "new render entity should receive extracted CxTilemap when map RenderEntity changes"
        );
    }
}

impl<'a> Spatial for (&'a CxTiles, &'a CxTileset) {
    fn frame_size(&self) -> UVec2 {
        let (tiles, tileset) = self;
        tiles.size() * tileset.tile_size
    }
}

/// A tilemap component. Contains tile data and a tileset handle.
#[derive(Component, Default, Clone, Debug)]
#[require(CxPosition, DefaultLayer, CxRenderSpace)]
#[cfg_attr(feature = "headed", require(Visibility))]
pub struct CxTilemap {
    /// The map's tiles
    pub tiles: CxTiles,
    /// The map's tileset
    pub tileset: Handle<CxTileset>,
}

impl AnimatedAssetComponent for CxTilemap {
    type Asset = CxTileset;

    fn handle(&self) -> &Handle<Self::Asset> {
        &self.tileset
    }

    fn max_frame_count(tileset: &CxTileset) -> usize {
        tileset.max_frame_count
    }
}

/// A tile. Must be added to tiles added to [`CxTilemap`].
#[derive(Component, Clone, Default, Debug, Reflect)]
#[cfg_attr(feature = "headed", require(Visibility))]
pub struct CxTile {
    /// The index to the tile texture in the tileset
    pub texture: u32,
}

impl From<u32> for CxTile {
    fn from(value: u32) -> Self {
        Self { texture: value }
    }
}

pub(crate) type MapComponents<L> = (
    &'static CxTilemap,
    &'static CxPosition,
    &'static L,
    &'static CxRenderSpace,
    Option<&'static CxFrameView>,
    Option<&'static CxFilter>,
);

#[cfg(feature = "headed")]
fn extract_maps<L: CxLayer>(
    maps: Extract<Query<(MapComponents<L>, &InheritedVisibility, RenderEntity)>>,
    changed_maps: Extract<
        Query<
            (&CxTilemap, &InheritedVisibility, RenderEntity),
            Or<(
                Changed<CxTilemap>,
                Changed<InheritedVisibility>,
                Changed<RenderEntity>,
            )>,
        >,
    >,
    changed_tiles: Extract<Query<(), (With<CxTile>, Changed<RenderEntity>)>>,
    render_entities: Extract<Query<RenderEntity>>,
    mut cmd: Commands,
) {
    let remap_all_maps = changed_tiles.iter().next().is_some();

    if remap_all_maps {
        for ((map, _, _, _, _, _), visibility, id) in &maps {
            if !visibility.get() {
                continue;
            }

            let mapped = remap_map_tiles(map, |tile| render_entities.get(tile).ok());
            cmd.entity(id).insert(mapped);
        }
    } else {
        for (map, visibility, id) in &changed_maps {
            if !visibility.get() {
                continue;
            }

            let mapped = remap_map_tiles(map, |tile| render_entities.get(tile).ok());
            cmd.entity(id).insert(mapped);
        }
    }

    for ((_, &position, layer, &canvas, frame, filter), visibility, id) in &maps {
        let mut entity = cmd.entity(id);

        if !visibility.get() {
            entity.remove::<L>();
            continue;
        }
        entity.insert((position, layer.clone(), canvas));

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

pub(crate) type TileComponents = (&'static CxTile, Option<&'static CxFilter>);

#[cfg(feature = "headed")]
fn extract_tiles(
    tiles: Extract<Query<(TileComponents, &InheritedVisibility, RenderEntity)>>,
    mut cmd: Commands,
) {
    for ((tile, filter), visibility, entity) in &tiles {
        if !visibility.get() {
            // TODO This doesn't work
            continue;
        }

        let mut entity = cmd.entity(entity);
        entity.insert(tile.clone());

        if let Some(filter) = filter {
            entity.insert(filter.clone());
        } else {
            entity.remove::<CxFilter>();
        }
    }
}
