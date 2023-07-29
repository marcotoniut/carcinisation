use bevy::prelude::*;

pub const NORMAL_BUTTON_COLOR: Color = Color::rgb(0.15, 0.15, 0.15);
pub const HOVERED_BUTTON_COLOR: Color = Color::rgb(0.25, 0.25, 0.25);
pub const PRESSED_BUTTON_COLOR: Color = Color::rgb(0.35, 0.35, 0.35);

pub fn get_main_menu_style() -> Style {
    Style {
        flex_direction: FlexDirection::Column,
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        row_gap: Val::Px(8.0),
        column_gap: Val::Px(8.0),
        width: Val::Percent(100.0),
        height: Val::Percent(100.0),
        ..default()
    }
}

pub fn get_title_style() -> Style {
    Style {
        flex_direction: FlexDirection::Row,
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        margin: UiRect::new(Val::Px(0.0), Val::Px(0.0), Val::Px(0.0), Val::Px(16.0)),
        ..default()
    }
}

// pub const BUTTON_STYLE: Style = Style {
//     width: Val::Px(200.0),
//     height: Val::Px(80.0),
//     ..Style::DEFAULT
// };

// https://github.com/bevyengine/bevy/issues/9095
pub fn get_button_style() -> Style {
    Style {
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        width: Val::Px(200.0),
        height: Val::Px(80.0),
        ..Style::DEFAULT
    }
}

pub fn get_title_image_style() -> Style {
    Style {
        width: Val::Px(64.0),
        height: Val::Px(64.0),
        margin: UiRect::all(Val::Px(8.0)),
        ..default()
    }
}

pub fn get_title_text_style(asset_server: &Res<AssetServer>) -> TextStyle {
    TextStyle {
        font_size: 64.0,
        color: Color::WHITE,
        font: asset_server.load("fonts/FiraSans-Bold.ttf"),
        ..default()
    }
}

pub fn get_button_text_style(asset_server: &Res<AssetServer>) -> TextStyle {
    TextStyle {
        font_size: 40.0,
        color: Color::WHITE,
        font: asset_server.load("fonts/FiraSans-Bold.ttf"),
        ..default()
    }
}
