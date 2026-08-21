## Context

现有客户端为 iced 单文件骨架：Alt+C 在 Windows 上取选中文本、`messages` 列表占位、剪贴板写入为硬编码字符串。`Cargo.toml` 已声明 `warp`、`async-tungstenite`、`tokio`、`cli-clipboard`、`tauri-hotkey`，但未形成对等会话。动机与范围见 `proposal.md`；行为契约见 `specs/`。

## Goals / Non-Goals

**Goals:**
- 在现有 iced + tokio 栈上落地：mDNS 发现、手动 IP、对等 WebSocket、口令握手、历史列表与多媒体载荷
- 模块边界清晰，便于单测协议编解码与握手，而不必启动完整 GUI
- 双拨号时有确定的单会话裁决规则

**Non-Goals:**
- 引入独立后端服务或中继
- 本阶段升级 iced 大版本或重写整个 GUI 框架
- 实现 TLS 或密码学级保密传输

## Decisions

### 1. 传输：双方 warp 监听 + async-tungstenite 拨号
- **选择**：每端启动本地 WebSocket 服务（warp）；发现或手填后由拨号端用 async-tungstenite 连接。
- **理由**：与现有依赖一致；对等模型下「人人可被连」。
- **备选**：纯客户端连中心房间（Out-of-Scope）；仅一方主机（已否决）。

### 2. 双连接裁决
- **选择**：会话建立后交换稳定 `peer_id`（启动时随机 UUID）。若检测到与同一 `peer_id` 存在两条活连接，保留「本端 `peer_id` 字典序较小者作为拨号方」的那条，关闭另一条。
- **理由**：无需时钟同步，行为确定。
- **备选**：一律由用户指定主/从（更简单但违背「对等」体验）。

### 3. 发现：mDNS（DNS-SD）
- **选择**：注册服务类型如 `_clipboard-share._tcp.local.`，TXT 含显示名与（可选）协议版本；浏览结果驱动 UI 列表。
- **理由**：用户明确要求 mDNS；跨平台有成熟 crate（实现阶段选定并锁定版本）。
- **备选**：UDP 广播（已否决为默认发现）。

### 4. 口令握手
- **选择**：WebSocket 连通后首条控制消息携带口令的常量时间可比较摘要（或双方预共享口令明文比对的最小实现）；失败则关闭。口令存在本地配置（进程内 / 本地文件），不入库到公开日志。
- **理由**：满足「简单口令防误连」；不伪装成 TLS。
- **备选**：全链路 TLS + PSK（过重，Out-of-Scope）。

### 5. 应用消息格式
- **选择**：JSON 文本帧（控制与元数据）+ 大图可用 binary 帧或 base64 字段；统一 envelope：`{ "v", "kind", "id", "ts", "payload" }`，`kind` ∈ `auth` | `auth_ok` | `auth_fail` | `history_item` | `goodbye`。
- **payload**：`text` | `html` | `image`（mime + bytes）。
- **理由**：易测、易扩展；与 iced 消息驱动更新契合。
- **备选**：全 protobuf（收益低）。

### 6. 历史与 GUI
- **选择**：内存中 `Vec<HistoryItem>`（含来源、类型、摘要、完整载荷句柄）；上限（如 100 条）溢出丢最旧。点击 → 仅调用本机剪贴板写 API。推送热键保持 Alt+C（Windows 选中文本路径可复用 `fetch_selected_text.exe`）；跨平台优先读当前剪贴板多格式。
- **理由**：对齐既有 UI 雏形与产品决策「点击不回传」。
- **备选**：持久化 SQLite（非本阶段目标）。

### 7. 剪贴板多媒体
- **选择**：抽象 `ClipboardPort`（读：text/html/image；写：同上）。`cli-clipboard` 不足的平台用条件编译补平台 API；HTML 与 PNG 优先。
- **理由**：规格要求图片 + HTML，且需可测的失败路径。
- **备选**：第一版只做文本（与用户决定冲突）。

### 8. 模块划分（建议）
```
src/
  main.rs          # iced Application 入口
  ui/              # 视图与 Message
  net/             # warp 服务、拨号、会话、双连接裁决
  discovery/       # mDNS 注册与浏览
  history/         # 入队规则与列表模型
  clipboard/       # ClipboardPort 实现
  protocol/        # envelope 编解码（单测重点）
```

## Risks / Trade-offs

- [mDNS 在部分网络/防火墙不可用] → 手动 IP 为同等一等公民；UI 明确发现失败原因
- [双拨号竞态] → 明确 peer_id 裁决；集成测试覆盖「几乎同时连接」
- [大图阻塞 UI / 撑爆内存] → 载荷大小上限 + 列表只存缩略/摘要、原图按需持有；超限拒绝推送并提示
- [cli-clipboard 对 HTML/图片支持不齐] → `ClipboardPort` 隔离；平台差异在规格中已要求明确失败提示
- [口令仅防误连，流量可被嗅探] → 文档与 UI 不宣称端到端加密；后续可另开 change 加 TLS
- [对等 + 多媒体范围偏大] → tasks 按协议 → 连接 → 历史文本 → 多媒体分层，便于分批验证

## Migration Plan

- 无旧协议兼容包袱；开发分支直接演进骨架代码
- 配置项（口令、端口、显示名）使用合理默认；首次启动可提示设置口令
- 回滚：不发布则无运行时迁移；若已发预览版，仅丢弃不兼容会话（无持久历史）

## Open Questions

- 默认监听端口与服务显示名的最终文案（实现时可定，不影响规格）
- 图片大小上限的具体数值（实现时选保守默认，如数 MB 级，可配置）
