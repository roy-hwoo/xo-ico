//! 图片缩略图列表（横向滚动，可逐张移除）。

use eframe::egui;

use crate::app::IcoApp;

pub fn draw(app: &mut IcoApp, ui: &mut egui::Ui) {
    ui.add_space(8.0);
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.set_width(ui.available_width());

        if app.images.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(36.0);
                ui.label("拖放图像到此处，或单击「添加图像」");
                ui.add_space(36.0);
            });
            return;
        }

        let mut remove: Option<usize> = None;
        egui::ScrollArea::horizontal()
            .max_height(150.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    for (index, entry) in app.images.iter().enumerate() {
                        ui.vertical(|ui| {
                            ui.add(egui::Image::new(egui::load::SizedTexture::new(
                                entry.texture.id(),
                                egui::vec2(72.0, 72.0),
                            )));
                            if ui.small_button("移除").clicked() {
                                remove = Some(index);
                            }
                            ui.label(short_name(&entry.desc.name, 14));
                        });
                    }
                });
            });

        if let Some(index) = remove {
            app.images.remove(index);
        }
    });
}

fn short_name(name: &str, max_chars: usize) -> String {
    if name.chars().count() <= max_chars {
        name.to_string()
    } else {
        let head: String = name.chars().take(max_chars.saturating_sub(1)).collect();
        format!("{}…", head)
    }
}
