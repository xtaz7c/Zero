use eframe::egui::{self, Color32};

pub const BG_COLOR: Color32 = Color32::BLACK;
pub const TEXT_COLOR: Color32 = Color32::from_rgb(0xEE, 0xEE, 0xEE);
pub const HIGHLIGHT_COLOR: Color32 = Color32::from_rgb(0xE9, 0x1E, 0x63);
pub const MUTED_COLOR: Color32 = Color32::from_rgb(0x88, 0x88, 0x88);

pub fn theme_visuals() -> egui::Visuals {
    let mut v = egui::Visuals::dark();
    v.dark_mode = true;
    v.panel_fill = BG_COLOR;
    v.window_fill = BG_COLOR;
    v.extreme_bg_color = Color32::from_rgb(0x12, 0x12, 0x12); // TextEdit etc.
    v.faint_bg_color = BG_COLOR;
    v.override_text_color = Some(TEXT_COLOR);
    v.selection.bg_fill = HIGHLIGHT_COLOR.gamma_multiply(0.45);
    v.selection.stroke = egui::Stroke::new(1.0, HIGHLIGHT_COLOR);
    v.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, TEXT_COLOR);
    v.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, HIGHLIGHT_COLOR);
    v.widgets.hovered.fg_stroke = egui::Stroke::new(1.5, HIGHLIGHT_COLOR);
    v.widgets.active.fg_stroke = egui::Stroke::new(2.0, HIGHLIGHT_COLOR);
    v.widgets.open.fg_stroke = egui::Stroke::new(1.0, HIGHLIGHT_COLOR);
    v
}
