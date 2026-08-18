# X / Twitter Media Downloader (Tauri + Rust)

高性能 X (Twitter) 媒体批量下载工具，采用 Rust 异步并发与流式落盘技术，前端使用现代响应式暗黑毛玻璃界面。

## 目录结构
- `.github/workflows/release.yml` : GitHub Actions 自动化编译流
- `src-tauri/` : Rust 后端核心代码（GraphQL 解析、Tokio 并发下载引擎、智能去重）
- `ui/` : 前端界面（HTML5 / 现代 CSS 变量 / 原生 JS 通信）

## 云端构建与使用（GitHub Actions）
在您的 GitHub 上新建一个仓库（例如 x-media-downloader）。
将解压出来的所有文件直接推送到该仓库。
进入仓库的 Actions 页面，找到 Build and Release EXE 工作流，点击右侧的 Run workflow。
等待 2~3 分钟，构建完成后在仓库的 Releases 栏目直接下载生成的 Windows 独立可执行文件（.exe）与安装包（.msi）。


X / Twitter 媒体下载器（Tauri + Rust 重构版）
📦 项目概述
本项目已完整重构为 Tauri + Rust 后端 + 现代化毛玻璃前端 架构
完整目录结构：
├── .github/

│   └── workflows/

│       └── release.yml          # GitHub Actions 自动化编译工作流

├── package.json                 # 前端配置与 Tauri CLI

├── README.md                    # 项目快速指引

├── src-tauri/

│   ├── Cargo.toml               # Rust 依赖项清单（Tokio, Reqwest, Serde 等）

│   ├── tauri.conf.json          # Tauri 窗口与打包配置

│   └── src/

│       ├── main.rs              # Tauri Command 注册与应用入口

│       ├── types.rs             # 数据结构定义

│       ├── twitter_client.rs    # Twitter GraphQL 协议与解析

│       └── downloader.rs        # Tokio 高并发异步流式落盘引擎

└── ui/

    ├── index.html               # 现代深色仪表盘界面

    ├── styles.css               # 毛玻璃与响应式样式

    └── main.js                  # 前端交互与 Tauri IPC 通信

