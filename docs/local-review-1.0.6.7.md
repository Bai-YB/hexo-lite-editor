# 1.0.6.7 本地候选：验收与安全审查

本记录始于本地 `work/1.0.6.7-local-review` 候选分支；后续版本映射与正式发布结果见 [验证记录](validation-1.0.6.7.md)。

## 本地验收

在本目录运行 `pnpm dev --host 127.0.0.1 --port 1420`，然后用浏览器打开 <http://127.0.0.1:1420/?demo=1&macLayout=1&updateAvailable=1>。`macLayout=1` 仅对演示页生效，便于 Windows 电脑检查 macOS 布局。

1. 把浏览器窗口调至约 980×700。文章列表应默认展开并占据自己的列，编辑区或预览区从其右侧开始；切换文章、切换编辑/预览后列表仍保持展开，按 Escape 或列表按钮可收起。手动收起后调整窗口尺寸不应重新展开。
2. 输入 `#` 至 `######` 的各级标题，以及带 `js`、`ts`、`html`、`css` 等语言标记的围栏代码；检查编辑区与预览区的标题层级、代码颜色。未知语言保留转义后的纯文本。
3. 点击右下角“查看更新”，应进入更新页；后台任务通知中的“任务详情”应打开详情对话框。
4. 打开 [macOS 图标预览](../src-tauri/icons/icon-macos.png)。图案占画布 74%；Dock 实际大小和悬停放大效果需要在 macOS 实机复核。

本地截图：[侧栏分栏](../output/local-review/mac-sidebar-split.png)、[Markdown 编辑区](../output/local-review/mac-markdown-editor.png)、[Markdown 预览](../output/local-review/mac-markdown-preview.png)。这些截图是未跟踪的本地检查产物，不是发布资产。

## 已完成的代码审查

| 范围 | 结果与处理 |
| --- | --- |
| 前端内容注入 | 预览继续使用 DOMPurify 清理 HTML，代码着色仅对登记的语言产生 `<span>` 标记，并再次经过原有清理链；未知语言和超长代码块由 markdown-it 转义。Tauri CSP 仍禁止脚本内联执行、对象和框架。 |
| 布局与可操作性 | macOS 紧凑布局默认显示 flex 独立列，手动收起状态在窗口宽度变化时保留；右下角通知容器允许内部按钮接收点击。Chromium、WebKit 已验证更新跳转和任务详情。新增语法颜色在亮色正文/代码背景及暗色背景上的最小对比度均大于 4.5:1。 |
| 插件网络 | 修复无 `Content-Length` 的响应先完整 `arrayBuffer()` 再检查大小的问题；现在逐块计数，超过默认 5 MiB 即取消流。保留 HTTPS、精确来源权限、禁止自动重定向和超时限制。 |
| 原生文件与外链 | 已复核项目文件路径逐段规范化与符号链接拒绝、Markdown 外链限 HTTP/HTTPS 且不带凭据、WebDAV/图床生产环境 HTTPS 约束、Git 以参数数组执行。修正 Hexo 路由失败时两处乱码提示。 |
| 项目信任边界 | 预览与发布会按设计执行所选 Hexo 项目的本地 Node 脚本。用户打开来源不明的项目之前仍需确认其脚本与依赖；这属于项目执行能力，不能由 Markdown 消毒器代替隔离。 |
| 依赖公告 | `pnpm audit --prod --registry=https://registry.npmjs.org/` 未发现已知漏洞。Cargo 锁文件将 `rustls` 升至 0.23.45、`plist` 升至 1.10.1，并将 Linux `wayland-scanner` 升至 0.31.11，改用已修复的 `quick-xml` 0.41.0；`cargo audit` 无漏洞报告。 |

## 发布前仍需核对

- `cargo audit` 的两条 `quick-xml` 高危公告已随 `wayland-scanner` 更新消除。审计仍列出 8 条维护/健全性提醒；其中两条健全性提醒的依赖不在 Windows/macOS 目标链中。不能将全锁文件表述为“零告警”。
- 86 个生产 JavaScript 包的许可文本归集到随安装包分发的 `THIRD_PARTY_NOTICES.txt`，便携版、NSIS、MSI 冒烟测试均检查其存在。Rust 依赖的许可清单仍需结合 Cargo 锁文件和源包核对；本记录是工程审查，不作为法律合规证明。
- Windows 本机无法验证 macOS Dock 的真实视觉尺寸或 macOS 原生编译。用户验收后需在 macOS 构建环境复核，再准备发布。

## 测试记录

- `pnpm check`：0 错误、0 警告；`pnpm test`：33 个文件、195 项通过；`pnpm build`：通过。
- macOS 布局/着色/右下角通知专项 E2E：Chromium 与 WebKit 各 3 项通过。
- 完整 Playwright 回归：Chromium 与 WebKit 共 130 项通过；`output/playwright/results/.last-run.json` 记录 `passed`，无失败项。
- `cargo fmt --check`、`cargo clippy --all-targets --all-features -- -D warnings` 通过；`cargo test --all-targets --all-features`：123 项通过。
