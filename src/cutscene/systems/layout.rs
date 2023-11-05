use super::super::components::*;
use crate::{
    cutscene::data::CutsceneLayer,
    globals::{GBColor, SCREEN_RESOLUTION},
    pixel::components::PxRectangle,
    Layer,
};
use bevy::prelude::*;
use seldom_pixel::{
    asset::*,
    filter::*,
    prelude::{PxAnchor, PxCanvas, PxSubPosition},
};

// pub fn build_screen(commands: &mut Commands, filters: &mut PxAssets<PxFilter>) -> Entity {
//     let mut entity_commands = commands.spawn(Cinematic);

//     let letterbox_filter = filters.load(GBColor::DarkGray.get_filter_path());
//     entity_commands.with_children(|parent| {
//         build_letterbox_top(parent, letterbox_filter.clone());
//         build_letterbox_bottom(parent, letterbox_filter.clone());
//     });

//     return entity_commands.id();
// }
