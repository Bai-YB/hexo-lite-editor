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

- 发布标签 `v1.0.6.5.1`（annotated，`e22b77b9`）指向源码提交 `b28ae9f8`（`main`）；Release 于 2026-09-21T10:01:32Z 公开：https://github.com/Bai-YB/hexo-lite-editor/releases/tag/v1.0.6.5.1 ，状态为公开（非草稿、非预发布），并已置为 Latest。
- 工作流运行（同一源码提交 `b28ae9f8`）：CI success run/35583040934；Release Windows success run/35584019575；Build macOS success run/35584019645；Finalize release success run/35585764006；Finalize release success run/35586449996。
- 公开资产共 14 项：Hexo-Lite-Editor_1.0.6.5.1_macos-universal.app.tar.gz、Hexo-Lite-Editor_1.0.6.5.1_macos-universal.app.tar.gz.sig、Hexo-Lite-Editor_1.0.6.5.1_macos-universal.app.zip、Hexo-Lite-Editor_1.0.6.5.1_macos-universal.dmg、Hexo-Lite-Editor_1.0.6.5.1_windows-x64-portable.zip、Hexo-Lite-Editor_1.0.6.5.1_windows-x64-setup.exe、Hexo-Lite-Editor_1.0.6.5.1_windows-x64-setup.exe.sig、Hexo-Lite-Editor_1.0.6.5.1_windows-x64.msi、Hexo-Lite-Editor_1.0.6.5.1_windows-x64.msi.sig、latest.json、release-manifest-macos.json、release-manifest.json、SHA256SUMS-macos.txt、SHA256SUMS.txt。
- 资产复核：从公开 Release 重新下载全部 14 项资产（合计 79 930 140 字节），逐项 SHA256 与 `SHA256SUMS.txt`、`SHA256SUMS-macos.txt` 及 GitHub 资产摘要一致；两份 manifest 记录的 `sourceCommit` 均为 `b28ae9f8`。
- 更新通道复核：`latest.json` 运行版本 `1.0.6+5.1`，覆盖 windows-x86_64、darwin-x86_64、darwin-aarch64 三个平台，包大小与记录一致；`scripts/verify-updater-signatures.mjs` 以 `tauri.conf.json` 内嵌公钥校验 `Hexo-Lite-Editor_1.0.6.5.1_windows-x64-setup.exe` 与 `Hexo-Lite-Editor_1.0.6.5.1_macos-universal.app.tar.gz` 的 Minisign 包签名与 trusted comment，均通过。
