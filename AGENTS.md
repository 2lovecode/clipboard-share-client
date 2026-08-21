# AGENTS.md

本文件是本仓库对所有 coding agent 的**项目级指令**（通用约定，见 https://agents.md/）。
请优先遵循本文件与 `openspec/`；`.cursor/`、`.agents/` 为本地生成物，不入库。

## 项目概览

- **clipboard-share-client**：Rust 跨平台剪贴板共享客户端
- **技术栈**：Rust (edition 2018)、iced GUI、tokio、async-tungstenite、warp、cli-clipboard、tauri-hotkey；Windows 侧有 `deps/fetch_selected_text.exe`
- **规格目录**：`openspec/`（唯一权威规格与变更历史）
- **配置**：`openspec/config.yaml`（产物语言：中文；结构标题与 SHALL/MUST 保持英文）

## 构建与验证

- 检查：`cargo check`
- 测试：`cargo test`
- 运行：`cargo run`

## OpenSpec + Superpowers 路由

本仓库同时使用 **OpenSpec**（规格与变更生命周期）和 **Superpowers**（执行纪律 skills）。两者不自动串联，必须按本规则路由，否则会出现双份规格或无测试实现。

### 职责边界

| 层 | 工具 | 拥有什么 |
|---|---|---|
| WHAT / WHY | OpenSpec | `proposal.md`、delta `specs/`、`design.md`、`tasks.md`、archive 历史 |
| HOW WELL | Superpowers | TDD、系统化调试、code review、验证、并行 subagent |

**禁止**：同一功能既跑 Superpowers `brainstorming` / `writing-plans`，又跑 OpenSpec propose，导致两套权威文档互相漂移。

### 何时走哪条路

- **多文件 / 会迭代 / 需要留「为什么」** → OpenSpec 全流程；apply 时挂 Superpowers 执行纪律
- **需求仍模糊** → 先 OpenSpec explore（可内嵌 brainstorming 对话），想清楚后再 propose
- **单点 bug / typo / 小改动** → **不走 OpenSpec**；bug 用 `systematic-debugging` 后直接修
- **一次性实验脚本** → Superpowers 即可，不必开 change

### 标准变更循环

命令名因工具而异（Cursor：`/opsx-propose`；通用 skill：`/openspec-propose` 等）。语义统一为：

```
explore（可选）→ propose → 人工审 Out-of-Scope → apply（强制 TDD）→ archive
```

CLI：`openspec list` / `openspec show <change>` / `openspec validate <change>`

工作流 skills（本地生成后可见）：`openspec-explore`、`openspec-propose`、`openspec-apply-change`、`openspec-update-change`、`openspec-sync-specs`、`openspec-archive-change`

#### propose 阶段

- 只写规划产物，**不要改业务代码**
- 审 `proposal.md` 的 Out-of-Scope；不要静默扩 scope
- **不要**再触发 `brainstorming` 或 `writing-plans`

#### apply 阶段（必须挂 Superpowers）

对 `tasks.md` 逐项执行时：

1. 先调用 `test-driven-development`：先失败测试，再最小实现；先写实现再补测试的应删除重来
2. 一批任务完成后调用 `verification-before-completion`
3. 需要审查时调用 `requesting-code-review`
4. 勾选任务立即改 `- [x]`，不要攒到最后批量勾
5. 优先用 `cargo test` / `cargo check` 验证

#### archive 阶段

- 完成后**必须** archive；未 archive 会让后续会话误以为工作未做完而重做
- 可在 archive 前单独 sync delta specs

### 红线

- 权威规格只在 `openspec/`；不要把规格另写到 `docs/superpowers/` 等目录
- 不要跳过 archive
- 不要假设 apply 自带 TDD——必须按本规则显式调用 Superpowers skills

## 本地 IDE / Skills 适配（不入库）

`.cursor/`、`.agents/` 已 gitignore。克隆仓库后在本机生成（勿提交）：

```bash
# 通用 Agent Skills → .agents/skills/
openspec init --tools agents --language zh

# 若使用 Cursor，额外生成 slash commands → .cursor/
openspec init --tools cursor --language zh

# 之后刷新生成物
openspec update
```
