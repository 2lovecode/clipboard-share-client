## Context

See `proposal.md` - Why。权威主规格在 Tauri 迁移后仍残留 iced 时代契约；运行时行为以 `src-tauri`（clipboard watcher、Noise P2P、Host 生成 PSK、Text/Image）与 README 为准。本变更为规格与 OpenSpec 上下文回写，不改应用代码。

## Goals / Non-Goals

**Goals:**
- 用 delta 精确改写四个 capability，使合并后的主规格可被场景测试对照当前实现。
- 同步刷新 `openspec/config.yaml` 的 stack `context`，避免 agent 继续引用 iced/WebSocket。
- 在 design/tasks 中记录「有意保留的缺口」（mDNS、HTML、来源标签）以免被当作回归。

**Non-Goals:**
- 不引入新运行时行为、依赖或 UI。
- 不做视觉重设计或 README 大改（仅当规格术语与 README 明显冲突时做最小措辞核对，可在 tasks 勾选）。

## Decisions

### Decision: 以运行时为权威，规格迁就产品
- **Choice**: 自动同步、收到即写本机剪贴板、Noise PSK、无 mDNS、无 HTML —— 全部写入 SHALL。
- **Rationale**: 用户选择「规格改成跟现状一致」；产品立场已在代码与 README 落地。
- **Alternatives**: 改代码迁就旧规格（显式推送）—— 属另一产品变更，Out-of-Scope。

### Decision: 来源标签降级为非必达
- **Choice**: 修改 History presentation / Remote items，去掉 local/remote 标签 MUST。
- **Rationale**: `HistorySummary` 无 source 字段；强留会制造永久红灯。
- **Alternatives**: 本变更顺带加字段（扩 scope，拒绝）。

### Decision: HTML 移除而非「MAY」
- **Choice**: REMOVED Rich text / HTML；Unsupported types 覆盖 HTML-only。
- **Rationale**: `ClipItem` 无 Html 变体；MAY 会造成「半支持」幻觉。
- **Alternatives**: 保留 HTML 为未来 ADDED（可另开 change）。

### Decision: config.yaml 与 specs 同变更落地
- **Choice**: tasks 中直接编辑 `openspec/config.yaml` context。
- **Rationale**: 该文件是 agent 约束源；与规格漂移同源。
- **Alternatives**: 另开 tooling change（不必要碎片化）。

## Risks / Trade-offs

- [历史读者误以为产品「降级」] → proposal/Out-of-Scope 写明是规格回写；archive 摘要说明。
- [误删仍成立的场景] → MODIFIED 时保留 Click writes local only、分页/搜索、热键快选等未漂移要求。
- [apply 时误改代码] → tasks 仅允许规格与 config.yaml；明确禁止 `src-tauri/` / `ui/`。

## Migration Plan

1. 审阅并合并四个 delta 到 `openspec/specs/*/spec.md`。
2. 更新 `openspec/config.yaml` context。
3. `openspec validate --specs` 与 `openspec validate align-specs-to-runtime`。
4. Archive 本变更；后续功能变更基于新权威规格。
5. 回滚：`git checkout` 恢复 specs 与 config.yaml。

## Open Questions

- （无）产品立场已由用户选项「规格对齐现状」确认。
