use crate::{
    components::Tag,
    cutscene::{
        components::Cinematic,
        data::{CinematicData, CutsceneAnimationSpawn},
        events::CinematicStartupEvent,
        CutscenePluginUpdateState,
    },
    stage::{
        bundles::{make_background_bundle, make_skybox_bundle},
        components::Stage,
        data::{StageData, StageSpawn},
        events::StageStartupEvent,
        player::events::PlayerStartupEvent,
        StagePluginUpdateState,
    },
    systems::{audio::VolumeSettings, spawn::make_music_bundle},
    Layer,
};
use bevy::{audio::PlaybackMode, prelude::*};
use seldom_pixel::{
    prelude::{PxAnchor, PxAnimationBundle, PxAnimationDuration, PxAssets, PxSubPosition},
    sprite::{PxSprite, PxSpriteBundle},
};

pub fn on_startup(
    mut commands: Commands,
    mut assets_sprite: PxAssets<PxSprite>,
    mut event_reader: EventReader<CinematicStartupEvent>,
    mut cutscene_state_next_state: ResMut<NextState<CutscenePluginUpdateState>>,
    asset_server: Res<AssetServer>,

    volume_settings: Res<VolumeSettings>,
) {
    for e in event_reader.iter() {
        cutscene_state_next_state.set(CutscenePluginUpdateState::Active);

        let data = e.data.as_ref();

        commands.insert_resource::<CinematicData>(data.clone());

        commands.spawn((Cinematic, Name::new("Cinematic")));

        // spawn_music(
        //     &mut commands,
        //     &asset_server,
        //     &volume_settings,
        //     e.data.music_path.clone(),
        //     PlaybackMode::Loop,
        // );
    }
}

pub fn initialise_cinematic_step(
    mut commands: Commands,
    query: Query<
        (Entity, &CutsceneAnimationSpawn),
        (With<Cinematic>, Added<CutsceneAnimationSpawn>),
    >,
    mut assets_sprite: PxAssets<PxSprite>,
) {
    if let Ok((entity, spawn)) = query.get_single() {
        let sprite = assets_sprite.load_animated(spawn.image_path.clone(), spawn.frame_count);

        commands.entity(entity).insert((
            Tag(spawn.tag.clone()),
            PxSpriteBundle::<Layer> {
                sprite,
                anchor: PxAnchor::BottomLeft,
                layer: Layer::CutsceneBackground,
                ..Default::default()
            },
            PxAnimationBundle {
                duration: PxAnimationDuration::millis_per_animation(
                    spawn.duration.as_millis() as u64
                ),
                ..Default::default()
            },
            PxSubPosition::from(spawn.start_coordinates),
        ));
    }
}
