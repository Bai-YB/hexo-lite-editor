# 启动与项目打开模块

## 职责

`AppShell.svelte` 负责尽快建立可交互外壳，再启动可以延后的服务。`project.rs` 负责在原生侧验证 Hexo 根目录、读取同步状态、扫描文章与图片资源、创建带 generation 的项目会话，并调度打开后的后台同步。

## 启动调用链

1. Svelte 挂载 `AppShell`，注册键盘入口并调用 `platform.loadConfig()`。
2. 配置返回后立即应用语言与主题，设置 `configLoaded=true`，首屏和导航可以渲染。
3. 外壳注册任务、预览、同步、重扫和关闭事件。第一帧提交后，分别启动更新状态初始化与最近项目恢复；其余页面模块只在浏览器空闲时逐个预热，不再与首屏并发加载。
4. `initializeUpdateState()` 订阅更新快照、读取当前状态，并按每日频率安排后台检查。
5. `initializeRecentWorkspace()` 同时读取最近项目列表；启用自动恢复时调用 `reopenRecentProject()`，结果仍属于当前空会话才交给 `acceptProject()`。
6. `acceptProject()` 更新项目 session 与文章列表，编辑器随后加载第一篇文章；同步与重扫继续通过事件回到外壳。

## 原生调用链

`pick_project`、`reopen_recent_project` 与 `open_recent_project` 是异步 Tauri 命令。文件选择完成后，项目验证和扫描通过 `tauri::async_runtime::spawn_blocking` 执行，避免同步磁盘读取占用 WebView/UI 线程。

原应用标识的数据迁移只在当前配置尚未建立时递归执行；迁移成功写入完成标记。已有 `config-v3.json` 的安装直接跳过旧缓存遍历，避免每次原生窗口创建前重复扫描历史目录。

后台工作依次调用：

`validate_hexo_root` → `sync_before_open` → 项目文件锁 → `scan_articles` → 编辑器图片缓存恢复 → `ProjectSession` → 最近项目记录 → `AppState.project` → `schedule_sync_after_open`。

## 状态与约束

- 项目身份由 `projectId + generation` 共同校验，旧异步结果不能写入新会话。
- 自动恢复不会覆盖启动期间由用户主动打开的项目。
- 更新读取、自动检查、最近项目扫描失败只产生独立通知或日志，不重新阻塞首屏。
- 页面模块保持动态导入；首屏可交互后由 `requestIdleCallback` 逐个预热，既避免启动峰值，也避免首次导航长时间停留在加载页。

## 回归入口

- `tests/e2e/shell-audit-regressions.spec.ts`：在更新状态与最近项目均延迟时验证设置页仍可立即进入。
- Rust `cargo check/test`：验证异步命令路由、项目扫描及现有会话约束。
