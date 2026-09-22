# 模块职责与调用链

文档基线版本：1.0.6.5.2。

本目录记录生产代码各模块的职责、入口、调用关系和跨模块约束。功能或版本发生变化时，必须先更新受影响模块文档，再完成版本归档与发布验证；`scripts/release-version.test.mjs` 会检查模块文档、版本归档、验证记录和发布说明是否与当前版本同时存在。

| 模块 | 主要入口 | 作用 | 主要调用方向 |
| --- | --- | --- | --- |
| 应用外壳与导航 | `src/app/AppShell.svelte` | 配置装载、页面切换、全局快捷键、关闭保护、通知与启动编排 | UI → `platform`；外壳 → 编辑器/文件/设置状态仓库 |
| 项目与文章 | `src-tauri/src/commands/project.rs` | 打开 Hexo 项目、扫描文章、维护最近项目与项目会话 | Tauri IPC → 校验/文章扫描/同步调度 → `AppState` |
| Markdown 编辑器 | `src/features/editor/EditorPage.svelte`、`MarkdownEditor.svelte` | 文档装载、编辑、保存、文章操作、即时预览和单向滚动跟随 | 页面原子状态 → `EditorSessionStore` 分组通道/`platform`/预览渲染管线 |
| HTML 即时预览 | `src/shared/markdown/safeMarkdown.ts`、`previewBlocks.ts`、`renderScheduler.ts`、`previewSurface.ts` | Markdown/HTML 解析、CSS 限定、DOM 清洗、图片源收集、源码锚点、分块增量渲染与自适应调度 | 编辑器内容 → 渲染模型 → 分块安全 HTML → 仅变化块 DOM |
| 全部文件 | `src/features/files`、`src-tauri/src/commands/files.rs` | 项目目录浏览、文本读取保存、冲突和只读限制 | 文件页 → `FileSessionStore` → 文件 IPC |
| 设置与配置 | `src/features/settings`、`src-tauri/src/commands/config.rs` | 六组设置即时生效：改动防抖持久化、校验失败回滚并聚焦、主题/语言预览和连接设置 | 设置页 → config IPC/同步 IPC（无独立控制器） |
| 图片与图床 | `src/features/image-bed`、`src-tauri/src/commands/images.rs` | 本地图片、编辑器图片缓存、Cloudflare 资源管理和后台上传 | 编辑器/图床页 → 图片 IPC → 缓存或远端服务 |
| 插件 | `src/features/plugins`、`src/shared/plugins`、`src-tauri/src/plugins` | 插件安装、权限、Worker 隔离、设置与图床提供者桥接 | 插件页/编辑器 → Provider runtime → Worker/native plugin API |
| 预览与发布任务 | `src-tauri/src/commands/preview.rs`、`tasks.rs` | Hexo 真实预览、生成、发布、日志、取消与状态事件 | UI → 后台任务 → Hexo/系统进程 → 任务事件 |
| 项目同步 | `src-tauri/src/commands/sync.rs` | GitHub/WebDAV 预检、增量清单、冲突、备份和后台进度 | 设置/编辑器 → 同步命令 → 远端 → 项目重扫事件 |
| 更新 | `src/features/update`、`src-tauri/src/commands/update.rs` | 检查、下载进度、签名验证和安装状态 | 外壳/关于页 → updater IPC → Tauri updater |
| 国际化与共用 UI | `src/shared/i18n`、`src/shared/components` | 显式中英文词条、模态栈、加载/错误/空状态等共用交互 | 各页面 → `$ui`/共用组件 |
| 发布工程 | `.github/workflows`、`scripts/package-*`、`scripts/verify-*` | 双平台构建、签名、哈希、更新清单、集中公开和发布后校验 | 版本元数据 → CI/平台打包 → finalize → GitHub Release |

当前改动涉及的详细链路：

- [启动与项目打开](startup-and-projects.md)
- [Markdown、HTML 与滚动预览](markdown-preview.md)
- [内容同步](content-sync.md)
- [版本与发布](release-process.md)
