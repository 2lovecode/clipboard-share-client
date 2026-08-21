## Context

See `proposal.md` - Why。当前仓库是单二进制 egui/eframe 应用：`main.rs` 持有 UI 状态，后台线程跑 tokio 桥接，模块含 `clipboard`、`history_sqlite`、`p2p`、`hotkey`（已禁用）、`notification`、`types`。权威业务行为见 `openspec/specs/{clipboard-history,clipboard-payload,peer-connection}`；本变更在其上增加 `app-shell` 并对历史/连接的 UI 交付面补齐对等要求。

约束：功能对等；Vue3 前端；不改 P2P/载荷语义；Windows/macOS/Linux 仍为目标平台。

## Goals / Non-Goals

**Goals:**
- 以 Tauri 2 为桌面壳，Vue3 + Vite 实现与现 egui 对等的主界面与配置界面。
- 将业务逻辑留在 Rust，经 commands/events 暴露给前端。
- 用 Tauri 全局快捷键替换失效的 `tauri-hotkey` 桩，恢复窗口切换与快速粘贴。
- 给出可验证的工程布局与迁移步骤，便于按 TDD 逐步落地。

**Non-Goals:**
- 不重写 P2P/加密协议或 SQLite schema（除非桥接层需要薄 DTO）。
- 不引入 Pinia/复杂状态库以外的重型前端架构（保持简单可维护）。
- 不做自动更新、托盘复杂菜单体系（本变更仅对等现有窗口与热键行为；托盘可作为后续变更）。

## Decisions

### Decision: Tauri 2 + Vue3 + Vite 标准布局
- **Choice**: 采用 `src-tauri/`（Rust）+ `ui/`（Vue3/Vite）标准拆分；根目录 README/`AGENTS.md` 指向 `tauri dev` / `tauri build`。
- **Rationale**: Tauri 2 是当前主线；Vue3 为用户指定；Vite 为官方模板默认。
- **Alternatives**: Tauri 1（过时）；单仓继续用 egui（违背目标）；前端用 React（用户已选定 Vue3）。

### Decision: 业务模块迁入 `src-tauri/src/`，去掉 egui 入口
- **Choice**: 将现有 `clipboard`、`history_sqlite`、`p2p`、`notification`、`types` 迁入 Tauri crate；删除 `eframe` 驱动的 `ClipboardShare` UI 结构体；用 `AppState` + commands 替代。
- **Rationale**: 减少双入口；业务已是 Rust，迁移成本最低。
- **Alternatives**: 保留独立 lib crate 再被 Tauri 依赖（过度分层，本阶段不需要）。

### Decision: Commands 负责动作，Events 负责推送
- **Choice**:
  - Commands（示例名，实现时可微调）：`get_history`、`search_history`、`apply_history_item`、`delete_history_item`、`start_host`、`start_join`、`disconnect`、`get_connection_status`、`get_current_preview`。
  - Events：`connection-changed`、`history-updated`、`psk-generated`、`clipboard-preview-updated`。
- **Rationale**: 与现 `BackendEvent` 模型同构，前端无轮询。
- **Alternatives**: 仅轮询 commands（更简单但延迟与浪费更高）。

### Decision: 前端用 Composition API + 轻量 store
- **Choice**: Vue3 `<script setup>`；用 `ref`/`reactive` 或极薄 composable 管 UI 状态；按需 `@tauri-apps/api`。
- **Rationale**: 对等 UI 规模不大，避免过早引入大型状态库。
- **Alternatives**: Pinia（可后续加）；Nuxt（桌面壳过重）。

### Decision: 全局热键用 Tauri plugin-global-shortcut
- **Choice**: 注册窗口 toggle 与数字快选（或与现设计等价的快捷键集）；回调内调用同一套 apply/clipboard 逻辑，且快选不触发对端广播。
- **Rationale**: 官方插件跨平台；替换已坏的 `tauri-hotkey`/core-graphics 路径。
- **Alternatives**: 自研 CGEventTap（已失败）；继续禁用热键（违背对等）。

### Decision: 点击历史写本机剪贴板的语义与现规格对齐
- **Choice**: `apply_history_item` 默认只写本机；若现 egui 在 Enter/数字键路径曾附带 `p2p_send`，迁移时以权威规格为准：**点击/选择不得仅因选择而回传对端**（见 `clipboard-history` Click writes local clipboard only）。全局热键快选同样不回传。
- **Rationale**: OpenSpec 为权威；修正 egui 与规格不一致之处属于对等迁移的正确行为，不单独立项改协议。
- **Alternatives**: 原样复制 egui 回传副作用（违背规格，拒绝）。

### Decision: edition / toolchain
- **Choice**: Tauri 2 模板所需的 edition 与 MSRV 跟随官方模板（可升到 2021）；业务代码尽量少改语法。
- **Rationale**: 模板兼容性优先于保留 edition 2018。
- **Alternatives**: 强行锁 2018（可能与 Tauri 2 冲突）。

## Risks / Trade-offs

- [双工具链复杂度] → 文档写清 Node + Rust；CI/`cargo check` 与 `npm`/Vite 检查路径在 tasks 中分开验证。
- [热键与系统权限（macOS 辅助功能等）] → 失败时 UI/日志明确提示；降级为仅窗口内快捷键。
- [egui 与规格不一致的回传行为] → 以规格为准，在 design/tasks 中写清，避免“像素级副作用对等”。
- [大图/事件频繁导致前端卡顿] → 历史列表传摘要 DTO；图片用缩略/标记，完整字节仅在 apply 时由后端写剪贴板。
- [Windows WebView2 依赖] → 打包说明与安装前提写入 README。

## Migration Plan

1. 用官方 `create-tauri-app`（Vue3 + TypeScript）脚手架到仓库约定目录，或手工对齐等价结构。
2. 迁入业务模块并实现最小 commands（history + status），前端先打通只读列表。
3. 补齐配置 Host/Join、事件推送、搜索分页、键盘交互。
4. 接入全局热键；移除 egui 依赖与旧 `main` 入口。
5. `cargo test` / 前端单测或手工验收清单对照 specs；更新 README/`AGENTS.md`。
6. 回滚策略：在 archive 前保留 git 分支；不删除历史 commit。若需紧急回退，检出迁移前标签/分支即可恢复 egui 构建。

## Open Questions

- 全局热键具体默认键位（Windows vs macOS）可在实现时按平台惯例选定并写入 README，不改变规格场景。
- 是否在本变更加入系统托盘图标：默认不做；若实现中发现“关窗即退出”与后台保活冲突，再用最小托盘隐藏窗口，不扩展菜单范围。
