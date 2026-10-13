//! 纯数据模型：不依赖 UI / 文件系统，便于单元测试。

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// 输出格式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutputFormat {
    Ico,
    Icns,
}

impl OutputFormat {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Ico => "ico",
            Self::Icns => "icns",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Ico => "ICO (Windows)",
            Self::Icns => "ICNS (macOS)",
        }
    }
}

/// 颜色模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ColorMode {
    /// 32 位，保留透明通道
    Transparent,
    /// 叠加自定义背景色（支持半透明）
    SolidBackground,
}

/// 缩放模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResizeMode {
    Cover,
    Fit,
    Stretch,
}

impl ResizeMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Cover => "填充（裁剪居中，铺满画布）",
            Self::Fit => "适应（保持比例，留白边）",
            Self::Stretch => "拉伸（强制拉伸，可能变形）",
        }
    }
}

/// 预设尺寸（与旧版一致）
pub const PRESET_SIZES: [u32; 12] = [16, 24, 32, 48, 64, 72, 96, 128, 256, 512, 768, 1024];

/// ICNS 支持的尺寸
pub const ICNS_SIZES: [u32; 7] = [16, 32, 64, 128, 256, 512, 1024];

/// 生成参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenParams {
    /// 勾选的预设尺寸
    pub sizes: Vec<u32>,
    pub format: OutputFormat,
    pub color_mode: ColorMode,
    /// 背景色 RGBA（alpha 支持半透明）
    pub background: [u8; 4],
    pub resize_mode: ResizeMode,
}

impl Default for GenParams {
    fn default() -> Self {
        Self {
            sizes: vec![16, 24, 32, 48, 64, 256],
            // macOS 默认输出 ICNS，其它平台默认 ICO
            format: if cfg!(target_os = "macos") {
                OutputFormat::Icns
            } else {
                OutputFormat::Ico
            },
            color_mode: ColorMode::Transparent,
            background: [255, 255, 255, 255],
            resize_mode: ResizeMode::Cover,
        }
    }
}

impl GenParams {
    /// 归一化尺寸列表：过滤非法值、升序、去重
    pub fn normalized_sizes(&self) -> Vec<u32> {
        let mut sizes: Vec<u32> = self
            .sizes
            .iter()
            .copied()
            .filter(|s| *s > 0 && *s <= 4096)
            .collect();
        sizes.sort_unstable();
        sizes.dedup();
        sizes
    }
}

/// 一张待处理的图片（纯描述，不含 UI 资源）
#[derive(Debug, Clone)]
pub struct ImageDesc {
    pub path: PathBuf,
    pub name: String,
}

/// 生成结果
#[derive(Debug, Clone)]
pub struct OutputFile {
    pub name: String,
    pub bytes: Vec<u8>,
    pub sizes: Vec<u32>,
}
