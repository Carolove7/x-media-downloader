# X 媒体下载器（Tauri / Rust 版）

基于 **Tauri 2 + Rust + Vue 3** 重写的 X（Twitter）媒体批量下载器，是原 Python（customtkinter）版的 1:1 移植——功能与行为一致，但体积更小（约 6 MB）、启动更快、不再依赖 Python 运行时。

## 简介

- 使用 X 网页 Cookie（`auth_token` + `ct0`）通过 GraphQL 接口鉴权，批量拉取指定账户的图片/视频时间线并下载。
- 按媒体唯一 ID 去重，重复运行不会重复下载；支持时间范围、类型过滤、并发下载（1–32 线程，默认 16）、断点安全（`.part` + 原子改名）。
- 实时反馈进度、速度、统计；配置持久化到 exe 同目录 `config.json`；支持打开目录、历史账户记忆、CLI 自检（`--selftest`）。
- 新版本通过 GitHub Actions 自动构建，发布为 `XMD_x.x.x_x64-setup.exe` 安装包。

## 快速使用

1. 浏览器登录 X，从 Cookie 中复制 `auth_token` 与 `ct0`。
2. 打开程序，填入 Cookie、账户 ID（不含 `@）、保存位置。
3. 选择类型 / 线程数 / 时间范围，点击「开始下载」。

## 免责声明

仅供学习与个人合法用途，用于下载你本人拥有权利的媒体内容；请合理控制请求频率。
