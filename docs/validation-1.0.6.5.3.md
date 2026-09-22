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

- 待 `Build macOS` 与 `Finalize macOS hotfix` 运行完成后补录运行号、发布标签、公开资产清单与资产复核结果。
