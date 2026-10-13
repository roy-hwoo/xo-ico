# XO ICO Generator · Code Wiki

> 离线图标生成器：把一张（或多张）位图批量转换、缩放、合成并编码为 **Windows ICO** 或 **macOS ICNS** 图标文件。

本 Wiki 面向希望理解、维护或二次开发本项目的工程师，系统性地说明项目的整体架构、模块职责、关键 API、依赖关系与运行方式，并额外解释了「源码很小、工程目录却很大」的原因。

---

## 1. 项目速览

| 项目 | 值 |
| --- | --- |
| 包名 | `ico-generator` |
| 版本 | 1.0.0 |
| 语言 / 版本 | Rust · edition 2021 · MSRV 1.80 |
| 界面框架 | `eframe` / `egui` 0.36（原生窗口） |
| 产物 | Windows `ico_generator.exe`（release 约 **10.5 MB**） |
| 平台 | Windows 为主，同时兼容 macOS / Linux 路径与字体 |
| 描述 | 离线图标生成器：批量生成 ICO / ICNS 图标 |

核心能力：

- 支持导入 `png / jpg / jpeg / bmp / gif` 图片，支持拖放与整目录导入；
- 12 种预设尺寸（16–1024）+ 自定义尺寸，自动合并去重；
- 三种缩放策略：填充（Cover）/ 适应（Fit）/ 拉伸（Stretch）；
- 两种颜色模式：保留透明 / 叠加自定义背景色（支持半透明）；
- 输出 ICO（Windows）或 ICNS（macOS）；
- 参数与主题自动持久化到用户配置目录。

## 2. Wiki 目录

| 文档 | 内容 |
| --- | --- |
| [架构总览](./01-architecture.md) | 分层结构、模块依赖、端到端数据流、线程模型 |
| [模块职责](./02-modules.md) | `app` / `model` / `settings` / `worker` / `encode` / `imaging` / `ui` 逐模块说明 |
| [关键类与函数参考](./03-api-reference.md) | 核心结构体、枚举与关键函数的签名、职责与调用点 |
| [依赖关系](./04-dependencies.md) | 直接依赖、构建依赖、间接依赖与依赖方向图 |
| [构建与运行](./05-build-and-run.md) | 环境要求、构建/运行/测试命令、发布配置 |
| [为什么 target 目录这么大](./06-why-target-is-large.md) | 工程目录 4.3 GB 的体积拆解与清理方法 |

## 3. 目录结构（源码）

```
xo-ico/
├── Cargo.toml            # 包定义与依赖
├── Cargo.lock            # 依赖版本锁定
├── build.rs              # 构建脚本：嵌入 Windows 程序图标
├── assets/
│   └── XO.ico            # 程序图标（同时内嵌进可执行文件）
├── src/
│   ├── main.rs           # 程序入口：窗口、主题、中文字体、内嵌图标
│   ├── app.rs            # 应用状态 IcoApp 与 egui 主循环
│   ├── model.rs          # 纯数据模型（无 UI / 无 IO）
│   ├── settings.rs       # 设置持久化（JSON）
│   ├── worker.rs         # 后台生成线程
│   ├── encode/           # 编码层
│   │   ├── ico.rs        # 手工拼装 ICO 容器
│   │   └── icns.rs       # 手工拼装 ICNS 容器
│   ├── imaging/          # 图像处理层
│   │   ├── loader.rs     # 读取图片 / 生成缩略图
│   │   ├── resize.rs     # cover / fit / stretch 缩放
│   │   └── background.rs # 背景色合成（alpha 混合）
│   └── ui/               # egui 界面层
│       ├── toolbar.rs    # 顶部工具栏 + 底部操作区
│       ├── options.rs    # 生成参数面板
│       ├── preview_grid.rs # 缩略图列表
│       └── dialogs.rs    # 结果对话框 + Toast 通知
└── docs/                 # 本 Code Wiki
```

## 4. 快速开始

```powershell
# 调试运行（带控制台窗口）
cargo run

# 发布构建（无控制台窗口，产物经 LTO + strip 体积优化）
cargo build --release
# 产物：target/release/ico_generator.exe

# 运行单元测试
cargo test
```

详细说明见 [构建与运行](./05-build-and-run.md)。

## 5. 一句话数据流

```
用户拖放/选择图片
      │
      ▼
  app.rs  IcoApp（UI 状态）
      │  start_generate()
      ▼
  worker.rs 后台线程  ──►  loader（解码）─► resize（缩放）
      │                                     ──► background（合成背景）
      │                                     ──► encode::ico / encode::icns（编码）
      ▼
  mpsc 通道 ProgressMsg ──► app::pump_messages() ──► save_all() 写盘
```
