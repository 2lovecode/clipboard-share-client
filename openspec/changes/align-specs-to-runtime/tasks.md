## 1. 合并主规格（clipboard-history）

- [x] 1.1 将 delta `specs/clipboard-history/spec.md` 合并进 `openspec/specs/clipboard-history/spec.md`：ADDED 自动入队/远端写剪贴板；MODIFIED Remote items 与 History presentation；REMOVED Hotkey-initiated；验证主规格不再含 Hotkey-initiated，且保留 Click writes local only / 搜索 / 分页 / 键盘选择
- [x] 1.2 对照 `src-tauri/src/clipboard.rs` 与 `commands.rs` 的 `handle_clipboard_changed` / `P2PEvent::Received` 做场景勾选清单，验证每个新/改场景在运行时有对应行为（文档核对，不改代码）
  - Connected sync: `spawn_watcher` → `handle_clipboard_changed` → `history_sqlite::add` + `p2p_send`
  - Disconnected: 无 `p2p_send` 时仍 `add`，不假装送达
  - Oversized: `is_image_too_large` → 状态提示，不入队
  - Received: `P2PEvent::Received` → `add` + `cb::set`
  - Click apply: `apply_history_item` → `apply_local`，无 peer send

## 2. 合并主规格（peer-connection / clipboard-payload / app-shell）

- [x] 2.1 合并 `peer-connection` delta：ADDED Noise PSK；MODIFIED Manual / listening / Session setup；REMOVED mDNS 与 Shared passphrase；验证主规格 Purpose 仍准确或同步改为「手动 Host/Join + PSK」（若 Purpose 仍写 mDNS，直接改主规格 Purpose）
- [x] 2.2 合并 `clipboard-payload` delta：MODIFIED 文本/图片/Unsupported；REMOVED HTML；验证主规格无 HTML 必达，且 Payload integrity 仍在
- [x] 2.3 合并 `app-shell` delta：MODIFIED Configuration surface parity（Host 生成 PSK）；验证 Host/Join 场景与配置 UI 一致

## 3. OpenSpec 上下文与校验

- [x] 3.1 更新 `openspec/config.yaml` 的 `context` 技术栈为 Tauri 2 + Vue3（`ui/`）+ Vite、`src-tauri`（tokio、arboard、rusqlite、snow Noise P2P、tauri-plugin-global-shortcut），去掉 iced/WebSocket/cli-clipboard 过时描述；验证文件可读且仍含 Language: zh 与 AGENTS.md 路由提示
- [x] 3.2 运行 `openspec validate --specs` 与 `openspec validate align-specs-to-runtime`，验证全部通过
- [x] 3.3 确认未修改 `src-tauri/`、`ui/` 业务代码（`git status` / diff 仅含 openspec 与本变更产物），验证无意外代码改动

## 4. 收尾准备

- [x] 4.1 在变更说明或 tasks 备注中列出有意未实现项（mDNS、HTML、来源标签、托盘），验证与 proposal Out-of-Scope 一致，便于 archive 摘要引用
  - **有意未实现（Out-of-Scope）**：mDNS 发现、HTML 富文本载荷、历史 local/remote 来源标签、系统托盘、断线重连、显式推送热键模型、收到不覆盖本机剪贴板
