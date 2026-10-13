//! 顶部工具栏与底部操作区。

use std::path::{Path, PathBuf};

use eframe::egui;

use crate::app::IcoApp;
use crate::imaging::loader;

pub fn draw(app: &mut IcoApp, ui: &mut egui::Ui) {
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        if ui.button("添加图像").clicked() {
            if let Some(paths) = rfd::FileDialog::new()
                .set_title("选择图片")
                .add_filter("图片", loader::IMAGE_DIALOG_FILTER)
                .pick_files()
            {
                let ctx = ui.ctx().clone();
                app.add_paths(&ctx, &paths);
            }
        }

        if ui.button("添加目录").clicked() {
            if let Some(folder) = rfd::FileDialog::new()
                .set_title("选择图片目录")
                .pick_folder()
            {
                let paths = collect_images_in_dir(&folder);
                let ctx = ui.ctx().clone();
                app.add_paths(&ctx, &paths);
            }
        }

        if ui.button("全部清除").clicked() {
            app.images.clear();
            app.results.clear();
        }

        ui.separator();
        ui.label(format!("已添加 {} 张图像", app.images.len()));

        // 主题切换固定在整行最右侧
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let theme_icon = if app.dark { "☀" } else { "🌙" };
            if ui
                .button(theme_icon)
                .on_hover_text("切换深色模式")
                .clicked()
            {
                let ctx = ui.ctx().clone();
                app.toggle_theme(&ctx);
            }
        });
    });
}

pub fn draw_bottom(app: &mut IcoApp, ui: &mut egui::Ui) {
    ui.add_space(10.0);
    ui.separator();
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        let can_save = !app.images.is_empty() && !app.generating;
        // 「生成图标」与「保存文件」合并为唯一入口，尺寸约为普通按钮的 2 倍
        let button = egui::Button::new(egui::RichText::new("保存图标").size(28.0).strong())
            .min_size(egui::vec2(240.0, 60.0));
        if ui.add_enabled(can_save, button).clicked() {
            let ctx = ui.ctx().clone();
            app.start_generate_and_save(&ctx);
        }
        if app.generating {
            ui.spinner();
        }
    });
}

fn collect_images_in_dir(dir: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && loader::is_image_path(&path) {
                paths.push(path);
            }
        }
    }
    paths.sort();
    paths
}
