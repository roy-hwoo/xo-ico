# 架构总览

## 1. 设计目标与原则

本项目是一个**单二进制、离线、无网络**的桌面小工具。架构上遵循以下原则：

- **前后端解耦**：UI 层只负责交互与状态展示；真正的图像处理与编码放在独立的后台线程中执行，避免阻塞渲染循环。
- **纯数据模型**：`model.rs` 不依赖 UI 与文件系统，所有枚举/结构体可被单元测试直接使用。
- **零重型编码器依赖**：ICO 与 ICNS 容器均由项目**手工拼装**（`encode/`），仅借用 `image` crate 编解码 PNG/BMP 像素，避免引入额外的图标编码库。
- **可移植**：配置目录、中文字体、默认输出格式均按 Windows / macOS / Linux 分别处理。

## 2. 分层结构

项目在逻辑上分为四层，自顶向下为：

```
┌─────────────────────────────────────────────────────────────┐
│  入口层  main.rs                                             │
│    · 创建 eframe 原生窗口 / 设置标题与图标                    │
│    · 加载系统中文字体（egui 内置字体不含中文）                │
└─────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────────────────────────────────────┐
│  应用层  app.rs (IcoApp)  +  ui/*                            │
│    · 持有全部 UI 状态（图片列表、参数、生成结果、提示）        │
│    · eframe::App::ui 主循环：拉取消息 → 处理拖放 → 绘制界面    │
│    · ui/{toolbar,options,preview_grid,dialogs} 各绘一块区域    │
└─────────────────────────────────────────────────────────────┘
                              │  提交任务 / 接收结果
┌─────────────────────────────────────────────────────────────┐
│  任务层  worker.rs（后台线程）                                │
│    · spawn_generate：解码→缩放→合成→编码，结果经通道回传      │
│    · ProgressMsg::{Finished, Failed}                         │
└─────────────────────────────────────────────────────────────┘
                              │  调用纯函数
┌─────────────────────────────────────────────────────────────┐
│  算法层  imaging/* (图像处理)  +  encode/* (容器编码)          │
│    · loader / resize / background      — 纯图像算法           │
│    · ico / icns                        — 纯字节拼装           │
└─────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────────────────────────────────────┐
│  支撑层  model.rs（数据模型）  +  settings.rs（持久化）        │
│    · model   定义枚举/结构体，被上面所有层引用                │
│    · settings 读写用户配置目录下的 JSON                       │
└─────────────────────────────────────────────────────────────┘
```

依赖方向是**单向向下**的：`main → app → (ui, worker) → (imaging, encode) → model`。`settings` 依赖 `model`，被 `app` 使用。不存在反向依赖或循环依赖。

## 3. 模块依赖关系图

```
                 ┌──────────┐
                 │ main.rs  │
                 └────┬─────┘
                      ▼
                 ┌──────────┐   ┌───────────────┐
                 │  app.rs  │◄──│ settings.rs   │
                 └──┬────┬──┘   └───────┬───────┘
          ┌─────────┘    └───────┐       │
          ▼                      ▼       ▼
    ┌──────────┐          ┌──────────┐ ┌────────┐
    │  ui/*    │          │ worker.rs│ │model.rs│
    └────┬─────┘          └────┬─────┘ └────┬───┘
         │                     │            ▲
         ▼                     ▼            │
   ┌───────────┐        ┌───────────┐       │
   │ imaging/* │        │ encode/*  │       │
   └─────┬─────┘        └─────┬─────┘       │
         └──────────┬─────────┘             │
                    ▼                       │
              （image / anyhow）────────────┘
```

关键点：

- `ui/*` 与 `worker.rs` 都只通过 `app.rs` 暴露的状态与方法协作，UI 不直接调用编码逻辑。
- `model.rs` 是被引用的「叶子」，不引用任何业务模块。

## 4. 端到端数据流

以「用户点击『保存图标』」为例，完整链路如下：

| 步骤 | 位置 | 动作 |
| --- | --- | --- |
| 1 | [toolbar.rs](../src/ui/toolbar.rs) `draw_bottom` | 按钮点击 → `app.start_generate_and_save(ctx)` |
| 2 | [app.rs](../src/app.rs) `start_generate` | 校验图片非空、尺寸非空；合并预设+自定义尺寸；`persist()` 落盘配置 |
| 3 | [app.rs](../src/app.rs) | 调用 `worker::spawn_generate(items, params)`，置 `generating = true`，`request_repaint()` |
| 4 | [worker.rs](../src/worker.rs) 后台线程 | 对每张图片执行 `generate_one`： |
| 4.1 | [loader.rs](../src/imaging/loader.rs) | `load_rgba` 解码为 RGBA8；`normalized_sizes` 过滤尺寸 |
| 4.2 | [resize.rs](../src/imaging/resize.rs) | 按 `ResizeMode` 缩放到 `size×size` |
| 4.3 | [background.rs](../src/imaging/background.rs) | 若为纯色背景模式，`flatten_on_background` 做 alpha 合成 |
| 4.4 | [ico.rs](../src/encode/ico.rs) / [icns.rs](../src/encode/icns.rs) | 拼装容器字节 |
| 5 | worker → 通道 | 全部成功后发送 `ProgressMsg::Finished(Vec<OutputFile>)`，任一失败则 `Failed(String)` |
| 6 | [app.rs](../src/app.rs) `pump_messages` | 每帧 `try_recv` 拉取消息；`Finished` 后置 `show_result`；若 `pending_save` 则调用 `save_all` |
| 7 | [app.rs](../src/app.rs) `save_all` | 弹目录选择框，逐个 `std::fs::write` 写出 |

## 5. 线程与消息模型

- **UI 主线程**：`eframe` 事件循环驱动 `IcoApp::ui`。
- **工作线程**：每次生成通过 `std::thread::spawn` 创建**一次性**后台线程（见 [worker.rs](../src/worker.rs#L16-L36)），不使用线程池。
- **通信**：标准库 `std::sync::mpsc` 单向通道，工作线程持有 `Sender`，UI 持有 `Receiver`。
- **轮询**：UI 侧非阻塞 `try_recv`：
  - `Empty` → 保留通道，本轮结束；
  - `Finished/Failed` → 处理结果并丢弃通道；
  - `Disconnected` → 线程异常结束，重置 `generating`。
- **重绘节流**：生成中每 60ms `request_repaint_after` 一次（见 [app.rs](../src/app.rs#L277-L279)）；Toast 显示期间每 250ms 重绘一次。
- **一次性设计的好处**：逻辑简单、无共享可变状态、天然规避数据竞争；代价是每次生成都要新建线程（对轻量批处理足够）。

## 6. 状态与持久化

- 运行期状态集中在 `IcoApp`（见 [app.rs](../src/app.rs#L21-L35)）。
- 需要跨会话保留的仅三项：`dark`（主题）、`params`（生成参数）、`custom_sizes`（自定义尺寸文本），由 `Settings` 负责读写（见 [settings.rs](../src/settings.rs#L31-L48)）。
- 触发持久化的时机：切换主题、开始生成（`start_generate` 中调用 `persist()`）。

## 7. 关键设计取舍

| 决策 | 原因 |
| --- | --- |
| 手工拼装 ICO / ICNS | 避免第三方编码器依赖，且可精确控制位深与目录项一致性（旧 JS 版曾出现位深不一致问题） |
| `opt-level = "z"` + `lto` + `strip` | 面向分发的体积优先；换来 release 构建较慢 |
| 用系统字体而非内嵌字体 | 内嵌 CJK 字体会让二进制体积激增数 MB，改为运行时从系统路径查找 |
| 无第三方 async/runtime | 单线程任务 + 标准库通道已足够，降低复杂度与体积 |
