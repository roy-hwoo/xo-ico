# 依赖关系

## 1. 直接依赖（[Cargo.toml](../Cargo.toml#L8-L17)）

| 依赖 | 版本 | 用途 |
| --- | --- | --- |
| `anyhow` | 1 | 统一错误类型（`Result` + `Context`），见 `worker.rs`、`encode/*`、`imaging/loader.rs` |
| `eframe` | 0.36 | 原生 GUI 框架，内部封装 egui / winit / wgpu，提供窗口与渲染循环 |
| `image` | 0.25（`default-features = false`，features = `png, jpeg, bmp, gif, ico`） | 图片解码/编码：读取输入、导出 PNG 子图、解析内嵌 ICO 图标 |
| `rfd` | 0.15 | 调用系统原生文件/目录选择对话框 |
| `serde` | 1（`derive`） | 为 `Settings`、`GenParams`、各枚举派生序列化 |
| `serde_json` | 1 | 设置文件（JSON）的读写 |

### 构建依赖（build-dependencies）

| 依赖 | 版本 | 用途 |
| --- | --- | --- |
| `winresource` | 0.1 | 在 [build.rs](../build.rs) 中把 `assets/XO.ico` 及版本信息嵌入 Windows 可执行文件资源（rc.exe 缺失时降级为警告，不阻断构建） |

## 2. 依赖使用分布（按模块）

| 模块 | 使用的第三方 crate |
| --- | --- |
| `main.rs` | `eframe`、`image` |
| `app.rs` | `eframe`/`egui`、`rfd`、`image`（经 `imaging`）、标准库 `mpsc` |
| `model.rs` | `serde`（无 GUI / 无 IO） |
| `settings.rs` | `serde`、`serde_json` |
| `worker.rs` | `anyhow`、`image`、标准库 `thread`/`mpsc` |
| `encode/ico.rs`、`encode/icns.rs` | `anyhow`、`image`、标准库 `io::Cursor` |
| `imaging/*` | `image`、`anyhow` |
| `ui/*` | `eframe`/`egui`、`rfd` |

要点：**编码/图像层不依赖 GUI**，`model.rs` 连 `anyhow` 都不依赖（只用 `serde`），保证可独立测试。

## 3. `image` 功能裁剪

`image` 关闭默认特性，只开启实际需要的编解码器，以减少编译时间与体积：

```
features = ["png", "jpeg", "bmp", "gif", "ico"]
```

- `png`：导出 ICO/ICNS 子图、读取 PNG 输入；
- `jpeg` / `bmp` / `gif`：读取对应输入格式；
- `ico`：解码内嵌的 `assets/XO.ico`（窗口图标）；
- 未启用 avif/webp/tiff/hdr 等，避免无谓依赖。

## 4. 间接依赖（传递依赖）概览

`eframe` 是重量级依赖，会拉入整个 GUI / GPU 栈。从构建产物（`target/*/deps`）可观察到的主要传递依赖：

```
eframe
├── egui            # 即时模式 GUI 核心
├── egui_wgpu       # egui 的 wgpu 渲染后端
│   └── wgpu ───────► wgpu_core / wgpu_hal / naga / ash / glow
├── egui_winit      # 窗口与输入（winit）
│   └── winit
├── epaint / emath / ecolor
├── accesskit       # 无障碍支持
├── egui 字体栈：read_fonts / skrifa / harfrust / font_types
└── winit 平台栈：windows / windows_sys / windows_core（Windows）

rfd
├── windows（Windows 平台）
└── webbrowser / ashpd（其他平台兜底）

image
├── png / fdeflate / flate2 / miniz_oxide / simd-adler32 / crc32fast
├── zune-jpeg / zune-core
├── gif / weezl / color_quant
└── moxcms / pxfm（色彩管理）
```

这些传递依赖正是 `target/` 目录体积巨大的直接原因，详见 [为什么 target 目录这么大](./06-why-target-is-large.md)。

## 5. 依赖方向与版本锁定

- **依赖方向单向**：`eframe` / `rfd` / `image` 是项目依赖的「叶」，业务代码不会反向影响它们；项目内部模块依赖方向见 [架构总览](./01-architecture.md#3-模块依赖关系图)。
- **版本锁定**：仓库包含 [Cargo.lock](../Cargo.lock)（约 111 KB），锁定全部传递依赖的精确版本，保证可复现构建。
- **MSRV**：`rust-version = "1.80"`，`edition = "2021"`。

## 6. 发布配置对体积的影响（[Cargo.toml](../Cargo.toml#L19-L23)）

```toml
[profile.release]
lto = true          # 链接时优化，跨 crate 内联与死代码消除
codegen-units = 1   # 单编译单元，优化更彻底（编译更慢）
strip = true        # 剥离符号表与调试信息
opt-level = "z"     # 体积优先
```

该组合使最终 release 可执行文件约 **10.5 MB**；但**中间产物（rlib/rmeta/pdb）不受此配置影响**，仍会保留在 `target/` 中，这就是目录体积远大于产物的原因。
