# Hexo Lite Editor 1.0.6.5.3 验证记录

## 修改与回归范围

macOS 内容同步的只读文件写入（应用远端文件、创建备份、恢复备份与回滚）、Node.js 环境诊断的 PATH 口径、macOS 专用发布通道（`Finalize macOS hotfix`）、版本映射与中英文下载入口。

## 本地预检

- `node scripts/release-version.mjs`：输出 `1.0.6.5.3` / `1.0.6+5.3` / `v1.0.6.5.3`，与 `package.json` 一致；`scripts/release-version.test.mjs` 的文档门禁同时确认模块文档基线、版本归档、验证记录与发布说明存在且版本一致。
- `pnpm test`：33 文件、193 用例全部通过（含上面两条文档与版本门禁）。
- `pnpm check`：0 错误、0 警告。
- `pnpm build`：通过。
- `cargo fmt --check`、`cargo clippy --all-targets --all-features -- -D warnings`：通过，无告警。
- `cargo test --all-targets --all-features`：123 个 Rust 用例全部通过，其中新增「只读项目文件仍可应用远端内容」「只读项目文件可回滚恢复」「恢复失败信息包含具体路径」「macOS 候选路径覆盖 Homebrew 与版本管理器」「命令探测与补齐 PATH 一致」五条用例，并确认 `1.0.6+5.3` 高于 `1.0.6+5.2`（Tauri updater 2.10.1 的版本比较包含 build metadata）。
- 本机为 Windows，macOS 通用包构建与 WebKit 交互回归由 `Build macOS` 工作流在 macOS runner 上执行。

## 发布结果

- 源码提交 `c83403ea7ce47396126a2c7fa8a47c04909298ae`（`main`）同时用于本版本的 macOS 构建与发布；标签 `v1.0.6.5.3` 在草稿公开时由 GitHub 按该提交创建，复核为轻量标签且 `object.sha` 等于该源码提交。
- 工作流运行（同一源码提交）：`Build macOS` success run/35740815100（含前端 `pnpm check`、`pnpm test`、`pnpm build`、`cargo fmt/clippy/test` 与 desktop-webkit 交互回归）；`Finalize macOS hotfix` success run/35743548171。
- 发布通道第一次尝试（run/35743178925）因草稿 Release 尚无 tag ref、`gh api git/ref/tags` 返回 404 失败；第二次（run/35743392026）因工作流 token 无权创建受保护的 `v*` 标签返回 403 失败。两次都发生在写入任何公开状态之前，草稿资产未被改动；发布通道改为不写标签、并在公开后校验标签指向后重跑成功。
- Release 于 2026-09-22T14:54:21Z 前后公开并置为 Latest：https://github.com/Bai-YB/hexo-lite-editor/releases/tag/v1.0.6.5.3 ，`repos/.../releases/latest` 解析为 `v1.0.6.5.3`。
- 公开资产共 7 项且全部为 macOS：`Hexo-Lite-Editor_1.0.6.5.3_macos-universal.dmg`（19 327 241 字节）、`Hexo-Lite-Editor_1.0.6.5.3_macos-universal.app.zip`（17 758 608 字节）、`Hexo-Lite-Editor_1.0.6.5.3_macos-universal.app.tar.gz`（18 205 529 字节）、`Hexo-Lite-Editor_1.0.6.5.3_macos-universal.app.tar.gz.sig`（416 字节）、`release-manifest-macos.json`（564 字节）、`SHA256SUMS-macos.txt`（646 字节）、`latest.json`（3 989 字节）；Release 中不含任何 Windows 资产。
- 资产复核：`release-manifest-macos.json` 的 `version`、`runtimeVersion`、`bundleShortVersion`、`bundleVersion`、`sourceCommit`、`architecture` 分别为 `1.0.6.5.3`、`1.0.6+5.3`、`1.0.6`、`1.1.2`、`c83403ea7ce47396126a2c7fa8a47c04909298ae`、`x86_64 arm64`；`SHA256SUMS-macos.txt` 的 6 条校验值（4 个安装包资产 + manifest + `latest.json`）与 GitHub 记录的资产 SHA256 摘要逐项一致。本机到 GitHub Release 大文件的下载受限，因此哈希比对使用 GitHub 资产摘要完成，打包产物本身的签名校验在发布工作流内对已下载资产执行。
- 更新通道复核：`latest.json` 的运行版本为 `1.0.6+5.3`，`platforms` 只含 `darwin-x86_64` 与 `darwin-aarch64`，两者指向同一签名更新包；工作流在公开前用 `tauri.conf.json` 内嵌公钥校验该更新包的 Minisign 包签名与 trusted comment，日志记录 `Verified updater signature and size: Hexo-Lite-Editor_1.0.6.5.3_macos-universal.app.tar.gz`，并用 `verify-updater-manifest.mjs` 确认清单版本、HTTPS 地址与发布标签。
- Windows 影响：本版本为 Latest 期间更新清单不含 `windows-x86_64`，Windows 客户端手动检查更新会提示检查失败且不会下载或安装任何内容（后台自动检查只记录日志）；下一次双平台发布会写回含 Windows 的 `latest.json` 并恢复正常。
