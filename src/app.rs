//! 应用状态与 egui 主循环。

use std::path::PathBuf;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::Duration;

use eframe::egui;

use crate::imaging::loader;
use crate::model::{GenParams, ImageDesc, OutputFile};
use crate::settings::Settings;
use crate::ui;
use crate::worker::{self, ProgressMsg};

/// 列表中的一张图片（描述 + 缩略图纹理）
pub struct ImageEntry {
    pub desc: ImageDesc,
    pub texture: egui::TextureHandle,
}

pub struct IcoApp {
    pub images: Vec<ImageEntry>,
    pub params: GenParams,
    pub dark: bool,
    pub generating: bool,
    pub results: Vec<OutputFile>,
    /// 生成完成后是否直接进入保存流程（「保存图标」按钮）
    pub pending_save: bool,
    pub show_result: bool,
    pub custom_sizes: String,
    pub hex_input: String,
    /// (消息, 是否错误, 创建时间)
    pub toast: Option<(String, bool, f64)>,
    receiver: Option<Receiver<ProgressMsg>>,
}

impl IcoApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let settings = Settings::load();
        install_cjk_font(&cc.egui_ctx);
        cc.egui_ctx.set_theme(theme_preference(settings.dark));

        Self {
            images: Vec::new(),
            params: settings.params,
            dark: settings.dark,
            generating: false,
            results: Vec::new(),
            pending_save: false,
            show_result: false,
            custom_sizes: settings.custom_sizes,
            hex_input: String::new(),
            toast: None,
            receiver: None,
        }
    }

    // ---------- 图片管理 ----------

    pub fn add_paths(&mut self, ctx: &egui::Context, paths: &[PathBuf]) {
        let mut added = 0usize;
        let mut failed = 0usize;

        for path in paths {
            if !loader::is_image_path(path) {
                continue;
            }
            match loader::load_rgba(path) {
                Ok(image) => {
                    let thumbnail = loader::thumbnail(&image, 96);
                    let color_image = rgba_to_color_image(&thumbnail);
                    let name = path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();
                    let texture = ctx.load_texture(
                        format!("thumb-{}-{}", name, self.images.len()),
                        color_image,
                        egui::TextureOptions::LINEAR,
                    );
                    self.images.push(ImageEntry {
                        desc: ImageDesc {
                            path: path.clone(),
                            name,
                        },
                        texture,
                    });
                    added += 1;
                }
                Err(_) => failed += 1,
            }
        }

        if added == 0 && failed > 0 {
            self.toast_msg(format!("读取失败：{} 个文件", failed), true, ctx);
        } else if added > 0 {
            let message = if failed > 0 {
                format!("已添加 {} 张（{} 张失败）", added, failed)
            } else {
                format!("已添加 {} 张图片", added)
            };
            self.toast_msg(message, false, ctx);
        }
    }

    // ---------- 生成与保存 ----------

    /// 预设尺寸 + 自定义尺寸 合并去重
    pub fn effective_sizes(&self) -> Vec<u32> {
        let mut sizes = self.params.sizes.clone();
        sizes.extend(parse_custom_sizes(&self.custom_sizes));
        sizes.retain(|size| *size > 0 && *size <= 4096);
        sizes.sort_unstable();
        sizes.dedup();
        sizes
    }

    /// 「保存图标」：生成后直接进入保存流程（生成与保存按钮合并后的唯一入口）
    pub fn start_generate_and_save(&mut self, ctx: &egui::Context) {
        self.start_generate(ctx, true);
    }

    fn start_generate(&mut self, ctx: &egui::Context, save_after: bool) {
        if self.images.is_empty() {
            self.toast_msg("请先添加图片", true, ctx);
            return;
        }

        let sizes = self.effective_sizes();
        if sizes.is_empty() {
            self.toast_msg("请至少选择一个尺寸", true, ctx);
            return;
        }

        let mut params = self.params.clone();
        params.sizes = sizes;

        let items: Vec<(PathBuf, String)> = self
            .images
            .iter()
            .map(|entry| (entry.desc.path.clone(), entry.desc.name.clone()))
            .collect();

        self.pending_save = save_after;
        self.persist();
        self.receiver = Some(worker::spawn_generate(items, params));
        self.generating = true;
        self.show_result = false;
        self.results.clear();
        ctx.request_repaint();
    }

    pub fn save_all(&mut self, ctx: &egui::Context) {
        if self.results.is_empty() {
            return;
        }
        let Some(folder) = rfd::FileDialog::new()
            .set_title("选择保存目录")
            .pick_folder()
        else {
            self.toast_msg("未选择保存位置", true, ctx);
            return;
        };

        let mut saved = 0usize;
        for file in &self.results {
            if std::fs::write(folder.join(&file.name), &file.bytes).is_ok() {
                saved += 1;
            }
        }

        if saved == self.results.len() {
            self.toast_msg(format!("已保存 {} 个文件", saved), false, ctx);
        } else {
            self.toast_msg(
                format!("部分文件保存失败（成功 {}/{}）", saved, self.results.len()),
                true,
                ctx,
            );
        }
    }

    // ---------- 主题与通知 ----------

    pub fn toggle_theme(&mut self, ctx: &egui::Context) {
        self.dark = !self.dark;
        ctx.set_theme(theme_preference(self.dark));
        self.persist();
    }

    pub fn toast_msg(&mut self, text: impl Into<String>, is_error: bool, ctx: &egui::Context) {
        let now = ctx.input(|i| i.time);
        self.toast = Some((text.into(), is_error, now));
    }

    pub fn persist(&self) {
        Settings {
            dark: self.dark,
            params: self.params.clone(),
            custom_sizes: self.custom_sizes.clone(),
        }
        .save();
    }

    // ---------- 内部 ----------

    fn pump_messages(&mut self, ctx: &egui::Context) {
        let Some(receiver) = self.receiver.take() else {
            return;
        };

        let keep = match receiver.try_recv() {
            Ok(ProgressMsg::Finished(files)) => {
                self.results = files;
                self.generating = false;
                self.show_result = true;
                if self.pending_save {
                    self.pending_save = false;
                    self.save_all(ctx);
                } else {
                    self.toast_msg("图标生成成功", false, ctx);
                }
                false
            }
            Ok(ProgressMsg::Failed(message)) => {
                self.generating = false;
                self.toast_msg(format!("生成失败：{}", message), true, ctx);
                false
            }
            Err(TryRecvError::Empty) => true,
            Err(TryRecvError::Disconnected) => {
                self.generating = false;
                false
            }
        };

        if keep {
            self.receiver = Some(receiver);
        }
    }

    fn handle_dropped_files(&mut self, ctx: &egui::Context) {
        let dropped: Vec<PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .map(|file| file.path().to_path_buf())
                .collect()
        });
        if !dropped.is_empty() {
            self.add_paths(ctx, &dropped);
        }
    }

}

impl eframe::App for IcoApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.pump_messages(&ctx);
        self.handle_dropped_files(&ctx);

        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui::toolbar::draw(self, ui);
                    ui::preview_grid::draw(self, ui);
                    ui::options::draw(self, ui);
                    ui::toolbar::draw_bottom(self, ui);
                    ui.add_space(12.0);
                });
        });

        ui::dialogs::draw(self, &ctx);

        if self.generating {
            ctx.request_repaint_after(Duration::from_millis(60));
        }
    }
}

fn theme_preference(dark: bool) -> egui::ThemePreference {
    if dark {
        egui::ThemePreference::Dark
    } else {
        egui::ThemePreference::Light
    }
}

/// egui 内置字体不含中文字形，这里加载系统中文字体作为首选字体。
fn install_cjk_font(ctx: &egui::Context) {
    const CANDIDATES: &[&str] = &[
        // Windows
        r"C:\Windows\Fonts\msyh.ttc",
        r"C:\Windows\Fonts\msyh.ttf",
        r"C:\Windows\Fonts\simhei.ttf",
        r"C:\Windows\Fonts\simsun.ttc",
        r"C:\Windows\Fonts\Deng.ttf",
        // macOS
        "/System/Library/Fonts/PingFang.ttc",
        "/System/Library/Fonts/STHeiti Medium.ttc",
        "/Library/Fonts/Arial Unicode.ttf",
        // Linux
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
    ];

    let Some(bytes) = CANDIDATES.iter().find_map(|path| std::fs::read(path).ok()) else {
        return;
    };

    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "cjk".to_owned(),
        std::sync::Arc::new(egui::FontData::from_owned(bytes)),
    );
    for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        fonts
            .families
            .entry(family)
            .or_default()
            .insert(0, "cjk".to_owned());
    }
    ctx.set_fonts(fonts);
}

fn rgba_to_color_image(image: &image::RgbaImage) -> egui::ColorImage {
    egui::ColorImage::from_rgba_unmultiplied(
        [image.width() as usize, image.height() as usize],
        image.as_raw(),
    )
}

/// 解析逗号/空格分隔的自定义尺寸
pub fn parse_custom_sizes(text: &str) -> Vec<u32> {
    text.split(|c: char| c == ',' || c == '，' || c.is_whitespace())
        .filter_map(|part| part.trim().parse::<u32>().ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_mixed_separators_and_skips_invalid() {
        assert_eq!(parse_custom_sizes("48, 96，192"), vec![48, 96, 192]);
        assert_eq!(parse_custom_sizes("abc, 32"), vec![32]);
        assert_eq!(parse_custom_sizes(""), Vec::<u32>::new());
    }
}
