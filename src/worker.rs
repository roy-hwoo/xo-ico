//! 后台生成线程：解码 → 缩放 → 背景合成 → 编码，并把结果通过通道回传。

use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};

use crate::encode::{icns, ico};
use crate::imaging::{background, loader, resize};
use crate::model::{ColorMode, GenParams, OutputFile, OutputFormat};

pub enum ProgressMsg {
    Finished(Vec<OutputFile>),
    Failed(String),
}

/// 在后台线程中批量生成图标，返回结果接收端
pub fn spawn_generate(items: Vec<(PathBuf, String)>, params: GenParams) -> Receiver<ProgressMsg> {
    let (sender, receiver) = mpsc::channel();

    std::thread::spawn(move || {
        let mut outputs = Vec::with_capacity(items.len());

        for (path, name) in items {
            match generate_one(&path, &name, &params) {
                Ok(file) => outputs.push(file),
                Err(err) => {
                    let _ = sender.send(ProgressMsg::Failed(format!("{}: {}", name, err)));
                    return;
                }
            }
        }

        let _ = sender.send(ProgressMsg::Finished(outputs));
    });

    receiver
}

fn generate_one(path: &Path, name: &str, params: &GenParams) -> anyhow::Result<OutputFile> {
    let source = loader::load_rgba(path)?;

    let mut sizes = params.normalized_sizes();
    if params.format == OutputFormat::Icns {
        sizes.retain(|size| crate::model::ICNS_SIZES.contains(size));
    }
    if sizes.is_empty() {
        anyhow::bail!("没有可用的尺寸");
    }
    // 大尺寸在前
    sizes.sort_unstable_by(|a, b| b.cmp(a));

    let mut encoded: Vec<(u32, image::RgbaImage)> = Vec::with_capacity(sizes.len());
    for size in &sizes {
        let mut resized = resize::resize(&source, *size, params.resize_mode);
        if params.color_mode == ColorMode::SolidBackground {
            resized = background::flatten_on_background(&resized, params.background);
        }
        encoded.push((*size, resized));
    }

    let stem = Path::new(name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("icon");

    let bytes = match params.format {
        OutputFormat::Ico => ico::encode_ico(&encoded)?,
        OutputFormat::Icns => icns::encode_icns(&encoded)?,
    };

    Ok(OutputFile {
        name: format!("{}.{}", stem, params.format.extension()),
        bytes,
        sizes: sizes.clone(),
    })
}
