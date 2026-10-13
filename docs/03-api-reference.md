# 关键类与函数参考

本文列出项目中最关键的公开类型与函数，含签名、职责与调用关系。所有链接指向源码。

---

## 一、数据模型（[model.rs](../src/model.rs)）

### 枚举

```rust
pub enum OutputFormat { Ico, Icns }
impl OutputFormat {
    pub fn extension(self) -> &'static str; // "ico" | "icns"
    pub fn label(self) -> &'static str;     // "ICO (Windows)" | "ICNS (macOS)"
}

pub enum ColorMode { Transparent, SolidBackground }
pub enum ResizeMode { Cover, Fit, Stretch }
impl ResizeMode { pub fn label(self) -> &'static str; }
```

### `struct GenParams`（[model.rs#L64-L105](../src/model.rs#L64-L105)）

```rust
pub struct GenParams {
    pub sizes: Vec<u32>,          // 勾选的预设尺寸
    pub format: OutputFormat,
    pub color_mode: ColorMode,
    pub background: [u8; 4],      // 背景色 RGBA
    pub resize_mode: ResizeMode,
}
impl Default for GenParams { /* sizes=[16,24,32,48,64,256]; format 按平台默认 */ }
impl GenParams {
    /// 过滤非法值、升序、去重
    pub fn normalized_sizes(&self) -> Vec<u32>;
}
```

### 其他结构体与常量

```rust
pub struct ImageDesc { pub path: PathBuf, pub name: String }
pub struct OutputFile { pub name: String, pub bytes: Vec<u8>, pub sizes: Vec<u32> }

pub const PRESET_SIZES: [u32; 12] = [16,24,32,48,64,72,96,128,256,512,768,1024];
pub const ICNS_SIZES:   [u32; 7]  = [16,32,64,128,256,512,1024];
```

---

## 二、应用状态（[app.rs](../src/app.rs)）

### `struct ImageEntry`

```rust
pub struct ImageEntry {
    pub desc: ImageDesc,
    pub texture: egui::TextureHandle, // 已上传的缩略图纹理
}
```

### `struct IcoApp`（[app.rs#L21-L35](../src/app.rs#L21-L35)）

核心状态容器，字段说明见 [模块职责 · app.rs](./02-modules.md#2-apprs--应用状态与主循环)。

关键方法：

| 方法 | 签名 | 职责 |
| --- | --- | --- |
| `new` | `fn new(cc: &eframe::CreationContext) -> Self` | 加载设置、装载中文字体、设置主题、初始化默认状态 |
| `add_paths` | `fn add_paths(&mut self, ctx: &egui::Context, paths: &[PathBuf])` | 批量导入图片（过滤类型→解码→缩略图→上传纹理），并 Toast 统计结果 |
| `effective_sizes` | `fn effective_sizes(&self) -> Vec<u32>` | 预设 + 自定义尺寸合并、过滤 `0<size<=4096`、升序去重 |
| `start_generate_and_save` | `fn start_generate_and_save(&mut self, ctx: &egui::Context)` | 「保存图标」唯一入口，委托 `start_generate(ctx, true)` |
| `start_generate` | `fn start_generate(&mut self, ctx, save_after: bool)` | 参数校验、持久化设置、`spawn_generate`、置生成中状态 |
| `save_all` | `fn save_all(&mut self, ctx: &egui::Context)` | 弹目录选择框并逐个写盘，统计成功数并 Toast |
| `toggle_theme` | `fn toggle_theme(&mut self, ctx: &egui::Context)` | 切换深浅主题并持久化 |
| `toast_msg` | `fn toast_msg(&mut self, text: impl Into<String>, is_error: bool, ctx)` | 记录一条带时间戳的提示 |
| `persist` | `fn persist(&self)` | 把 `dark/params/custom_sizes` 写回 `Settings` |
| `pump_messages` | `fn pump_messages(&mut self, ctx) -> ()` *(私有)* | 非阻塞拉取后台结果，处理 `Finished/Failed/Empty/Disconnected` |
| `handle_dropped_files` | `fn handle_dropped_files(&mut self, ctx)` *(私有)* | 把窗口拖入的文件转交 `add_paths` |

### 实现 `eframe::App`

```rust
impl eframe::App for IcoApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame); // 每帧主循环
}
```

### 辅助函数

```rust
fn theme_preference(dark: bool) -> egui::ThemePreference;
fn install_cjk_font(ctx: &egui::Context);
fn rgba_to_color_image(image: &image::RgbaImage) -> egui::ColorImage;
pub fn parse_custom_sizes(text: &str) -> Vec<u32>;
```

---

## 三、后台生成（[worker.rs](../src/worker.rs)）

```rust
pub enum ProgressMsg {
    Finished(Vec<OutputFile>),
    Failed(String),
}

/// 在后台线程中批量生成图标，返回结果接收端
pub fn spawn_generate(
    items: Vec<(PathBuf, String)>,
    params: GenParams,
) -> Receiver<ProgressMsg>;

/// 单张图片的完整流水线（私有）
fn generate_one(path: &Path, name: &str, params: &GenParams) -> anyhow::Result<OutputFile>;
```

调用关系：`IcoApp::start_generate` → `spawn_generate` →（线程内）`generate_one` → `loader::load_rgba` / `resize::resize` / `background::flatten_on_background` / `ico::encode_ico` | `icns::encode_icns`。

---

## 四、编码层

### `encode/ico.rs`

```rust
pub fn encode_ico(images: &[(u32, RgbaImage)]) -> Result<Vec<u8>>;
fn encode_png(img: &RgbaImage) -> Result<Vec<u8>>;   // size >= 256 的子图
fn encode_bmp32(img: &RgbaImage) -> Result<Vec<u8>>; // size < 256 的子图
```

- `PNG_THRESHOLD = 256` 决定子图使用 PNG 还是 BMP。
- 空输入返回 `Err("没有可编码的图像")`。

### `encode/icns.rs`

```rust
pub fn encode_icns(images: &[(u32, RgbaImage)]) -> Result<Vec<u8>>;
fn type_code(size: u32) -> Option<[u8; 4]>; // 尺寸 → ICNS 类型码
```

- 非标准尺寸条目跳过；无有效条目时返回 `Err("ICNS 仅支持 16/32/64/128/256/512/1024 尺寸")`。

---

## 五、图像处理层（[imaging/](../src/imaging)）

### `loader.rs`

```rust
pub const SUPPORTED_EXTENSIONS: &[&str] = &["png","jpg","jpeg","bmp","gif"];
pub const IMAGE_DIALOG_FILTER: &[&str] = &["png","jpg","jpeg","bmp","gif"];

pub fn load_rgba(path: &Path) -> Result<RgbaImage>;             // 解码为 RGBA8
pub fn thumbnail(img: &RgbaImage, max_side: u32) -> RgbaImage;  // 最长边等比缩略图
pub fn is_image_path(path: &Path) -> bool;                      // 扩展名判断（忽略大小写）
```

### `resize.rs`

```rust
pub fn resize(src: &RgbaImage, size: u32, mode: ResizeMode) -> RgbaImage;
fn fit_scale(src: &RgbaImage, size: u32) -> (u32, u32); // 等比缩放目标尺寸
```

输出恒为 `size×size`；`Stretch` 用 Lanczos3 拉伸，`Cover` 中心裁剪后缩放，`Fit` 居中留透明边。

### `background.rs`

```rust
/// source-over 非预乘 alpha 混合，叠加到自定义背景色
pub fn flatten_on_background(src: &RgbaImage, bg: [u8; 4]) -> RgbaImage;
```

---

## 六、设置持久化（[settings.rs](../src/settings.rs)）

```rust
pub struct Settings { pub dark: bool, pub params: GenParams, pub custom_sizes: String }
fn config_path() -> Option<PathBuf>; // 跨平台配置路径
impl Settings {
    pub fn load() -> Self;  // 失败回退默认值
    pub fn save(&self);     // 自动建目录后写 JSON
}
```

---

## 七、UI 层（[ui/](../src/ui)）

统一约定：每个子模块暴露一个 `pub fn draw(app: &mut IcoApp, ui: &mut egui::Ui)` 供主循环调用。

```rust
// toolbar.rs
pub fn draw(app: &mut IcoApp, ui: &mut egui::Ui);         // 顶部工具栏
pub fn draw_bottom(app: &mut IcoApp, ui: &mut egui::Ui);  // 底部「保存图标」操作区
fn collect_images_in_dir(dir: &Path) -> Vec<PathBuf>;

// options.rs
pub fn draw(app: &mut IcoApp, ui: &mut egui::Ui);
fn draw_color_mode(app: &mut IcoApp, ui: &mut egui::Ui);
fn parse_hex(text: &str) -> Option<[u8; 4]>; // #RGB / #RRGGBB / #RRGGBBAA

// preview_grid.rs
pub fn draw(app: &mut IcoApp, ui: &mut egui::Ui);
fn short_name(name: &str, max_chars: usize) -> String;

// dialogs.rs
pub fn draw(app: &mut IcoApp, ctx: &egui::Context);
```

---

## 八、调用速查（谁调用谁）

| 调用方 | 被调用方 |
| --- | --- |
| `main()` | `IcoApp::new`、`load_window_icon`、`install_cjk_font` |
| `IcoApp::ui` | `pump_messages`、`handle_dropped_files`、`ui::*::draw` |
| `ui::toolbar::draw` | `IcoApp::add_paths`、`IcoApp::toggle_theme`、`collect_images_in_dir` |
| `ui::toolbar::draw_bottom` | `IcoApp::start_generate_and_save` |
| `IcoApp::start_generate` | `effective_sizes`、`persist`、`worker::spawn_generate` |
| `IcoApp::pump_messages` | `save_all` / `toast_msg` |
| `worker::generate_one` | `loader` / `resize` / `background` / `encode::ico` / `encode::icns` |
