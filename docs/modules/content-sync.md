# 内容同步模块

## 职责

`SettingsPage.svelte` 负责同步方式选择、连接预检、状态/进度、首次合并、冲突选择和高级覆盖操作；`SyncProviderPicker.svelte` 只负责 GitHub/WebDAV 的清晰互斥选择。`platform/tauri.ts` 定义前端 IPC 合约，`sync.rs` 实现范围扫描、远端比较、合并、备份、传输与状态持久化，`sync_runtime.rs` 统一把长任务放入 blocking worker，并提供进度、停止和超时。

## 设置加载调用链

1. 打开普通设置分类时不检测仓库、不读取系统凭据，也不扫描项目文件。
2. 用户首次进入“文件同步”后，`refreshSync()` 并行读取 GitHub 候选、当前同步状态和现有任务进度；基础状态返回后立即展示页面。
3. 项目范围摘要、冲突详情和 WebDAV 凭据状态随后独立后台读取，不阻挡同步方式选择或页面导航。
4. 同步状态事件只在用户位于同步分类时触发摘要重扫，避免在常规、编辑器等设置分类后台读取整个项目。

## 配置与同步规划

未启用同步时，页面先展示三个固定阶段：

1. 选择 GitHub 或 WebDAV；只挂载所选方式对应的配置表单。
2. 检查连接与两端差异。GitHub 使用独立内容分支；WebDAV 验证真实读写权限、凭据和指定远端目录。预检不修改任一端文件。
3. 合并并开始同步。首次合并保留两端独有文件，同路径内容不同时进入逐文件冲突选择。

启用后页面只显示当前 provider、同步状态、本次变化和对应连接信息。保存操作把本地状态标为待同步，并在约 30 秒后调度增量上传；“立即同步”读取两端最新状态并合并。整端覆盖、备份目录和关闭同步保留在折叠的高级操作中。

## 原生执行链

- GitHub 预检：`preflight_content_sync` → `sync_runtime::run` → 本地快照 → GitHub 远端 fetch → manifest 校验 → 差异统计。
- WebDAV 测试：`test_webdav_content_sync` → worker → 系统凭据读取/真实读写探测 → 远端 manifest 与对象校验 → 成功后写入系统凭据库。
- 手动同步：`run_content_sync` → worker → 本地快照 → 远端 fetch → 三方比较 → 自动合并或冲突状态 → 备份/应用 → 上传 manifest → 项目重扫事件。
- 状态、凭据查询/删除和关闭同步中的磁盘或系统凭据操作也通过 `spawn_blocking` 执行，避免占用 Windows WebView2 或 macOS WKWebView 的主线程。

所有异步结果都校验 `projectId + sessionGeneration`。停止操作只设置当前 generation 的取消标记；每个网络请求和整体任务均有超时，已经完成的远端写入不会被伪装成撤销。

## 同步范围

同步完整站点源码，包括文章、草稿、Hexo 配置、主题、数据文件、页面与资源。依赖、生成目录、缓存、Git 内部文件和敏感凭据被排除。摘要按“文章与草稿 / 配置与页面 / 主题与模块 / 图片与资源”分类，并根据上次成功 baseline 统计新增、修改和删除。

## 回归入口

- `SettingsPage.test.ts`：普通设置不触发扫描、provider 互斥显示、迟到 WebDAV 结果、同步进度/停止、冲突与远端领先。
- `tests/e2e/desktop.spec.ts`：GitHub 公开确认、首次预检、多仓库选择、WebDAV 真实测试和认证恢复。
- `tests/e2e/sync-progress.spec.ts`：操作期间 UI 可响应、重复提交禁用、停止完成后才能重试。
- `tests/e2e/platform-parity.spec.ts`：Chromium/WebKit 在三种窗口宽度下同步分类无遮挡、键盘可达且布局一致。
- Rust `cargo test sync --all-features`：Git/WebDAV 合并、删除、冲突、缓存、备份、generation、取消、超时和真实本地测试服务器。
