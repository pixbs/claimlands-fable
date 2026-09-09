//! The prototype's CSS palette as egui visuals.

use egui::{Color32, Stroke};

/// Page background behind everything.
pub const DEEP: Color32 = Color32::from_rgb(0x0e, 0x0b, 0x16);
/// Panel fill: `rgba(27,22,38,.82)`.
pub const PANEL: Color32 = Color32::from_rgba_premultiplied(22, 18, 31, 209);
/// Panel border.
pub const LINE: Color32 = Color32::from_rgb(0x2f, 0x27, 0x40);
/// Body text.
pub const TEXT: Color32 = Color32::from_rgb(0xec, 0xe4, 0xd4);
/// Labels.
pub const MUTED: Color32 = Color32::from_rgb(0x8b, 0x80, 0x9c);
/// Accent for values and the pressed state.
pub const GOLD: Color32 = Color32::from_rgb(0xf2, 0xb4, 0x5c);
/// Accent for focus and the hover ring.
pub const TEAL: Color32 = Color32::from_rgb(0x58, 0xc2, 0xb0);

/// Installs the palette: square corners, thin lines, monospace text.
pub fn apply(ctx: &egui::Context) {
    ctx.set_theme(egui::Theme::Dark);
    ctx.all_styles_mut(style_prototype);
}

fn style_prototype(style: &mut egui::Style) {
    let v = &mut style.visuals;
    *v = egui::Visuals::dark();
    v.panel_fill = PANEL;
    v.window_fill = PANEL;
    v.window_stroke = Stroke::new(1.0, LINE);
    v.window_corner_radius = egui::CornerRadius::ZERO;
    v.widgets.noninteractive.bg_fill = PANEL;
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT);
    v.widgets.inactive.bg_fill = Color32::TRANSPARENT;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, LINE);
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, MUTED);
    v.widgets.hovered.bg_fill = Color32::TRANSPARENT;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, MUTED);
    v.widgets.hovered.fg_stroke = Stroke::new(1.0, TEXT);
    v.widgets.active.bg_fill = GOLD;
    v.widgets.active.fg_stroke = Stroke::new(1.0, Color32::from_rgb(0x1a, 0x12, 0x04));
    v.selection.bg_fill = GOLD;
    v.selection.stroke = Stroke::new(1.0, Color32::from_rgb(0x1a, 0x12, 0x04));
    for w in [
        &mut v.widgets.noninteractive,
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
        &mut v.widgets.open,
    ] {
        w.corner_radius = egui::CornerRadius::ZERO;
    }
    style
        .text_styles
        .iter_mut()
        .for_each(|(_, font)| font.family = egui::FontFamily::Monospace);
}
