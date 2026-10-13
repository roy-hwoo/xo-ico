//! ICO 编码（手工拼装容器，避免额外的第三方编码器依赖）。
//!
//! - 尺寸 < 256：写入 32 位 BGRA 的 BMP 子图（BITMAPINFOHEADER + XOR 位图 + AND 掩码）
//! - 尺寸 >= 256：写入 PNG 子图（现代 Windows 的通行做法）
//!
//! 相比旧版 JS 实现，这里目录项的位深与数据严格一致（统一 32 位）。

use std::io::Cursor;

use anyhow::{anyhow, Result};
use image::{ImageFormat, RgbaImage};

const PNG_THRESHOLD: u32 = 256;

/// 将多张同源、不同尺寸的图像编码为单个 ICO 文件
pub fn encode_ico(images: &[(u32, RgbaImage)]) -> Result<Vec<u8>> {
    if images.is_empty() {
        return Err(anyhow!("没有可编码的图像"));
    }

    let count = images.len();
    let mut dir_entries: Vec<u8> = Vec::with_capacity(count * 16);
    let mut payloads: Vec<Vec<u8>> = Vec::with_capacity(count);
    let mut offset = 6 + 16 * count; // ICONDIR(6) + ICONDIRENTRY*N(16)

    for (size, img) in images {
        let data = if *size >= PNG_THRESHOLD {
            encode_png(img)?
        } else {
            encode_bmp32(img)?
        };

        // 宽/高为 256 时按规范写 0
        let dimension: u8 = if *size >= 256 { 0 } else { *size as u8 };

        dir_entries.push(dimension); // 宽度
        dir_entries.push(dimension); // 高度
        dir_entries.push(0); // 颜色数
        dir_entries.push(0); // 保留
        dir_entries.extend_from_slice(&1u16.to_le_bytes()); // 颜色平面
        dir_entries.extend_from_slice(&32u16.to_le_bytes()); // 位深度
        dir_entries.extend_from_slice(&(data.len() as u32).to_le_bytes()); // 数据大小
        dir_entries.extend_from_slice(&(offset as u32).to_le_bytes()); // 数据偏移

        offset += data.len();
        payloads.push(data);
    }

    let mut out = Vec::with_capacity(offset);
    out.extend_from_slice(&0u16.to_le_bytes()); // 保留
    out.extend_from_slice(&1u16.to_le_bytes()); // 类型：1 = ICO
    out.extend_from_slice(&(count as u16).to_le_bytes()); // 图像数量
    out.extend_from_slice(&dir_entries);
    for payload in payloads {
        out.extend_from_slice(&payload);
    }
    Ok(out)
}

fn encode_png(img: &RgbaImage) -> Result<Vec<u8>> {
    let mut cursor = Cursor::new(Vec::new());
    img.write_to(&mut cursor, ImageFormat::Png)?;
    Ok(cursor.into_inner())
}

/// 32 位 BGRA 位图：自下而上，行按 4 字节对齐（32 位天然对齐）
fn encode_bmp32(img: &RgbaImage) -> Result<Vec<u8>> {
    let width = img.width();
    let height = img.height();
    let row_bytes = width as usize * 4;
    let xor_size = row_bytes * height as usize;
    let and_row_bytes = (width as usize).div_ceil(32) * 4;
    let and_size = and_row_bytes * height as usize;

    let mut out = Vec::with_capacity(40 + xor_size + and_size);

    // BITMAPINFOHEADER (40 字节)
    out.extend_from_slice(&40u32.to_le_bytes());
    out.extend_from_slice(&(width as i32).to_le_bytes());
    out.extend_from_slice(&((height as i32) * 2).to_le_bytes()); // XOR + AND 合计高度
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&32u16.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes()); // 压缩方式：0 = 无压缩
    out.extend_from_slice(&(xor_size as u32).to_le_bytes());
    out.extend_from_slice(&0i32.to_le_bytes()); // X 分辨率
    out.extend_from_slice(&0i32.to_le_bytes()); // Y 分辨率
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());

    let raw = img.as_raw();

    // XOR 位图（BGRA，自下而上）
    for y in (0..height).rev() {
        for x in 0..width {
            let index = ((y * width + x) * 4) as usize;
            out.push(raw[index + 2]); // B
            out.push(raw[index + 1]); // G
            out.push(raw[index]); // R
            out.push(raw[index + 3]); // A
        }
    }

    // AND 掩码（1 位/像素，MSB 优先，自下而上），alpha < 128 视为透明
    for y in (0..height).rev() {
        let mut row = vec![0u8; and_row_bytes];
        for x in 0..width {
            let index = ((y * width + x) * 4) as usize;
            if raw[index + 3] < 128 {
                row[(x as usize) / 8] |= 0x80 >> ((x as usize) % 8);
            }
        }
        out.extend_from_slice(&row);
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_and_directory_entries_are_valid() {
        let small = RgbaImage::from_pixel(16, 16, image::Rgba([1, 2, 3, 255]));
        let large = RgbaImage::from_pixel(256, 256, image::Rgba([4, 5, 6, 255]));
        let bytes = encode_ico(&[(16, small), (256, large)]).unwrap();

        // ICONDIR: 保留=0, 类型=1, 数量=2
        assert_eq!(&bytes[0..4], &[0x00, 0x00, 0x01, 0x00]);
        assert_eq!(u16::from_le_bytes([bytes[4], bytes[5]]), 2);

        // 第一条目录项：16x16，位深 32
        assert_eq!(bytes[6], 16);
        assert_eq!(bytes[7], 16);
        assert_eq!(u16::from_le_bytes([bytes[12], bytes[13]]), 32);

        // 第二条目录项：256 按规范记为 0
        assert_eq!(bytes[6 + 16], 0);
        assert_eq!(bytes[6 + 16 + 1], 0);
    }

    #[test]
    fn bmp_pixels_are_written_bottom_up() {
        let mut img = RgbaImage::new(1, 2);
        img.put_pixel(0, 0, image::Rgba([1, 1, 1, 255])); // 顶行
        img.put_pixel(0, 1, image::Rgba([2, 2, 2, 255])); // 底行
        let bmp = encode_bmp32(&img).unwrap();
        // 位图数据紧随 40 字节头，自下而上 → 首个像素应来自底行
        assert_eq!(bmp[40], 2);
    }

    #[test]
    fn empty_input_is_rejected() {
        assert!(encode_ico(&[]).is_err());
    }
}
