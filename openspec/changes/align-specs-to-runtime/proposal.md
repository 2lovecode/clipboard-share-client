## Why

主规格仍描述 iced 时代的「快捷键主动推送 / mDNS / WebSocket 明文口令 / HTML 载荷」，而 Tauri 运行时已是「剪贴板自动同步 + Noise/TCP + Host 生成 PSK + 文本/图片」。规格与实现分叉会导致后续变更对着错误契约设计。现在把权威规格回写为当前可观察行为，并刷新过时的 `openspec/config.yaml` 技术栈描述。

## What Changes

- **BREAKING（规格语义）**：将 `clipboard-history` 从「仅快捷键入队」改为「系统剪贴板变化自动入队；已连接时同步推送对端」。
- 明确对端条目到达时：写入本机历史，并写回本机系统剪贴板（与当前实现一致）。
- 保留「用户从历史 apply / 全局快选仅写本机、不因该动作回传」契约。
- **BREAKING（规格语义）**：`peer-connection` 去掉 mDNS 必达要求；会话模型改为 Host 监听 / Join 拨号 + Noise PSK 握手（加密通道）。
- **BREAKING（规格语义）**：`clipboard-payload` 去掉 HTML 富文本必达要求，权威载荷集合收敛为纯文本与图片。
- 调整 `app-shell` 配置面表述：Host 侧展示系统生成的 PSK，而非用户自拟共享口令。
- 更新 `openspec/config.yaml` 中的技术栈上下文，与 Tauri 2 + Vue3 + Noise 现状一致。
- 历史列表暂不要求 local/remote 来源标签（实现尚未提供该字段）；排序与摘要仍保留。

## Capabilities

### New Capabilities

- （无）

### Modified Capabilities

- `clipboard-history`: 入队/推送改为剪贴板监听驱动；远端到达写本机剪贴板；去掉未实现的来源标签必达。
- `peer-connection`: 移除 mDNS 必达；握手改为 Noise PSK；状态与配置面与 Host/Join 对齐。
- `clipboard-payload`: 移除 HTML 必达；保留文本/图片与完整性约束。
- `app-shell`: 配置面 Host 生成并展示 PSK 的表述与实现对齐。

## Impact

- **规格**：`openspec/specs/{clipboard-history,peer-connection,clipboard-payload,app-shell}/spec.md` 经 delta 合并后成为新权威。
- **工具上下文**：`openspec/config.yaml` 的 `context` 栈描述更新，避免 agent 继续按 iced/WebSocket 假设工作。
- **代码**：本变更**不改**业务行为；仅文档/规格对齐。若日后要回到「显式推送 / 收到不覆盖」，应另开产品变更。

## Out-of-Scope / Non-goals

- 不实现 mDNS、HTML 载荷、历史来源标签、系统托盘、断线重连。
- 不把产品改回「快捷键推送 / 收到不覆盖剪贴板」。
- 不改 P2P 帧格式、Noise 模式或 SQLite schema。
- 不引入 CI、自动更新或多人拓扑。
