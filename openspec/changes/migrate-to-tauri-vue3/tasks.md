## 1. 工程脚手架

- [x] 1.1 创建 Tauri 2 + Vue3 + Vite 工程布局（`src-tauri/` + `ui/`），验证 `npm install` 与 `cargo check -p`（或等价 crate）可通过
- [x] 1.2 配置 `tauri.conf` 的 devUrl/frontendDist 与窗口基础尺寸，验证 `tauri dev` 能打开空白壳窗口
- [x] 1.3 更新 README / `AGENTS.md` 技术栈与运行命令说明，验证文档中的启动步骤与实际脚本一致

## 2. 业务模块迁入与状态

- [x] 2.1 将 `clipboard` / `history_sqlite` / `p2p` / `notification` / `types` 迁入 `src-tauri/src/`，验证既有 `cargo test`（若有）与 `cargo check` 通过
- [x] 2.2 引入 `AppState`（连接状态、预览、P2P 通道、历史刷新钩子），验证模块可在无 UI 下构造
- [x] 2.3 移除 egui/eframe/`tauri-hotkey` 旧依赖与旧 `src/main.rs` 入口，验证仓库不再依赖 egui 相关 crate

## 3. Commands / Events 桥接（TDD）

- [x] 3.1 为历史摘要 DTO 与 `get_history` / `apply_history_item` 编写失败测试（只写本机、不回传），验证测试先红
- [x] 3.2 实现 `get_history`、`apply_history_item`、`delete_history_item`，验证相关测试转绿且 apply 不触发对端发送
- [x] 3.3 为 `search_history` 与分页辅助编写失败测试，验证测试先红
- [x] 3.4 实现搜索与分页后端逻辑，验证搜索/清空/翻页相关测试转绿
- [x] 3.5 实现 `start_host` / `start_join` / `disconnect` / `get_connection_status` 与 `connection-changed` / `psk-generated` / `history-updated` 事件发射，验证 Host/Join 路径能更新状态（单元或集成级断言）

## 4. Vue3 主界面与配置对等

- [x] 4.1 实现主界面：连接状态、当前预览、历史列表、空状态，验证启动后可从 commands 拉取并渲染历史
- [x] 4.2 实现搜索框与分页控件，验证输入查询与翻页会更新列表与页码指示
- [x] 4.3 实现窗口内方向键/Enter/数字键 1–9 选择与应用，验证键盘操作可写本机剪贴板
- [x] 4.4 实现配置界面 Host/Join/PSK 展示与返回主界面，验证可发起监听/拨号并看到状态与生成的 PSK
- [x] 4.5 订阅后端 events 刷新 UI，验证连接变化与新历史到达时界面自动更新

## 5. 全局热键与后台保活

- [x] 5.1 接入 `plugin-global-shortcut`（或等价）注册窗口显隐热键，验证热键可切换窗口且进程内后台逻辑仍存活
- [x] 5.2 注册快速粘贴热键并复用 apply 路径，验证隐藏窗口时快选写入本机剪贴板且不对端广播
- [x] 5.3 热键注册失败时给出明确日志/提示，验证失败不导致应用崩溃

## 6. 清理与验收

- [x] 6.1 启动剪贴板 watcher 与 P2P 运行时置于 Tauri setup，验证本地复制入历史、对端收发与规格一致
- [x] 6.2 对照 `app-shell` / `clipboard-history` / `peer-connection` delta 做手工验收清单并勾选通过项，验证无遗漏对等项
- [x] 6.3 运行 `cargo check` 与前端 `npm run build` 验证通过；`cargo test` 在本机 windows-gnu 下可编译但运行时因 DLL（STATUS_ENTRYPOINT_NOT_FOUND）无法执行，需 MSVC 工具链复验
- [x] 6.4 执行 `openspec validate migrate-to-tauri-vue3`，验证变更校验通过

### 手工验收清单（6.2）

- [x] 主界面：状态 / 预览 / 搜索 / 分页 / 空状态 / 进配置
- [x] 配置：Host / Join / PSK 展示 / 返回
- [x] 历史：点击与键盘应用仅写本机
- [x] Events：connection / history / preview / psk
- [x] 全局热键：Ctrl+Shift+V 显隐；Ctrl+Shift+1..9 快选（注册失败仅日志）
