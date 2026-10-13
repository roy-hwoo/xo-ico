//! 设置持久化：主题与生成参数写入用户配置目录下的 JSON 文件。

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::model::GenParams;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Settings {
    pub dark: bool,
    pub params: GenParams,
    #[serde(default)]
    pub custom_sizes: String,
}

/// 配置文件路径（跨平台）
fn config_path() -> Option<PathBuf> {
    let base = if cfg!(target_os = "windows") {
        std::env::var_os("APPDATA").map(PathBuf::from)
    } else if cfg!(target_os = "macos") {
        std::env::var_os("HOME").map(|home| PathBuf::from(home).join("Library/Application Support"))
    } else {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
    }?;
    Some(base.join("xo-ico").join("settings.json"))
}

impl Settings {
    pub fn load() -> Self {
        config_path()
            .and_then(|path| std::fs::read_to_string(path).ok())
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        if let Some(path) = config_path() {
            if let Some(dir) = path.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            if let Ok(json) = serde_json::to_string_pretty(self) {
                let _ = std::fs::write(path, json);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_path_uses_xo_ico_folder() {
        let path = config_path().expect("应能解析配置目录");
        assert_eq!(
            path.file_name().and_then(|n| n.to_str()),
            Some("settings.json")
        );
        assert_eq!(
            path.parent()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str()),
            Some("xo-ico"),
            "配置文件应位于 xo-ico 目录下"
        );
    }
}
