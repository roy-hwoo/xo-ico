//! 生成参数：尺寸、输出格式、缩放模式、颜色模式与自定义背景色。

use eframe::egui;

use crate::app::IcoApp;
use crate::model::{ColorMode, OutputFormat, ResizeMode, PRESET_SIZES};

/// 背景色预设
const PRESETS: [[u8; 4]; 4] = [
    [255, 255, 255, 255], // 白
    [0, 0, 0, 255],       // 黑
    [37, 99, 235, 255],   // 主色
    [245, 158, 11, 255],  // 次级色
];

pub fn draw(app: &mut IcoApp, ui: &mut egui::Ui) {
    ui.add_space(8.0);
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.set_width(ui.available_width());

        ui.label("选择预设尺寸（可多选）");
        ui.horizontal(|ui| {
            if ui.small_button("全选尺寸").clicked() {
                app.params.sizes = PRESET_SIZES.to_vec();
            }
            if ui.small_button("取消全选").clicked() {
                app.params.sizes.clear();
            }
        });

        ui.horizontal_wrapped(|ui| {
            for size in PRESET_SIZES {
                let mut checked = app.params.sizes.contains(&size);
                if ui.checkbox(&mut checked, format!("{0}x{0}", size)).changed() {
                    if checked {
                        app.params.sizes.push(size);
                    } else {
                        app.params.sizes.retain(|s| *s != size);
                    }
                }
            }
        });

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label("自定义尺寸");
            ui.add(
                egui::TextEdit::singleline(&mut app.custom_sizes)
                    .hint_text("例如: 48,96,192")
                    .desired_width(220.0),
            );
            ui.label("（逗号分隔，与预设合并去重）");
        });

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label("输出格式");
            egui::ComboBox::from_id_salt("output-format")
                .selected_text(app.params.format.label())
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut app.params.format,
                        OutputFormat::Ico,
                        OutputFormat::Ico.label(),
                    );
                    ui.selectable_value(
                        &mut app.params.format,
                        OutputFormat::Icns,
                        OutputFormat::Icns.label(),
                    );
                });

            ui.separator();
            ui.label("缩放模式");
            egui::ComboBox::from_id_salt("resize-mode")
                .selected_text(app.params.resize_mode.label())
                .show_ui(ui, |ui| {
                    for mode in [ResizeMode::Cover, ResizeMode::Fit, ResizeMode::Stretch] {
                        ui.selectable_value(&mut app.params.resize_mode, mode, mode.label());
                    }
                });
        });

        ui.add_space(8.0);
        draw_color_mode(app, ui);
    });
}

fn draw_color_mode(app: &mut IcoApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.label("颜色模式");
        egui::ComboBox::from_id_salt("color-mode")
            .selected_text(color_mode_label(app.params.color_mode))
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut app.params.color_mode,
                    ColorMode::Transparent,
                    color_mode_label(ColorMode::Transparent),
                );
                ui.selectable_value(
                    &mut app.params.color_mode,
                    ColorMode::SolidBackground,
                    color_mode_label(ColorMode::SolidBackground),
                );
            });
    });

    if app.params.color_mode != ColorMode::SolidBackground {
        return;
    }

    ui.add_space(6.0);
    ui.horizontal_wrapped(|ui| {
        ui.label("背景颜色");
        ui.color_edit_button_srgba_unmultiplied(&mut app.params.background);

        for preset in PRESETS {
            let color =
                egui::Color32::from_rgba_unmultiplied(preset[0], preset[1], preset[2], preset[3]);
            let button = egui::Button::new(egui::RichText::new("■").color(color).size(18.0))
                .frame(false);
            if ui.add(button).on_hover_text("设为背景色").clicked() {
                app.params.background = preset;
            }
        }

        ui.separator();
        ui.label("Hex");
        let response = ui.add(
            egui::TextEdit::singleline(&mut app.hex_input)
                .hint_text("#RRGGBBAA")
                .desired_width(110.0),
        );
        if response.changed() {
            if let Some(rgba) = parse_hex(&app.hex_input) {
                app.params.background = rgba;
            }
        }
    });
}

fn color_mode_label(mode: ColorMode) -> &'static str {
    match mode {
        ColorMode::Transparent => "透明（32位，带透明）",
        ColorMode::SolidBackground => "纯色背景（自定义颜色/透明度）",
    }
}

/// 解析 #RGB / #RRGGBB / #RRGGBBAA
fn parse_hex(text: &str) -> Option<[u8; 4]> {
    let hex = text.trim().trim_start_matches('#');
    let byte = |s: &str| u8::from_str_radix(s, 16).ok();
    match hex.len() {
        6 => Some([
            byte(&hex[0..2])?,
            byte(&hex[2..4])?,
            byte(&hex[4..6])?,
            255,
        ]),
        8 => Some([
            byte(&hex[0..2])?,
            byte(&hex[2..4])?,
            byte(&hex[4..6])?,
            byte(&hex[6..8])?,
        ]),
        _ => None,
    }
}
