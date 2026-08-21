## ClipboardShareClient

跨 macOS / Windows / Linux 的剪贴板共享客户端。

## 技术栈

- **桌面壳**：Tauri 2
- **前端**：Vue 3 + Vite + TypeScript
- **后端**：Rust（剪贴板、SQLite 历史、Noise P2P）

## 功能

1. 两台客户端之间 P2P 加密同步剪贴板（文字 / 图片）
2. 接管系统剪贴板，本地保留历史，支持搜索
3. 快捷键：窗口内方向键 / Enter / 数字键；全局 `Ctrl+Shift+V` 显隐窗口，`Ctrl+Shift+1..9` 快速粘贴

## 开发

前置：Rust、Node.js、（Windows 需 WebView2）

```bash
npm install
npm run tauri dev
```

## 构建

```bash
npm run tauri build
```

## 检查与测试

```bash
cd src-tauri && cargo check
npm run build
```

Windows 建议安装 [VS Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)（含 C++ 工具集），然后：

```bash
rustup default stable-x86_64-pc-windows-msvc
```

若暂时只能用 `windows-gnu`，`src-tauri/build.rs` 已加 `--exclude-all-symbols` 规避 `export ordinal too large`；测试二进制仍可能因 DLL 问题无法运行。