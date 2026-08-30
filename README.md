# X 媒体下载器（Tauri / Rust 版）

基于 **Tauri 2 + Rust + Vue 3** 重写的 X（Twitter）媒体批量下载器，是原 Python（customtkinter）版 `twitter_downloader_gui.py` 的 **1:1 移植**，功能与行为保持一致，但体积更小（约 6 MB）、启动更快、不再依赖 Python 运行时。

> 下载的媒体按「媒体唯一 ID」去重，重复运行不会重复下载；支持时间范围过滤、类型过滤、并发下载、断点安全（`.part` + 原子改名）。

---

## 功能特性

- **Cookie 认证**：使用 X（Twitter）网页 Cookie 中的 `auth_token` 与 `ct0` 进行 GraphQL 接口鉴权。
- **媒体时间线分页**：`UserByScreenName` 获取用户 → `UserMedia` 拉取全部媒体时间线（最多 500 页）。
- **唯一 ID 去重**：图片/视频按媒体 ID 去重，已下载文件自动跳过，可安全重复运行。
- **时间范围过滤**：可选起始/结束日期（年/月/日下拉），格式 `YYYY-MM-DD:YYYY-MM-DD`；越界自动收敛到 `[1990-01-01, 当天]`，起止倒序自动交换。
- **类型过滤**：全部媒体 / 仅图片 / 仅视频。
- **并发下载**：1–16 线程（信号量控制），默认 8。
- **安全下载**：先写 `*.part` 临时文件，完成后原子改名；用户在下载中取消时静默中断当前任务。
- **实时反馈**：进度、速度（0.4s 汇总一次）、统计（已下载 / 已跳过 / 失败）通过 Tauri 事件推送到前端。
- **历史账户**：最近使用的账户 ID 自动记忆（最多 7 个），支持自定义下拉框与键盘上下选择。
- **打开目录**：一键打开 `save_path/user_id` 媒体目录（详见「已知问题」）。
- **配置持久化**：配置写入 exe 同目录的 `config.json`。
- **CLI 自检模式**：`--selftest` 不开窗口，跑真实链路并把结果写入 `selftest_result.txt`。

---

## 技术栈

| 层 | 技术 |
|---|---|
| 桌面壳 | Tauri 2（`tauri-build` / `tauri` / `tauri-utils`） |
| 后端 | Rust 2021（tokio 异步、`reqwest` 流式下载、`regex`、`md-5`、`chrono`、`rfd` 文件对话框） |
| 前端 | Vue 3 + Vite 6（原生 `<script setup>`，无 UI 框架） |
| 打包 | NSIS 安装包（`bundle.targets: ["nsis"]`），图标 `icons/icon.ico` |

窗口：标题「X 媒体下载器」，`820×940`，最小 `780×800`，可缩放、居中。

---

## 目录结构

```
X媒体下载器-Tauri/
├── src/                      # 前端（Vue 3）
│   ├── App.vue               # 主界面 + 全部交互逻辑
│   ├── main.js               # Vue 挂载入口
│   └── style.css             # 全局样式与主题 CSS 变量
├── src-tauri/                # Rust 后端（Tauri）
│   ├── Cargo.toml
│   ├── build.rs
│   ├── tauri.conf.json       # 应用配置（窗口/权限/打包）
│   ├── capabilities/default.json
│   └── src/main.rs           # 全部后端逻辑（下载器 + Tauri 命令）
├── dist/                     # 前端构建产物（vite build 生成，由 Tauri 嵌入）
├── release/                  # 便携分发目录（exe + WebView2Loader.dll + config.json）
├── X媒体下载器-便携版.zip     # 便携打包
├── build.bat                 # 一键构建脚本（Windows）
├── package.json
└── vite.config.js
```

---

## 配置说明（config.json）

`config.json` 位于 **exe 同目录**，字段与 Python 版完全一致：

| 字段 | 类型 | 说明 |
|---|---|---|
| `auth_token` | string | X 网页 Cookie 中的 `auth_token` |
| `ct0` | string | X 网页 Cookie 中的 `ct0`（CSRF token） |
| `user_id` | string | 目标账户 handle（不含 `@`） |
| `save_path` | string | 保存根目录，下载到 `save_path/user_id/` |
| `concurrency` | u32 | 并发线程数，默认 8，范围 1–16 |
| `media_filter_label` | string | 界面类型筛选标签（`全部媒体`/`仅图片`/`仅视频`） |
| `time_range` | string | 时间范围 `YYYY-MM-DD:YYYY-MM-DD`，空为不限 |
| `recent_user_ids` | string[] | 历史账户 ID（最多 7 个） |

媒体文件命名：`{时间}-{img|vid}_{媒体ID}.{ext}`，例如 `2026-08-30 12-00-img_AbC123.jpg`。

---

## 使用方法

### 1. 获取 Cookie

1. 浏览器登录 X（Twitter），打开开发者工具 → 应用/存储 → Cookie（`twitter.com` 或 `x.com`）。
2. 复制 `auth_token` 与 `ct0` 的值，填入主界面对应输入框。
3. 填写「账户 ID」（如 `ekin9527`，不含 `@`）与「保存位置」。

### 2. 界面操作

- **开始下载** / **回车**：启动任务（任务进行中按钮禁用）。
- **取消下载** / **Esc**：中断当前队列（已完成的文件保留）。
- **类型**：切换全部 / 仅图片 / 仅视频。
- **线程数**：1–16 并发。
- **时间范围**：勾选「不限时间」或选择起止日期。
- **打开目录**：打开 `save_path/user_id` 媒体目录（目录尚不存在时回退打开保存位置）。
- **运行日志**：实时输出，可「复制」/「清空」；按标签着色（`[成功]` 绿 / `[跳过]` 黄 / `[失败]` 红）。

### 3. 便携分发

`release/` 目录需包含三个文件才能独立运行：

- `x-media-downloader.exe`
- `WebView2Loader.dll`（GNU 工具链构建必需；改用 MSVC 工具链则不需要）
- `config.json`（首次运行可留空，程序会在界面填写后自动写入）

---

## 构建与打包

### 前置依赖

- **Node.js**（含 `npm`）
- **Rust 工具链**（rustup，使用 **GNU** 目标 `x86_64-pc-windows-gnu` + 本机 `mingw64`）
- `WebView2` 运行时（Windows 10/11 一般已内置）

### 一键构建（推荐）

直接运行项目根目录的 `build.bat`：

```bat
build.bat
```

它会：检查工具链 → `npm install` + `npm run build` → `cargo build --release` → 把 exe 复制到 `release/` 并补齐 `WebView2Loader.dll` / `config.json`。

### 手动构建

```bat
set "PATH=C:\Users\19641\mingw64\bin;%PATH%"
set "CARGO_TARGET_DIR=C:\Users\19641\.rust-target"
npm install
npm run build
cd src-tauri
cargo build --release
```

### ⚠️ 构建环境注意事项（本机坑点）

本机使用 **MinGW（GNU）工具链**，构建时有两处易踩的坑：

1. **`dlltool.exe` 必须在 PATH**：MinGW 的 `dlltool`/`as`/`ld` 用于编译 `windows-sys` 等 crate。系统默认 PATH 里的 `/mingw64/bin` 指向错误位置，需改为本机真实路径 `C:\Users\19641\mingw64\bin`，否则报 `error calling dlltool 'dlltool.exe': program not found`。
2. **编译目标目录必须是纯 ASCII 路径**：`dlltool`/`ld` 不支持中文路径，项目路径含中文（`X媒体下载器-Tauri`）时直接 `cargo build` 会报 `ld: cannot find ... .o: Invalid argument`。解决：把 `CARGO_TARGET_DIR` 指向纯 ASCII 目录（如 `C:\Users\19641\.rust-target`），`build.bat` 已内置此设置。

> 若改用 **MSVC** 工具链（`x86_64-pc-windows-msvc` + Visual Studio 生成工具），则无需 `WebView2Loader.dll`，且不受中文路径限制。

---

## CLI 自检模式

无需打开窗口即可验证整条链路（Cookie 有效性、接口可达、单文件与并发下载、去重回环）：

```bat
x-media-downloader.exe --selftest
```

结果（含 `SELFTEST_PASSED` / `SELFTEST_FAILED` 与逐项明细）写入 **exe 同目录** 的 `selftest_result.txt`，便于无控制台环境核对。

---

## 架构与实现要点

### 后端（Rust，`src-tauri/src/main.rs`）

**Tauri 命令**

| 命令 | 说明 |
|---|---|
| `get_config` / `save_config` | 读取 / 写入 `config.json` |
| `start_download` | 启动异步下载任务（带 `running` 互斥与取消标记） |
| `cancel_download` | 置位取消标记，实时中止下载器 |
| `browse_folder` | 系统文件夹选择对话框（`rfd`） |
| `open_folder` | 打开目录（`explorer`，已修复跳转问题） |
| `show_message` | 原生消息弹窗（`rfd`） |

**推送到前端的事件**：`xdl-log`（日志）、`xdl-stats`（统计）、`xdl-progress`（进度）、`xdl-speed`（速度）、`xdl-done`（完成/取消）。

**核心逻辑**

- `Downloader`：对应 Python 版 `DownloaderWorker`，两阶段——先全量遍历时间线构建去重后的下载队列，再并发下载。
- 去重：扫描本地已有文件提取媒体 ID，已在则跳过；新文件写 `.part` 后原子改名。
- 速度监控：独立 tokio 任务每 0.4s 汇总一次总吞吐。
- AppUserModelID：通过 `shell32` FFI 固定任务栏图标，避免显示为宿主进程图标。

### 前端（Vue 3，`src/App.vue`）

- 监听上述 Tauri 事件刷新进度条、统计卡、日志区。
- 时间范围选择器、历史账户下拉框、类型/线程切换均为纯前端逻辑，与 Python 版行为一致。
- 配置在「开始下载」时通过 `save_config` 持久化。

---

## 已知问题与修复

### 「打开目录」跳转到「文档」目录

**现象**：点击「打开目录」会打开 `C:\Users\19641\Documents` 而非目标媒体目录。

**根因**：Windows `explorer <path>` 在路径以分隔符结尾且被加引号（路径含空格时 Rust 自动加引号）时，结尾的反斜杠会把闭合引号「吃掉」，explorer 收到无效路径后回退打开「文档」目录。

**修复**（`main.rs` 的 `open_folder` + `App.vue` 的 `openFolder`）：

- Rust 端对路径做规整：统一为反斜杠、去掉末尾分隔符；目录不存在时回退打开上级保存位置并提示「请先执行下载」。
- 前端端去掉 `save_path` 末尾多余分隔符；`user_id` 为空时直接打开 `save_path`。

---

## 免责声明

本项目仅供学习与个人合法用途，用于下载**你本人拥有权利**的媒体内容。使用者需自行承担因违反 X/Twitter 服务条款或当地法律法规而产生的风险。请合理控制请求频率，避免对目标服务造成不当压力。
