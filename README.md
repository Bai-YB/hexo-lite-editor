# Hexo Lite Editor

Hexo Lite Editor 是面向 Windows 10/11 的独立 Hexo 桌面写作与发布工作区，基于 Tauri 2、Svelte 5、TypeScript 与 Rust。

当前版本：`1.0.3`

## 下载

- [Windows x64 安装版（推荐）](https://github.com/Bai-YB/hexo-lite-editor/releases/download/v1.0.3/Hexo-Lite-Editor_1.0.3_windows-x64-setup.exe)
- [Windows x64 免安装版](https://github.com/Bai-YB/hexo-lite-editor/releases/download/v1.0.3/Hexo-Lite-Editor_1.0.3_windows-x64-portable.zip)
- [Windows x64 MSI](https://github.com/Bai-YB/hexo-lite-editor/releases/download/v1.0.3/Hexo-Lite-Editor_1.0.3_windows-x64.msi)
- [版本记录](CHANGELOG.md)

安装版下载后运行 EXE，按向导完成安装。免安装版解压后直接双击 `Hexo Lite Editor.exe`，不需要打开终端。

本项目当前未使用商业代码签名证书，Windows SmartScreen 可能显示未知发布者提示。请只从本仓库 Releases 下载，并使用 Release 中的 `SHA256SUMS.txt` 校验文件。

安装器会在需要时补齐 Microsoft Edge WebView2 Runtime。免安装版若检测不到 WebView2，会在应用界面创建前显示系统提示并引导至微软官方下载页。

## 1.0.3 主要变化

- 编辑器顶栏压缩为单行项目切换与写作操作；左侧导航只保留编辑器、图床、设置和关于。
- 发布继续保留编辑器主按钮与 `Ctrl + Shift + P`，clean、generate、deploy、Git 状态和预览启停收进紧凑高级菜单。
- 修复长文章滚轮、PageDown、深色光标和三栏独立滚动；切换文章后恢复正文与预览滚动位置。
- 右侧提供安全 Markdown 与隔离 Hexo 主题双模式预览；真实文章路由由项目自身 Hexo API 解析。
- Rust 持有 Hexo Server 状态机，端口就绪后才标记运行；发布任务会无感停服并按原状态恢复。
- 图床升级为目录型资源浏览器，文件夹、图片、压缩包、文档、音视频和普通文件使用正确图标。
- 图片双击或 Enter 打开可缩放灯箱；三点、右键和 `Shift + F10` 使用同一动作菜单，不再从图床插入文章。
- 删除自动任务抽屉，任务摘要保持安静；脱敏 JSONL 日志只在设置的“诊断与日志”中主动查看。
- 配置升级为 `schemaVersion: 3`，从 V2 自动备份迁移，新增自动预览、草稿预览和日志轮转设置。
- 关于页只保留简介、版本、主页、许可证和更新检查。
- 使用 36px 自绘标题栏、88px 导航栏和 Quiet Pro 中性视觉系统，提供浅色、深色与跟随系统模式。
- Rust 后端持有单活动项目会话。文章、图片和任务通过项目 ID、资源 ID 与 session generation 访问，不再向前端开放任意路径读写。
- 保存请求绑定文章 ID 与 revision，同一文章串行写入；切换文章、项目和关闭窗口时统一保护未保存内容。
- Markdown 原始 HTML 默认禁用，DOMPurify 使用严格白名单，生产环境启用固定 CSP。
- Cloudflare Token 存入系统凭据库，前端只能设置、查询状态或删除，不能读取明文。
- 本地图床限定 `source/images`，只接受 PNG、JPEG、GIF 与 WebP，删除操作进入系统回收站。

## 快捷键

| 快捷键 | 操作 |
| --- | --- |
| `Ctrl + O` | 打开 Hexo 项目 |
| `Ctrl + S` | 保存当前文章 |
| `Ctrl + N` | 新建文章 |
| `Ctrl + F` | 在编辑器中搜索 |
| `Ctrl + Shift + P` | 发布当前博客 |
| `Ctrl + ,` | 打开设置 |
| `Ctrl + 1…4` | 切换四个主工作区 |
| `↑ / ↓` | 在聚焦的文章列表中切换文章 |

## 开发与验证

```bash
pnpm install
pnpm check
pnpm test
pnpm test:e2e
pnpm build
pnpm tauri dev
```

Rust 检查：

```bash
cd src-tauri
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

## 构建 Windows 安装包

```bash
pnpm tauri build
```

构建目标固定为 MSI 与 NSIS。生产界面加载安装包内的静态资源，不依赖浏览器、Vite Server 或 Hexo Server。

生成可发布的安装版、MSI、免安装 ZIP、SHA-256 和发行清单：

```powershell
pnpm release:windows
```

应用不内置 Node.js、Hexo 或博客依赖。这些依赖只在用户主动执行预览、生成或部署任务时需要，不影响应用本身启动。

## 设计参考

1.0.3 延续参考 Patina 的 Quiet Pro 层级、密度、状态与桌面窗口方法，但不复制其品牌、图标、页面内容或业务组件。
