//! ICNS 编码（macOS）：'icns' + 总长度(大端) + [类型码(4) + 项长度(大端) + PNG 数据]。
//!
//! 采用手工拼装（与旧版行为一致），仅依赖 `image` crate 编码 PNG。

use std::io::Cursor;

use anyhow::{anyhow, Result};
use image::{ImageFormat, RgbaImage};

/// 尺寸 → ICNS 类型码
fn type_code(size: u32) -> Option<[u8; 4]> {
    Some(match size {
        16 => *b"icp4",
        32 => *b"icp5",
        64 => *b"icp6",
        128 => *b"ic07",
        256 => *b"ic08",
        512 => *b"ic09",
        1024 => *b"ic10",
        _ => return None,
    })
}

pub fn encode_icns(images: &[(u32, RgbaImage)]) -> Result<Vec<u8>> {
    let mut body: Vec<u8> = Vec::new();
    let mut count = 0usize;

    for (size, img) in images {
        let Some(code) = type_code(*size) else {
            continue; // 非标准尺寸直接跳过
        };
        let mut cursor = Cursor::new(Vec::new());
        img.write_to(&mut cursor, ImageFormat::Png)?;
        let data = cursor.into_inner();

        body.extend_from_slice(&code);
        body.extend_from_slice(&((data.len() as u32) + 8).to_be_bytes());
        body.extend_from_slice(&data);
        count += 1;
    }

    if count == 0 {
        return Err(anyhow!(
            "ICNS 仅支持 16/32/64/128/256/512/1024 尺寸"
        ));
    }

    let total = body.len() as u32 + 8;
    let mut out = Vec::with_capacity(total as usize);
    out.extend_from_slice(b"icns");
    out.extend_from_slice(&total.to_be_bytes());
    out.extend_from_slice(&body);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_magic_and_total_length() {
        let img = RgbaImage::from_pixel(128, 128, image::Rgba([9, 9, 9, 255]));
        let bytes = encode_icns(&[(128, img)]).unwrap();

        assert_eq!(&bytes[0..4], b"icns");
        let total = u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        assert_eq!(total as usize, bytes.len());
        assert_eq!(&bytes[8..12], b"ic07");
    }

    #[test]
    fn non_standard_size_is_rejected() {
        let img = RgbaImage::from_pixel(50, 50, image::Rgba([0, 0, 0, 255]));
        assert!(encode_icns(&[(50, img)]).is_err());
    }
}
