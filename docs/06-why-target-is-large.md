# 为什么 `target` 目录这么大？

> 结论先行：**大的是编译产物（`target/`），不是源码。** 源码 `src/` 仅约 **50 KB**，而 `target/` 达 **约 4.3 GB**。可执行文件只有 10.5 MB，是因为它经过了 `strip + lto + opt-level="z"` 的「瘦身」；而 `target/` 里保留的是编译过程中产生的**全部中间文件**，二者不是一回事。

## 1. 实测体积拆解

对本机仓库实测（`Get-ChildItem -Recurse | Measure-Object Length -Sum`）：

| 目录 | 体积 | 说明 |
| --- | --- | --- |
| `src/` | **0.05 MB** | 全部源代码（十几个 `.rs` 文件） |
| `assets/` | 0.08 MB | 程序图标 `XO.ico` |
| `target/` | **4,286 MB（≈4.3 GB）** | 构建产物缓存 |
| ├─ `target/debug/` | 3,210 MB | 调试构建产物 |
| │  └─ `debug/deps/` | 2,434 MB | 依赖库的中间产物 |
| └─ `target/release/` | 1,077 MB | 发布构建产物 |
| 　 └─ `release/deps/` | 948 MB | 依赖库的中间产物 |
| **`target/` 文件总数** | **6,201 个** | 大量小文件 + 若干巨型文件 |
| **最终可执行文件** | **10.51 MB** | `target/release/deps/ico_generator.exe` |

> 对比：真正要交付的东西只有 **10.51 MB**，其余约 4.28 GB 都是可再生的中间缓存。

## 2. 单个巨型文件示例（`target/` 内 Top 10）

| 体积 | 文件 | 类型 |
| --- | --- | --- |
| 237.15 MB | `debug/ico_generator.pdb` | **调试符号文件** |
| 139.61 MB | `debug/deps/ico_generator-*.pdb` | 调试符号 |
| 134.61 MB | `debug/deps/ico_generator-*.pdb` | 调试符号 |
| 115.82 MB | `debug/deps/libwgpu_core-*.rlib` | 依赖静态库（中间产物） |
| 110.87 MB | `debug/deps/ico_generator-*.pdb` | 调试符号 |
| 88.23 MB | `debug/deps/libnaga-*.rlib` | 依赖静态库 |
| 87.32 MB | `debug/deps/libwindows-*.rlib` | 依赖静态库 |
| 77.22 MB | `debug/deps/libread_fonts-*.rlib` | 依赖静态库 |
| 74.44 MB | `debug/deps/libwgpu_hal-*.rlib` | 依赖静态库 |
| 73.15 MB | `debug/deps/libash-*.rlib` | 依赖静态库 |

## 3. 体积为何会膨胀到 GB 级

`cargo build` 的工作方式决定了 `target/` 会保留「构建过程中的一切」，主要包括：

1. **每个依赖都生成 `.rlib`（静态库）+ `.rmeta`（元数据）**
   `eframe` 会拉入整个 GUI/GPU 栈：`egui`、`wgpu`、`wgpu_core`、`wgpu_hal`、`naga`、`ash`、`winit`、`read_fonts`、`skrifa`、`windows`……成百上千个 crate，每个都要留下体积可观的中间产物，`libwgpu_core.rlib` 一个就 115 MB。

2. **调试构建保留完整调试信息**
   debug 配置**不剥离**符号，会生成巨型 `.pdb`（本项目单个 PDB 就高达 **237 MB**）；`opt-level=0` 下代码也未做内联与去重，进一步放大 `.rlib`。

3. **`.d` 依赖清单文件**
   每个 crate 都有对应的 `.d` 文件（记录源文件依赖），数量多但单个很小，是「6201 个文件」的主要贡献者之一。

4. **同一 crate 的多个构建变体**
   同一依赖可能因不同 feature/hash 被编译出多份（文件名后缀哈希不同，如多个 `ico_generator-*.pdb`），debug 与 release 又各存一套，占用翻倍。

5. **增量编译与 build-script 输出**
   `target/*/build/<crate>-<hash>/output|stderr` 以及增量编译中间态同样会被缓存。

6. **`strip`/`lto` 只作用于最终产物**
   [Cargo.toml](../Cargo.toml#L19-L23) 的 `strip = true`、`lto = true`、`opt-level = "z"` 只让**最终 exe** 变小（→10.5 MB），**不会**让中间产物 `.rlib`/`.rmeta`/`.pdb` 变小，所以「exe 小」与「target 大」并不矛盾。

## 4. 一句话类比

> `target/` 相当于「厨房 + 所有备菜 + 锅碗瓢盆」；`ico_generator.exe` 只是最后端上桌的那盘菜。菜很小，不代表厨房小。

## 5. 如何清理与减小占用

```powershell
# 1) 清理全部构建产物（安全，可再通过 cargo build 重新生成）
cargo clean

# 2) 只清理 debug 缓存，保留 release
cargo clean --profile dev

# 3) 查看各子目录体积，定位大文件
Get-ChildItem target -Recurse -File | Sort-Object Length -Descending |
    Select-Object -First 10 Name, @{n='MB';e={[math]::Round($_.Length/1MB,2)}}

# 4) 针对调试场景：在 .cargo/config.toml 里给 dev profile 开启轻量调试信息
#    [profile.dev]
#    debug = 1        # 或 "line-tables-only"，可大幅缩小 .pdb
```

其他建议：

- `target/` **不需要提交到 Git**，本项目已在 [.gitignore](../.gitignore) 忽略 `/target`；打包源码时排除它即可从 4.3 GB 降到几十 KB。
- 若磁盘紧张，优先删 `target/debug`（3.2 GB），它只是调试缓存。
- release 目录除 `ico_generator.exe` 外的 `deps/`、`build/` 也可安全删除，但下次构建会重新编译。

## 6. 小结

| 问题 | 答案 |
| --- | --- |
| 为什么 exe 只有 10 MB？ | release 做了 `strip + LTO + opt-level="z"`，去掉符号并优化死代码 |
| 为什么目录有几个 GB？ | `target/` 缓存了所有依赖的 `.rlib/.rmeta/.pdb/.d` 及多构建变体 |
| 哪个才是最该保留的？ | `target\release\ico_generator.exe`（10.5 MB），其余均可再生 |
| 怎么变小？ | `cargo clean` / 删除 `target/debug` / 调低 dev 的 `debug` 级别 |
