use bevy_inspector_egui::egui::{self, Color32, FontId, RichText, epaint::Shadow};

/// Subdued label color for field names (lighter than default text).
pub const LABEL_COLOR: Color32 = Color32::from_rgb(160, 160, 170);

/// Accent color for section headers.
pub const HEADER_COLOR: Color32 = Color32::from_rgb(210, 210, 220);

/// Applies the editor-wide egui style: dark theme, tighter shadows, wider spacing.
pub fn apply_editor_style(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();

    style.visuals = egui::Visuals {
        window_shadow: Shadow {
            offset: [0, 0],
            ..Default::default()
        },
        ..egui::Visuals::dark()
    };

    // Slightly more vertical breathing room between items.
    style.spacing.item_spacing = egui::vec2(6.0, 4.0);

    ctx.set_style(style);
}

/// Renders a subdued field label (smaller, softer color).
pub fn field_label(ui: &mut egui::Ui, text: &str) {
    ui.label(
        RichText::new(text)
            .font(FontId::proportional(11.0))
            .color(LABEL_COLOR),
    );
}

/// Renders a section header (bold, slightly larger, accent color).
pub fn section_header(ui: &mut egui::Ui, text: &str) {
    ui.add_space(4.0);
    ui.label(
        RichText::new(text)
            .font(FontId::proportional(13.0))
            .color(HEADER_COLOR)
            .strong(),
    );
    ui.add_space(1.0);
}
