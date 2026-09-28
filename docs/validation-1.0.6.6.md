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

待双平台构建、finalize 与远端资产复核完成后填写源码提交、工作流运行号、Release 状态、资产数量、SHA256 和更新签名验证结果。
