# Hexo Lite Editor 1.0.6.7 验证记录

## 验证范围

macOS 图标、文章栏默认展开与独立分栏、Markdown 编辑及预览着色、通知点击、插件响应限额、版本映射、Windows/macOS 安装与更新资产。

## 本地预检

- `pnpm check`：0 错误、0 警告；`pnpm test`：33 个文件、195 项通过；`pnpm build`：通过。
- `cargo fmt --check`、`cargo clippy --all-targets --all-features -- -D warnings` 通过；`cargo test --all-targets --all-features`：123 项通过，含 1.0.6.6 至 1.0.6.7 的更新顺序断言。
- macOS 默认文章栏、Markdown 着色与右下角操作专项 E2E：Chromium 和 WebKit 各 3 项通过。干净 Vite 服务下完整双引擎回归 130/130 通过（15.3 分钟）。

## 安全与许可审查

`pnpm audit --prod --registry=https://registry.npmjs.org/` 未发现已知漏洞。将 Linux `wayland-scanner` 升至 0.31.11 后，`cargo audit` 对跨平台锁文件未报告漏洞，仍有 8 条维护/健全性提醒。86 个生产 JavaScript 包的许可文本已放入打包资源。审查范围、依赖提醒及许可材料边界见 [本地验收与安全审查](local-review-1.0.6.7.md)。

## 发布结果

待双平台标签构建、最终校验和独立下载复核后填写。
