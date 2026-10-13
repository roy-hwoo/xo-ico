# 构建与运行

## 1. 环境要求

| 项 | 要求 |
| --- | --- |
| Rust 工具链 | `>= 1.80`（`rust-version = "1.80"`，edition 2021） |
| 平台 | Windows（主要）/ macOS / Linux |
| Windows 构建图标 | 可选。需要 `rc.exe`（来自 Windows SDK）；缺失时 [build.rs](../build.rs) 仅告警，不阻断构建 |
| 图形环境 | 需要支持 wgpu 的 GPU 驱动/后端（`eframe` 默认走 wgpu） |

安装工具链（如未安装）：

```powershell
rustup default stable   # 或 rustup update
```

## 2. 构建与运行命令

```powershell
# 调试运行（保留控制台窗口，便于看日志）
cargo run

# 发布构建（无控制台窗口，体积经优化）
cargo build --release
# 产物：target\release\ico_generator.exe（约 10.5 MB）

# 运行单元测试
cargo test

# 静态检查
cargo clippy
```

发布构建请务必带 `--release`，因为 release 配置启用了 `lto + strip + opt-level="z"`，同时通过 `windows_subsystem = "windows"` 隐藏控制台（见 [main.rs#L1-L2](../src/main.rs#L1-L2)）。

> 首次构建（尤其是 debug）需要编译 `eframe`/`wgpu` 等大量依赖，耗时与磁盘占用均较大，属正常现象。

## 3. 测试覆盖点

项目内嵌单元测试（`#[cfg(test)]`），可通过 `cargo test` 运行：

| 位置 | 测试内容 |
| --- | --- |
| [main.rs](../src/main.rs#L52-L65) | 内嵌 `XO.ico` 能解码为窗口图标且 RGBA 长度匹配 |
| [app.rs](../src/app.rs#L341-L350) | `parse_custom_sizes` 兼容中英文逗号、忽略非法输入 |
| [settings.rs](../src/settings.rs#L51-L69) | 配置路径位于 `xo-ico/settings.json` |
| [encode/ico.rs](../src/encode/ico.rs#L118-L155) | ICO 头/目录项合法、BMP 自下而上、空输入被拒 |
| [encode/icns.rs](../src/encode/icns.rs#L56-L75) | ICNS 魔数与总长、非标准尺寸被拒 |
| [imaging/resize.rs](../src/imaging/resize.rs#L44-L74) | 三模式输出正方形、Fit 留透明边、Cover 填角 |
| [imaging/background.rs](../src/imaging/background.rs#L41-L72) | 不透明/半透明/全透明背景合成正确 |

## 4. 运行时行为

- **窗口**：固定 `880×500`，不可缩放、无最大化按钮，标题「XO_ICO生成器」。
- **中文字体**：启动时从系统字体候选路径加载（见 [main.rs](../src/main.rs#L292-L325)）；若全部缺失，界面中文会以缺字形式显示——属环境问题，非代码缺陷。
- **配置文件**：见 [设置持久化](./02-modules.md#4-settingsrs--设置持久化) 中的跨平台路径；删除该文件即可恢复默认设置。
- **生成流程**：底部「保存图标」按钮 →（后台线程生成，显示 spinner）→ 弹出「生成完成」窗口并自动弹出保存目录选择框。
- **拖放**：可直接把图片或文件夹拖入窗口。

## 5. 发布配置（[Cargo.toml#L19-L23](../Cargo.toml#L19-L23)）

```toml
[profile.release]
lto = true
codegen-units = 1
strip = true
opt-level = "z"
```

- `lto` + `codegen-units = 1`：跨 crate 优化、删除未用代码；
- `strip`：剥离符号/调试信息，显著减小可执行文件；
- `opt-level = "z"`：以体积为优先目标。

## 6. 分发建议

- 直接分发 `target\release\ico_generator.exe` 即可，**单文件、无运行时依赖**（Rust 静态链接系统库之外的依赖）。
- 无需附带 `assets/XO.ico`（图标已通过 `include_bytes!` 内嵌）。
- 若需分发源码，`target/` 不应打包（已在 [.gitignore](../.gitignore) 中忽略 `/target`）。

## 7. 常见问题

| 现象 | 原因与处理 |
| --- | --- |
| 构建告警「嵌入程序图标失败」 | 缺少 `rc.exe`（Windows SDK）。功能不受影响，可忽略或用默认图标 |
| 界面中文显示为方块/缺字 | 系统未找到候选中文字体；安装微软雅黑/Noto CJK 等即可 |
| 生成很慢 | 属正常，release 配置 `lto + opt-level="z"` 本身较慢；debug 首次构建更慢 |
| 磁盘占用暴涨（数 GB） | 这是 `target/` 构建缓存的正常结果，说明见 [下一节](./06-why-target-is-large.md) |
