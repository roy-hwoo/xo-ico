//! 背景色合成：把带透明的图像叠加到自定义背景色上（source-over alpha 混合）。
//!
//! `bg` 为 RGBA，alpha 支持 0..=255，因此背景色可以是半透明的。

use image::{Rgba, RgbaImage};

pub fn flatten_on_background(src: &RgbaImage, bg: [u8; 4]) -> RgbaImage {
    let (width, height) = src.dimensions();
    let mut out = RgbaImage::new(width, height);
    let bg_alpha = bg[3] as u32;

    for (x, y, pixel) in src.enumerate_pixels() {
        let src_alpha = pixel[3] as u32;
        // out_a = sa + ba * (1 - sa)
        let out_alpha = src_alpha + bg_alpha * (255 - src_alpha) / 255;
        if out_alpha == 0 {
            out.put_pixel(x, y, Rgba([0, 0, 0, 0]));
            continue;
        }
        // 非预乘混合：out_c = (src_c * sa + bg_c * ba * (1 - sa)) / out_a
        let channel = |src_c: u8, bg_c: u8| -> u8 {
            let numerator = src_c as u32 * src_alpha
                + (bg_c as u32 * bg_alpha / 255) * (255 - src_alpha);
            (numerator / out_alpha).min(255) as u8
        };
        out.put_pixel(
            x,
            y,
            Rgba([
                channel(pixel[0], bg[0]),
                channel(pixel[1], bg[1]),
                channel(pixel[2], bg[2]),
                out_alpha.min(255) as u8,
            ]),
        );
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    #[test]
    fn opaque_background_replaces_transparent_area() {
        let mut src = RgbaImage::new(2, 1);
        src.put_pixel(0, 0, Rgba([10, 20, 30, 0])); // 完全透明
        src.put_pixel(1, 0, Rgba([10, 20, 30, 255])); // 完全不透明
        let out = flatten_on_background(&src, [255, 255, 255, 255]);
        assert_eq!(out.get_pixel(0, 0).0, [255, 255, 255, 255]);
        assert_eq!(out.get_pixel(1, 0).0, [10, 20, 30, 255]);
    }

    #[test]
    fn half_transparent_source_blends_with_background() {
        let mut src = RgbaImage::new(1, 1);
        src.put_pixel(0, 0, Rgba([0, 0, 0, 128]));
        let out = flatten_on_background(&src, [255, 255, 255, 255]);
        let pixel = out.get_pixel(0, 0).0;
        assert_eq!(pixel[3], 255);
        assert!((120..=136).contains(&pixel[0]), "got {}", pixel[0]);
    }

    #[test]
    fn fully_transparent_background_stays_transparent() {
        let mut src = RgbaImage::new(1, 1);
        src.put_pixel(0, 0, Rgba([1, 2, 3, 0]));
        let out = flatten_on_background(&src, [9, 9, 9, 0]);
        assert_eq!(out.get_pixel(0, 0).0, [0, 0, 0, 0]);
    }
}
