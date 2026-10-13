//! 结果对话框与 Toast 通知。

use std::time::Duration;

use eframe::egui;

use crate::app::IcoApp;

pub fn draw(app: &mut IcoApp, ctx: &egui::Context) {
    result(app, ctx);
    toast(app, ctx);
}

fn result(app: &mut IcoApp, ctx: &egui::Context) {
    if !app.show_result {
        return;
    }

    let mut open = true;

    egui::Window::new("生成完成")
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .default_width(420.0)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ctx, |ui| {
            ui.label(format!("共生成 {} 个文件", app.results.len()));
            ui.add_space(4.0);

            egui::ScrollArea::vertical().max_height(180.0).show(ui, |ui| {
                for file in &app.results {
                    let sizes = file
                        .sizes
                        .iter()
                        .map(|s| s.to_string())
                        .collect::<Vec<_>>()
                        .join(", ");
                    ui.label(format!("• {}　尺寸: {}px", file.name, sizes));
                }
            });

            ui.separator();
            // 关闭按钮居中
            ui.vertical_centered(|ui| {
                if ui.button("关闭").clicked() {
                    app.show_result = false;
                }
            });
        });

    if !open {
        app.show_result = false;
    }
}

fn toast(app: &mut IcoApp, ctx: &egui::Context) {
    let Some((message, is_error, created)) = app.toast.clone() else {
        return;
    };

    let now = ctx.input(|i| i.time);
    if now - created > 3.0 {
        app.toast = None;
        return;
    }

    let fill = if is_error {
        egui::Color32::from_rgb(220, 38, 38)
    } else {
        egui::Color32::from_rgb(5, 150, 105)
    };

    egui::Area::new(egui::Id::new("ico-generator-toast"))
        .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -28.0))
        .interactable(false)
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style()).fill(fill).show(ui, |ui| {
                ui.colored_label(egui::Color32::from_rgb(255, 255, 255), message.clone());
            });
        });

    ctx.request_repaint_after(Duration::from_millis(250));
}
