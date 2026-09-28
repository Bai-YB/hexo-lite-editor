# Hexo Lite Editor 1.0.6.6 验证记录

## 验证范围

macOS 图标占比、按工作区域适配的窗口、三档写作布局、文章侧栏、编辑/预览切换、专注模式、Windows 布局回归、双平台版本映射与更新资产。

## 本地预检

- `pnpm check`：0 错误、0 警告；`pnpm test`：33 个文件、193 项通过；`pnpm build`：通过。
- `pnpm audit --prod --registry=https://registry.npmjs.org/`：未发现已知漏洞（本机配置的镜像站不提供审计端点）。
- Windows 本机 `cargo fmt --check`、`cargo clippy --all-targets --all-features -- -D warnings` 通过；`cargo test --all-targets --all-features`：123 项通过。
- WebKit 端到端测试：62 项通过。全量首轮 61 项通过，唯一失败是关于页测试写死旧版本号；修正后针对该项重跑通过。
- Chromium 以 macOS User-Agent 检查 1280×800 双栏和 980×700 单栏：编辑/预览切换、文章抽屉、专注模式正常，页面没有水平溢出。该模拟仅验证前端布局与交互；原生 Dock 与 WKWebView 效果仍需 macOS 实机复核。

## 发布结果

- 发布源码提交：`66139e70dc66a881bd0ac5bf58740754bd7d459c`；附注标签：`v1.0.6.6`。
- [CI](https://github.com/Bai-YB/hexo-lite-editor/actions/runs/36428698449)、[macOS 预检与打包](https://github.com/Bai-YB/hexo-lite-editor/actions/runs/36428710367)、[标签 macOS 构建](https://github.com/Bai-YB/hexo-lite-editor/actions/runs/36430526715)、[标签 Windows 构建](https://github.com/Bai-YB/hexo-lite-editor/actions/runs/36430526798) 和 [最终发布校验](https://github.com/Bai-YB/hexo-lite-editor/actions/runs/36433408541) 均成功。
- [公开 Release](https://github.com/Bai-YB/hexo-lite-editor/releases/tag/v1.0.6.6) 已设为 Latest，包含 14 个资产：Windows 5 个包/签名、macOS 4 个包/签名、2 个平台 manifest、2 个 SHA256 清单及 `latest.json`。两平台 manifest 的版本与源码提交一致。
- 从公开 Release 独立下载全部资产后，12 条 SHA256 记录逐一校验通过；`latest.json` 的 `windows-x86_64`、`darwin-x86_64`、`darwin-aarch64` 条目、更新包大小及 Windows/macOS 更新签名验证通过。
- 本机没有 macOS Dock 实机视觉验收环境；原生编译、WebKit 交互及 `.icns` 打包已经由 macOS Runner 验证。
