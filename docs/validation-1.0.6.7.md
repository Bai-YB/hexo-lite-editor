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

版本标签 `v1.0.6.7` 指向 `ecec04cd1326952e1f6bff95ab38098830883828`。公开 [Release](https://github.com/Bai-YB/hexo-lite-editor/releases/tag/v1.0.6.7) 已于 2026-09-29 发布，设为 Latest，非草稿或预发行，共有 14 个资产。

- [主分支 CI](https://github.com/Bai-YB/hexo-lite-editor/actions/runs/36535747191)、[Windows 构建与安装验证](https://github.com/Bai-YB/hexo-lite-editor/actions/runs/36571775083)、[macOS 通用包与 WebKit 验证](https://github.com/Bai-YB/hexo-lite-editor/actions/runs/36571775187)、[最终发布与资产复核](https://github.com/Bai-YB/hexo-lite-editor/actions/runs/36575492988) 均成功。
- 最终发布流程下载并核验全部资产的哈希、大小、签名及更新清单。本地另行下载 `latest.json`、双平台发布清单和校验文件，确认 3 个更新平台、12 条校验记录与 14 个公开资产一致。
- macOS 应用包未经过 Apple 开发者签名或公证；首次打开可能需要在 Finder 中右键选择“打开”。Windows 安装包没有商业代码签名证书，可能触发 SmartScreen 提示。应用内更新包仍按项目签名校验。
