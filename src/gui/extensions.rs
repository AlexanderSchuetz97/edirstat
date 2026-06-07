use eframe::egui;

use super::{GuiApp, theme};

pub struct ExtensionStat {
    pub ext: String,
    pub total_size: u64,
    pub file_count: u32,
    pub color: egui::Color32,
}

impl GuiApp {
    pub fn draw_extensions_contents(&mut self, ui: &mut egui::Ui) {
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
        ui.vertical(|ui| {
            ui.heading(
                egui::RichText::new("📂 Extensions")
                    .strong()
                    .color(ui.visuals().strong_text_color()),
            );
            ui.separator();

            // Map the pre-computed/pre-sorted stats vector from our background thread
            let shared_ext_stats = self.shared_state.extension_stats.load();
            if !shared_ext_stats.is_empty() {
                self.extension_stats = shared_ext_stats
                    .iter()
                    .map(|(ext, total_size, file_count)| ExtensionStat {
                        ext: ext.clone(),
                        total_size: *total_size,
                        file_count: *file_count,
                        color: theme::get_color_for_extension(ext),
                    })
                    .collect();
            }

            if self.extension_stats.is_empty() {
                ui.label("No statistics gathered yet.");
            } else {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for stat in &self.extension_stats {
                        ui.horizontal(|ui| {
                            // Colored dot
                            let (rect, _) = ui
                                .allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
                            ui.painter().circle_filled(rect.center(), 5.0, stat.color);

                            // Allocate name width and truncate it
                            let name_width = (ui.available_width() - 65.0).max(10.0);
                            ui.allocate_ui(
                                egui::vec2(name_width, ui.spacing().interact_size.y),
                                |ui| {
                                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);

                                    // Render the label and attach a hover tooltip showing file count
                                    ui.label(&stat.ext)
                                        .on_hover_text(format!("Files: {}", stat.file_count));
                                },
                            );

                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.label(
                                        prettier_bytes::ByteFormatter::new()
                                            .format(stat.total_size)
                                            .to_string(),
                                    );
                                },
                            );
                        });
                    }
                });
            }
        });
    }

    pub fn render_extension_panel(&mut self, ui: &mut egui::Ui) {
        egui::Panel::right("right_panel")
            .resizable(true)
            .size_range(80.0..=250.0)
            .default_size(210.0)
            .show_inside(ui, |ui| {
                self.draw_extensions_contents(ui);
            });
    }
}
