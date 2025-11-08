use bevy::prelude::*;
use bevy_prototype_lyon::prelude::*;
use carcinisation::globals::SCREEN_RESOLUTION;
use carcinisation::CutsceneData;

use crate::components::{
    CutsceneActConnection, CutsceneActNode, CutsceneImage, Draggable, LetterboxLabel, SceneItem,
};
use crate::constants::FONT_PATH;
use carcinisation::letterbox::events::LetterboxMove;

const ACT_OFFSET: f32 = 250.0;

pub fn spawn_cutscene(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    data: &CutsceneData, // assuming CutsceneData is the type of data in LoadedScene::Cutscene
                         // camera_query: &mut Query<&mut Transform, With<Camera>>,
) {
    // let mut camera_transform = camera_query.single_mut();
    // camera_transform.translation.x = ACT_OFFSET * data.steps.len() as f32 / 2.0;

    let font_handle = asset_server.load(FONT_PATH);
    let text_font = TextFont {
        font: font_handle.clone().into(),
        font_size: 16.0,
        ..default()
    };
    // let h2_text_style = TextStyle {
    //     font: asset_server.load(FONT_PATH),
    //     font_size: 14.0,
    //     color: Color::WHITE,
    // };

    let mut previous_entity_o: Option<Entity> = None;

    let mut connection_bundles = Vec::new();

    for (act_index, act) in data.steps.iter().enumerate() {
        let act_position = Vec3::new(ACT_OFFSET * act_index as f32, 0.0, 0.0);

        let mut entity_commands = commands.spawn((
            Name::new(format!("Act {}", act_index)),
            CutsceneActNode { act_index },
            Draggable,
            SceneItem,
            Transform::from_translation(act_position),
            GlobalTransform::default(),
        ));
        entity_commands.with_children(|p0| {
            p0.spawn((
                Text2d::new(format!("Act {} ({}s)", act_index, act.elapse.as_secs_f32())),
                text_font.clone(),
                TextColor(Color::WHITE),
                Transform::from_xyz(0.0, SCREEN_RESOLUTION.y as f32 / 2.0 + 25.0, 0.0),
            ));

            if let Some(spawn_images) = &act.spawn_images_o {
                for (image_index, image_spawn) in spawn_images.spawns.iter().enumerate() {
                    let mut sprite =
                        Sprite::from_image(asset_server.load(image_spawn.image_path.clone()));

                    p0.spawn((
                        Name::new(format!("Act {} : Image {}", act_index, image_index)),
                        CutsceneImage,
                        sprite,
                        Transform::from_xyz(0.0, 180.0 * image_index as f32, 0.0),
                    ));
                }
            }

            if let Some(letterbox_move) = &act.letterbox_move_o {
                let instruction = match letterbox_move {
                    LetterboxMove::Open => "Open".to_string(),
                    LetterboxMove::Hide => "Hide".to_string(),
                    LetterboxMove::Show => "Show".to_string(),
                    LetterboxMove::Close => "Close".to_string(),
                    LetterboxMove::To(x) => format!("To {}", x),
                    LetterboxMove::ToAt(x, y) => format!("ToAt {} {}", x, y),
                };

                p0.spawn((
                    LetterboxLabel,
                    Name::new("Letterbox Header"),
                    Text2d::new(format!("Letterbox {}", instruction)),
                    text_font.clone(),
                    TextColor(Color::WHITE),
                    Transform::from_xyz(0.0, SCREEN_RESOLUTION.y as f32 / 2.0 + 10.0, 0.0),
                ));
            }

            if let Some(previous_entity) = previous_entity_o {
                let current_entity = p0.target_entity();

                connection_bundles.push((
                    Name::new(format!(
                        "Act Connection {} {}",
                        previous_entity.index(),
                        current_entity.index(),
                    )),
                    CutsceneActConnection {
                        origin: previous_entity,
                        target: current_entity,
                    },
                    SceneItem,
                ));
            }
        });

        previous_entity_o = Some(entity_commands.id());
    }

    for (name, connection, scene_item) in connection_bundles {
        let placeholder_path = ShapePath::new();
        commands.spawn((
            name,
            connection,
            scene_item,
            ShapeBuilder::with(&placeholder_path)
                .stroke((Color::WHITE, 2.0))
                .build(),
            Transform::default(),
            GlobalTransform::default(),
        ));
    }
}
