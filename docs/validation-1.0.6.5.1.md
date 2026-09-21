# Hexo Lite Editor 1.0.6.5.1 验证记录

## 修改与回归范围

Windows 打包启动路径、便携包与安装包冒烟脚本、发布版本映射、Windows MSI/macOS bundle 构建号、发布工作流默认版本，以及中英文下载入口和发布说明。

## 本地预检

- `node scripts/release-version.mjs` 与 `scripts/release-version.test.mjs`：通过；公开版本 `1.0.6.5.1` 与运行时版本 `1.0.6+5.1` 映射一致。
- `pnpm check`：0 错误、0 警告；`pnpm test`：33 文件、193 用例全部通过；`pnpm build`：通过。
- `pnpm audit --prod --registry=https://registry.npmjs.org`：无已知漏洞。
- `cargo fmt --check`、`cargo clippy --all-targets --all-features -- -D warnings`、`cargo test --all-targets --all-features`：通过；116 个 Rust 用例全部通过，并确认 `1.0.6+5.1` 高于 `1.0.6+5`。
- `pnpm exec playwright test --project=desktop-chromium`：62 项通过。
- `pnpm exec tauri build --no-bundle`：通过；原生应用启动冒烟通过，文件版本为 `1.0.6+5.1`。
- 启动响应门槛：安装版和便携版必须在最多 15 秒内连续 3 秒具有窗口句柄且保持 `Responding`。

## 发布结果

发布完成后补录标签提交、CI 与双平台工作流运行、公开 Release 时间、14 项资产与哈希/签名复核结果。
