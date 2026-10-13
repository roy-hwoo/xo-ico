# 主要模块职责

本文逐模块说明职责、对外接口与实现要点。文件链接指向源码位置。

---

## 1. `main.rs` — 程序入口

**职责**：进程入口、窗口构建、主题初始化、系统中文字体装载、内嵌程序图标解码。

要点：

- release 构建时通过 `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]` 隐藏控制台窗口（[main.rs#L1-L2](../src/main.rs#L1-L2)）。
- 通过 `eframe::ViewportBuilder` 固定窗口：标题「XO_ICO生成器」、初始 `880×500`、不可缩放、无最大化按钮（[main.rs#L18-L22](../src/main.rs#L18-L22)）。
- `const APP_ICON: &[u8] = include_bytes!("../assets/XO.ico")` 把图标字节编译进二进制，运行时用 `image` 解码为 egui 的 `IconData` 作为窗口/任务栏图标；解码失败不阻断启动（[main.rs#L15, #L40-L50](../src/main.rs#L40-L50)）。
- `install_cjk_font`：egui 内置字体不含中文字形，此处按 Windows/macOS/Linux 候选路径**顺序查找**系统字体（微软雅黑、黑体、宋体、PingFang、Noto CJK 等），找到第一个可用者注入为 Proportional 与 Monospace 家族的首选字体（[main.rs#L292-L325](../src/main.rs#L292-L325)）。
- 含测试 `embedded_icon_decodes`，验证内嵌 ICO 可被解码且 RGBA 长度与宽高匹配。

---

## 2. `app.rs` — 应用状态与主循环

**职责**：持有全部 UI 状态，实现 `eframe::App`，编排「添加图片 → 生成 → 保存」流程，管理消息轮询与 Toast。

关键内容：

- `struct ImageEntry { desc: ImageDesc, texture: egui::TextureHandle }`：列表项 = 图片描述 + 已上传的缩略图纹理（[app.rs#L16-L19](../src/app.rs#L16-L19)）。
- `struct IcoApp`：核心状态容器（[app.rs#L21-L35](../src/app.rs#L21-L35)）：

  | 字段 | 含义 |
  | --- | --- |
  | `images: Vec<ImageEntry>` | 待处理图片列表 |
  | `params: GenParams` | 生成参数 |
  | `dark: bool` | 主题（深/浅） |
  | `generating: bool` | 是否正在后台生成 |
  | `results: Vec<OutputFile>` | 生成结果 |
  | `pending_save: bool` | 生成完成后是否自动进入保存流程 |
  | `show_result: bool` | 是否显示结果对话框 |
  | `custom_sizes: String` | 自定义尺寸输入文本 |
  | `hex_input: String` | Hex 颜色输入文本 |
  | `toast: Option<(String, bool, f64)>` | （消息, 是否错误, 创建时间） |
  | `receiver: Option<Receiver<ProgressMsg>>` | 后台线程结果通道 |

方法分三类：

- **图片管理**：`add_paths`（过滤扩展名 → 解码 → 生成缩略图 → 上传纹理 → 计数并 Toast）。
- **生成与保存**：`effective_sizes`（预设与自定义合并、过滤 `0<size<=4096`、升序去重）、`start_generate_and_save`、`start_generate`、`save_all`。
- **主题与通知**：`toggle_theme`、`toast_msg`、`persist`。

主循环实现 `eframe::App::ui`（[app.rs#L257-L281](../src/app.rs#L257-L281)）每帧顺序执行：

1. `pump_messages`：非阻塞拉取后台结果；
2. `handle_dropped_files`：处理窗口拖放的文件；
3. 在 `CentralPanel + ScrollArea` 内依次绘制 `toolbar::draw` → `preview_grid::draw` → `options::draw` → `toolbar::draw_bottom`；
4. `dialogs::draw` 绘制结果对话框与 Toast；
5. 若生成中则安排 60ms 后重绘。

工具函数：`rgba_to_color_image`（RGBA8 → egui `ColorImage`）、`parse_custom_sizes`（按英文/中文逗号与空白切分并解析，含测试）。

---

## 3. `model.rs` — 纯数据模型

**职责**：定义全部枚举、常量与结构体，**不依赖 UI 与文件系统**，便于单元测试与跨层复用。

| 类型 | 说明 |
| --- | --- |
| `enum OutputFormat { Ico, Icns }` | 输出格式；提供 `extension()`、`label()` |
| `enum ColorMode { Transparent, SolidBackground }` | 颜色模式 |
| `enum ResizeMode { Cover, Fit, Stretch }` | 缩放策略；提供中文 `label()` |
| `struct GenParams` | 生成参数：`sizes / format / color_mode / background([u8;4]) / resize_mode` |
| `struct ImageDesc` | 图片描述：`path: PathBuf`、`name: String` |
| `struct OutputFile` | 输出结果：`name / bytes / sizes` |

常量：

- `PRESET_SIZES: [u32; 12] = [16,24,32,48,64,72,96,128,256,512,768,1024]`
- `ICNS_SIZES: [u32; 7] = [16,32,64,128,256,512,1024]`

`GenParams::default()` 的默认尺寸为 `[16,24,32,48,64,256]`，并**按平台选择默认格式**：macOS → ICNS，其它 → ICO。

`GenParams::normalized_sizes()` 归一化尺寸：过滤非法值（`0< s <=4096`）、升序、去重（[model.rs#L92-L105](../src/model.rs#L92-L105)）。

---

## 4. `settings.rs` — 设置持久化

**职责**：把主题与生成参数以 JSON 形式读写到用户配置目录。

- `struct Settings { dark, params: GenParams, custom_sizes }`，`custom_sizes` 带 `#[serde(default)]` 以兼容旧配置（[settings.rs#L9-L15](../src/settings.rs#L9-L15)）。
- `config_path()` 跨平台解析（[settings.rs#L18-L29](../src/settings.rs#L18-L29)）：

  | 平台 | 路径 |
  | --- | --- |
  | Windows | `%APPDATA%\xo-ico\settings.json` |
  | macOS | `~/Library/Application Support/xo-ico/settings.json` |
  | Linux | `$XDG_CONFIG_HOME/xo-ico/settings.json` 或 `~/.config/xo-ico/settings.json` |

- `load()`：任何环节失败（无路径/读失败/解析失败）都回退为 `Default`，保证启动不崩。
- `save()`：自动创建父目录，`to_string_pretty` 后写盘，错误被忽略（尽力而为）。

---

## 5. `worker.rs` — 后台生成线程

**职责**：在独立线程完成「解码 → 缩放 → 背景合成 → 编码」批处理，并通过通道回传。

- `enum ProgressMsg { Finished(Vec<OutputFile>), Failed(String) }`。
- `spawn_generate(items, params) -> Receiver<ProgressMsg>`：创建通道并 `thread::spawn`，逐张调用 `generate_one`；**遇首个失败立即返回 `Failed`**（fail-fast），全部成功才发 `Finished`（[worker.rs#L16-L36](../src/worker.rs#L16-L36)）。
- `generate_one(path, name, params)` 的算法步骤（[worker.rs#L38-L75](../src/worker.rs#L38-L75)）：
  1. `loader::load_rgba` 解码源图；
  2. 取 `normalized_sizes`；若输出 ICNS 则**仅保留 `ICNS_SIZES` 中的尺寸**；
  3. 尺寸降序排序（大图在前）；
  4. 逐尺寸 `resize`，纯色模式再 `flatten_on_background`；
  5. 按格式分发 `encode_ico` / `encode_icns`；
  6. 组装 `OutputFile`（文件名取源文件 stem + 目标扩展名）。

---

## 6. `encode/` — 容器编码层

### 6.1 `encode/ico.rs`

**职责**：手工拼装 ICO 容器（`ICONDIR + ICONDIRENTRY*N + 数据`），避免额外第三方编码器。

- 入口 `encode_ico(&[(u32, RgbaImage)]) -> Result<Vec<u8>>`。
- 子图策略：`size < 256` 写 **32 位 BGRA BMP**（`BITMAPINFOHEADER` + XOR 位图 + AND 掩码）；`size >= 256` 写 **PNG**（现代 Windows 通行做法），阈值常量 `PNG_THRESHOLD = 256`。
- 目录项统一 32 位位深；尺寸为 256 时按规范把宽/高字节写 `0`。
- `encode_bmp32` 细节：自下而上写行、BGRA 顺序、行天然 4 字节对齐；AND 掩码 1 位/像素（alpha < 128 视为透明）。
- 测试覆盖：头部/目录项合法性、自下而上像素顺序、空输入被拒。

### 6.2 `encode/icns.rs`

**职责**：手工拼装 ICNS 容器（`'icns' + 总长(大端) + [类型码(4) + 项长(大端) + PNG 数据]`）。

- 尺寸 → 类型码：16→`icp4`、32→`icp5`、64→`icp6`、128→`ic07`、256→`ic08`、512→`ic09`、1024→`ic10`。
- 非标准尺寸条目**被跳过**；若最终一条都没有则报错。
- 每个子图均以 PNG 编码后内嵌。

---

## 7. `imaging/` — 图像处理层

### 7.1 `imaging/loader.rs`

- `SUPPORTED_EXTENSIONS` / `IMAGE_DIALOG_FILTER = ["png","jpg","jpeg","bmp","gif"]`。
- `load_rgba(path)`：`image::open` 后统一转 `RgbaImage`。
- `thumbnail(img, max_side)`：按最长边等比缩放（`Triangle` 滤波），已足够小则直接克隆。
- `is_image_path(path)`：大小写不敏感地按扩展名判断。

### 7.2 `imaging/resize.rs`

`resize(src, size, mode) -> RgbaImage` 输出恒为 `size×size`：

- **Stretch**：`Lanczos3` 直接拉伸到 `size×size`（可能变形）；
- **Cover**：先按短边中心裁剪成正方形再缩放（铺满画布）；
- **Fit**：等比缩放后在 `size×size` 透明画布上居中，四周留透明边；
- 空输入（宽/高/尺寸为 0）返回空尺寸兜底。
- 测试覆盖三种模式输出为正方形、Fit 保留透明边、Cover 填满角落。

### 7.3 `imaging/background.rs`

`flatten_on_background(src, bg: [u8;4]) -> RgbaImage`：把带透明度的源图按 **source-over** 非预乘模型叠加到自定义背景色上，背景 alpha 支持半透明。

- 合成 alpha：`out_a = sa + ba*(1-sa)`；
- 合成颜色：`out_c = (src_c*sa + bg_c*ba*(1-sa)) / out_a`（结果钳制到 255）；
- `out_a == 0` 时写入全透明像素。
- 测试覆盖不透明背景替换透明区、半透明源混合、全透明背景保持透明。

---

## 8. `ui/` — egui 界面层

界面被拆为四个 `draw` 函数，均由 [app.rs](../src/app.rs#L263-L275) 的主循环调用。

| 文件 | 绘制内容 |
| --- | --- |
| [toolbar.rs](../src/ui/toolbar.rs) | 顶部工具栏（添加图像 / 添加目录 / 全部清除 / 计数 / 主题切换）与底部操作区（「保存图标」大按钮 + 生成中 spinner）；`collect_images_in_dir` 收集目录内受支持图片并按名排序 |
| [options.rs](../src/ui/options.rs) | 生成参数面板：预设尺寸多选（全选/取消全选）、自定义尺寸输入、输出格式、缩放模式、颜色模式与背景色（取色器 + 4 个预设色 + Hex 输入）；`parse_hex` 支持 `#RGB` / `#RRGGBB` / `#RRGGBBAA` |
| [preview_grid.rs](../src/ui/preview_grid.rs) | 横向滚动的缩略图列表，空态提示「拖放图像到此处」；每项可单独「移除」；`short_name` 用于超长文件名截断 |
| [dialogs.rs](../src/ui/dialogs.rs) | 「生成完成」结果窗口（列出文件名与尺寸）+ 居中底部 Toast 通知（错误红/成功绿，3 秒自动消失） |

**交互约定**：「生成图标」与「保存文件」已合并为底部唯一入口「保存图标」——点击后先生成、成功后自动弹出保存目录选择。
