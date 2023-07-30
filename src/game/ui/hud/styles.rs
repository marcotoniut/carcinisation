use bevy::prelude::*;

pub const HUD_ELEMENT_BACKGROUND_COLOR: Color = Color::rgba(0.0, 0.0, 0.0, 0.3);

pub fn get_hud_style() -> Style {
    Style {
        position_type: PositionType::Absolute,
        justify_content: JustifyContent::SpaceBetween,
        padding: UiRect::all(Val::Px(8.0)),
        width: Val::Percent(100.0),
        height: Val::Percent(100.0),
        ..default()
    }
}

pub fn get_hud_element_style() -> Style {
    Style {
        flex_direction: FlexDirection::Row,
        justify_content: JustifyContent::SpaceBetween,
        align_items: AlignItems::Center,
        column_gap: Val::Px(16.0),
        padding: UiRect::all(Val::Px(16.0)),
        width: Val::Px(120.0),
        height: Val::Px(70.0),
        ..default()
    }
}

pub fn get_hud_element_text_style(asset_server: &Res<AssetServer>) -> TextStyle {
    TextStyle {
        font_size: 48.0,
        color: Color::WHITE,
        font: asset_server.load("fonts/FiraSans-Bold.ttf"),
        ..default()
    }
}

pub fn get_hud_element_image_style() -> Style {
    Style {
        width: Val::Px(42.0),
        height: Val::Px(42.0),
        ..default()
    }
}
