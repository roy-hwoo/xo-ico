// release 构建下隐藏 Windows 控制台窗口
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod encode;
mod imaging;
mod model;
mod settings;
mod ui;
mod worker;

use app::IcoApp;

/// 程序图标：同时用作系统标题栏图标与任务栏图标
const APP_ICON: &[u8] = include_bytes!("../assets/XO.ico");

fn main() -> eframe::Result<()> {
    let mut viewport = eframe::egui::ViewportBuilder::default()
        .with_title("XO_ICO生成器")
        .with_inner_size([880.0, 500.0])
        .with_resizable(false)
        .with_maximize_button(false);

    if let Some(icon) = load_window_icon() {
        viewport = viewport.with_icon(icon);
    }

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "XO_ICO生成器",
        options,
        Box::new(|cc| Ok(Box::new(IcoApp::new(cc)))),
    )
}

/// 把内嵌的 XO.ico 解码为窗口图标；失败时返回 None（不阻断启动）
fn load_window_icon() -> Option<eframe::egui::IconData> {
    let image = image::load_from_memory_with_format(APP_ICON, image::ImageFormat::Ico).ok()?;
    let rgba = image.to_rgba8();
    let (width, height) = rgba.dimensions();
    Some(eframe::egui::IconData {
        rgba: rgba.into_raw(),
        width,
        height,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_icon_decodes() {
        let icon = load_window_icon().expect("XO.ico 应能解码为窗口图标");
        assert!(icon.width > 0 && icon.height > 0);
        assert_eq!(
            icon.rgba.len(),
            (icon.width * icon.height * 4) as usize,
            "RGBA 数据长度应与宽高匹配"
        );
    }
}
