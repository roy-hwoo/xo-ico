//! 缩放：cover / fit / stretch 三种策略，输出 size×size 的 RGBA 图像。

use image::imageops::FilterType;
use image::RgbaImage;

use crate::model::ResizeMode;

/// 等比缩放后的目标尺寸（不超过 size×size）
fn fit_scale(src: &RgbaImage, size: u32) -> (u32, u32) {
    let scale = (size as f32 / src.width() as f32).min(size as f32 / src.height() as f32);
    let width = ((src.width() as f32 * scale).round() as u32).clamp(1, size);
    let height = ((src.height() as f32 * scale).round() as u32).clamp(1, size);
    (width, height)
}

pub fn resize(src: &RgbaImage, size: u32, mode: ResizeMode) -> RgbaImage {
    if src.width() == 0 || src.height() == 0 || size == 0 {
        return RgbaImage::new(size.max(1), size.max(1));
    }

    match mode {
        ResizeMode::Stretch => image::imageops::resize(src, size, size, FilterType::Lanczos3),
        ResizeMode::Cover => {
            let side = src.width().min(src.height());
            let left = (src.width() - side) / 2;
            let top = (src.height() - side) / 2;
            let cropped = image::imageops::crop_imm(src, left, top, side, side).to_image();
            image::imageops::resize(&cropped, size, size, FilterType::Lanczos3)
        }
        ResizeMode::Fit => {
            let (width, height) = fit_scale(src, size);
            let scaled = image::imageops::resize(src, width, height, FilterType::Lanczos3);
            let mut canvas = RgbaImage::new(size, size);
            let offset_x = (size - width) / 2;
            let offset_y = (size - height) / 2;
            for (x, y, pixel) in scaled.enumerate_pixels() {
                canvas.put_pixel(x + offset_x, y + offset_y, *pixel);
            }
            canvas
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(width: u32, height: u32) -> RgbaImage {
        RgbaImage::from_pixel(width, height, image::Rgba([10, 20, 30, 255]))
    }

    #[test]
    fn every_mode_outputs_square() {
        let src = sample(30, 10);
        for mode in [ResizeMode::Cover, ResizeMode::Fit, ResizeMode::Stretch] {
            let out = resize(&src, 64, mode);
            assert_eq!((out.width(), out.height()), (64, 64), "{mode:?}");
        }
    }

    #[test]
    fn fit_keeps_transparent_letterbox() {
        // 2:1 源图放入正方形，上下应留透明边
        let src = sample(40, 20);
        let out = resize(&src, 64, ResizeMode::Fit);
        assert_eq!(out.get_pixel(32, 0).0[3], 0);
    }

    #[test]
    fn cover_fills_corners() {
        let src = sample(10, 40);
        let out = resize(&src, 32, ResizeMode::Cover);
        assert_eq!(out.get_pixel(0, 0).0[3], 255);
    }
}
