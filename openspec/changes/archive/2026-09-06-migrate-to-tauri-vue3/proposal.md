## Why

当前桌面 UI 基于 egui/eframe，与现代前端生态割裂，难做布局迭代与跨端一致体验。迁移到 Tauri + Vue3 可保留现有 Rust 后端能力，同时用 Web 技术实现功能对等的桌面壳与界面。

## What Changes

- **BREAKING**：移除 egui/eframe UI，应用入口改为 Tauri 桌面壳 + Vue3 前端。
- 将现有界面能力以功能对等方式迁到 Vue3：历史列表、搜索、分页、当前内容预览、连接状态、配置（Host/Join/PSK）、窗口显隐、列表内键盘导航与数字键快选。
- Rust 侧保留并接入剪贴板监听、SQLite 历史、P2P、通知等模块；通过 Tauri commands / events 与前端通信。
- 全局热键改为基于 Tauri 能力（或等价插件）实现窗口切换与快速粘贴，替换当前已禁用的 `tauri-hotkey` 桩。
- 更新构建/运行方式与文档（`cargo`/`tauri`/`npm` 工作流），`AGENTS.md` / README 技术栈描述同步为 Tauri + Vue3。

## Capabilities

### New Capabilities
- `app-shell`: 定义 Tauri + Vue3 桌面壳、前后端桥接、窗口显隐，以及与现有业务能力对等的主界面/配置界面契约。

### Modified Capabilities
- `clipboard-history`: 补齐现有 egui 已具备但对等迁移必须保留的展示与交互要求（搜索、分页、键盘导航/数字键快选）。
- `peer-connection`: 明确配置界面中的 Host/Join/PSK 操作与状态展示须通过新壳完成（行为不变，交付面从 egui 改为 Vue）。

## Impact

- 代码：`src/main.rs` 及 egui 相关 UI 逻辑删除或下沉为 commands；新增 `src-tauri/`（或等价）与 Vue3 `frontend/`（或 `ui/`）工程。
- 依赖：移除 `egui`/`eframe`/`egui_extras`；引入 Tauri 2、Vue3、Vite；热键依赖切换。
- 构建：需 Node.js + Rust 工具链；发布产物变为 Tauri 打包应用。
- 规格：现有 `clipboard-payload` 行为不变，本变更不改载荷协议。

## Out-of-Scope / Non-goals

- 不改变 P2P 协议、加密握手语义或载荷类型集合（纯文本/图片/HTML）。
- 不做全新视觉重设计或引入与现网无关的营销式 UI。
- 不新增账号系统、云同步、跨公网中继。
- 不在本变更中引入多人对等拓扑（仍为双端会话模型）。
