//! egui HUD, menus and debug panel in the prototype's theme.
#![forbid(unsafe_code)]

pub mod theme;

/// Facts the HUD displays; the app fills it each frame.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HudInfo {
    /// Number of tiles on the planet.
    pub tiles: usize,
    /// Land tiles.
    pub land: usize,
    /// Pentagon tiles.
    pub pentagons: usize,
    /// Hex-sphere frequency.
    pub frequency: u8,
    /// World seed.
    pub seed: u32,
    /// Current pixel scale.
    pub pixel_scale: u32,
    /// Low-resolution target size in texels.
    pub target: (u32, u32),
    /// Build identifier shown in the readout.
    pub build: &'static str,
}

/// What the app should do after a HUD interaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudAction {
    /// Nothing.
    None,
    /// Change the pixel scale.
    SetPixelScale(u32),
}

/// The top-left readout plus the debug toggle, mirroring the prototype's `#readout` panel.
#[derive(Debug, Default)]
pub struct Hud {
    /// Whether the instrumentation panel is open.
    pub debug: bool,
}

impl Hud {
    /// Draws the HUD and returns the action the app should apply.
    pub fn ui(&mut self, ctx: &egui::Context, info: &HudInfo) -> HudAction {
        let mut action = HudAction::None;
        egui::Area::new(egui::Id::new("debug-toggle"))
            .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-12.0, 12.0))
            .show(ctx, |ui| {
                let label = if self.debug { "×" } else { "≡" };
                if ui
                    .add(egui::Button::new(label).min_size(egui::vec2(28.0, 28.0)))
                    .clicked()
                {
                    self.debug = !self.debug;
                }
            });
        if !self.debug {
            return action;
        }
        egui::Window::new("readout")
            .title_bar(false)
            .resizable(false)
            .anchor(egui::Align2::LEFT_TOP, egui::vec2(12.0, 12.0))
            .show(ctx, |ui| {
                ui.label(
                    egui::RichText::new("CLAIM LANDS")
                        .size(11.0)
                        .color(theme::TEXT),
                );
                ui.label(
                    egui::RichText::new(info.build)
                        .size(9.0)
                        .color(theme::MUTED),
                );
                ui.add_space(6.0);
                egui::Grid::new("stats")
                    .num_columns(2)
                    .spacing([14.0, 2.0])
                    .show(ui, |ui| {
                        let row = |ui: &mut egui::Ui, k: &str, v: String| {
                            ui.label(egui::RichText::new(k).size(9.0).color(theme::MUTED));
                            ui.label(egui::RichText::new(v).size(11.0).color(theme::GOLD));
                            ui.end_row();
                        };
                        row(ui, "TILES", info.tiles.to_string());
                        row(
                            ui,
                            "HEX / PENT",
                            format!(
                                "{} / {}",
                                info.tiles.saturating_sub(info.pentagons),
                                info.pentagons
                            ),
                        );
                        row(
                            ui,
                            "LAND / SEA",
                            format!("{} / {}", info.land, info.tiles.saturating_sub(info.land)),
                        );
                        row(ui, "SEED", format!("{} @ n{}", info.seed, info.frequency));
                        row(ui, "TARGET", format!("{}x{}", info.target.0, info.target.1));
                    });
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("PIXEL SIZE")
                            .size(9.0)
                            .color(theme::MUTED),
                    );
                    for s in 1..=4u32 {
                        let selected = info.pixel_scale == s;
                        if ui.selectable_label(selected, format!("{s}×")).clicked() && !selected {
                            action = HudAction::SetPixelScale(s);
                        }
                    }
                });
            });
        action
    }
}
