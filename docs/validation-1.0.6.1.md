# Hexo Lite Editor 1.0.6.1 验证记录

本记录对应 2026-09-11 的交互代码审计，发布标签为 `v1.0.6.1`。对外版本为 `1.0.6.1`，Tauri、Cargo 和更新清单使用合法的内部版本 `1.0.6+1`；Windows MSI/WiX 版本为 `1.0.6.1`，macOS bundle 使用数字版本 `CFBundleShortVersionString=1.0.6`、`CFBundleVersion=1.0.601`。

## 需求与实现

- GitHub / WebDAV 同步在后台 worker 执行，显示阶段、文件进度和耗时；每个网络请求有上限，整轮同步有 deadline，支持停止、重试和同项目并发保护。WebDAV 使用经过 hash 校验的下载缓存，只上传变化对象。打开项目先完成本地扫描，再后台检查远端状态。
- 编辑器和即时预览按源码块与行号锚点双向定位，保留 front matter 偏移；代码块、长折行、图片加载和窗口尺寸变化会重新测量，滚动同步有反馈回路保护。
- 插件管理从图床设置移到左侧独立“插件”工作区，保留安装、启用、配置、测试和卸载操作。快捷键为 Ctrl+5；原 Ctrl+1–4 保持不变。
- “全部文件”工作区使用懒加载目录树和相对路径打开文件。友链、YAML、JSON、JavaScript、TypeScript、CSS、HTML、Markdown 等 UTF-8 文本可编辑并按扩展名高亮；二进制、超大文件和 Git 内部文件只读。保存使用 hash CAS、原子写入、隔离备份和磁盘版本比较，切换、离开、切项目和退出均保护未保存内容。博文路径统一回到文章编辑器，避免双 store 覆盖。

## 自动化证据

- `pnpm check`：0 errors、0 warnings。
- `pnpm test`：27 个测试文件，137 项通过。
- `pnpm build`：生产构建成功。
- `pnpm audit --prod`：No known vulnerabilities found。
- `pnpm test:e2e`：46 项通过，覆盖同步进度/停止、源码锚点滚动、独立插件导航、全部文件树、保存/冲突/离开保护、发布前保存和原有编辑器回归。
- Rust `cargo clippy --all-targets --all-features -- -D warnings`：通过。
- Rust `cargo test --all-targets --all-features`：100 项通过；文件命令新增测试 4 项通过，覆盖 canonical 路径、别名防护、隐藏备份、外部修改 CAS。
- `node --test` 的版本映射和更新清单测试：6 项通过。
- 视觉复核：`output/playwright/1061-visual/` 采集 1360×860 与 1120×720、浅色/深色及英文插件/文件页；确认轮在 `confirmed/`，检查水平溢出、按钮可见性、对比度和窄窗口插件布局。所有页面无水平溢出；WebDAV 首次上传按钮在 1120×720 首屏可见。

## 边界

自动化浏览器使用本地 demo fixture，验证交互状态而非真实用户云端凭据。Rust 测试包含 loopback WebDAV、ETag/hash 增量和取消流程；需要外部 WebDAV 账号的测试在未配置凭据时跳过。未在本机执行 Apple 公证、商业代码签名或真实 Tauri 安装升级；GitHub Actions 会在对应平台构建、签名并上传资产后，再生成和验证三平台 `latest.json`。
