//! 图片读取与缩略图。

use std::path::Path;

use anyhow::{Context, Result};
use image::RgbaImage;

/// 支持的输入扩展名
pub const SUPPORTED_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "bmp", "gif"];

/// 文件对话框过滤器
pub const IMAGE_DIALOG_FILTER: &[&str] = &["png", "jpg", "jpeg", "bmp", "gif"];

/// 读取任意支持的图片为 RGBA8
pub fn load_rgba(path: &Path) -> Result<RgbaImage> {
    let img = image::open(path).with_context(|| format!("无法打开图片: {}", path.display()))?;
    Ok(img.to_rgba8())
}

/// 生成等比缩略图（最长边不超过 `max_side`）
pub fn thumbnail(img: &RgbaImage, max_side: u32) -> RgbaImage {
    let longest = img.width().max(img.height());
    if longest == 0 || longest <= max_side {
        return img.clone();
    }
    let scale = max_side as f32 / longest as f32;
    let width = ((img.width() as f32 * scale).round() as u32).max(1);
    let height = ((img.height() as f32 * scale).round() as u32).max(1);
    image::imageops::resize(img, width, height, image::imageops::FilterType::Triangle)
}

/// 判断路径是否为受支持的图片
pub fn is_image_path(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| SUPPORTED_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}
