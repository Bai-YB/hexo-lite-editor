# Hexo Lite Editor 1.0.6.5.2 验证记录

## 修改与回归范围

Windows 更新器的安装目录参数、NSIS 安装钩子（更新后桌面快捷方式）、可执行文件命名、安装包冒烟脚本、发布版本映射，以及中英文下载入口和发布说明。

## 本地预检

- `node scripts/release-version.mjs` 与 `scripts/release-version.test.mjs`：通过；公开版本 `1.0.6.5.2` 与运行时版本 `1.0.6+5.2` 映射一致，文档门禁同时要求当前版本的模块文档基线、版本归档、验证记录与发布说明存在。
- `pnpm check`：0 错误、0 警告；`pnpm test`：33 文件、193 用例全部通过；`pnpm build`：通过。
- `pnpm audit --prod --registry=https://registry.npmjs.org`：无已知漏洞。
- `cargo fmt --check`、`cargo clippy --all-targets --all-features -- -D warnings`、`cargo test --all-targets --all-features`：通过；118 个 Rust 用例全部通过（新增“安装目录参数保持原地更新”和“只有 NSIS 安装包接收安装目录参数”两条用例），并确认 `1.0.6+5.2` 高于 `1.0.6+5.1`。
- `pnpm exec playwright test --project=desktop-chromium tests/e2e/desktop.spec.ts`：30 项通过，包含界面版本显示 `1.0.6.5.2`。
- `scripts/package-release.ps1 -Version 1.0.6.5.2`：本机以临时签名密钥构建通过，产出 Setup EXE、MSI、便携 ZIP、`release-manifest.json` 与 `SHA256SUMS.txt`，安装器构建同时验证了 `nsis/hooks.nsh` 可编译。
- `scripts/smoke-portable.ps1`：通过；便携包启动后连续 3 秒保持响应，文件版本为 `1.0.6+5.2`。
- `scripts/smoke-installers.ps1`：通过；NSIS 安装/启动、MSI 管理提取，以及新增的原地更新断言——删除桌面快捷方式后执行 `/S /UPDATE /D=<安装目录>`，`Hexo Lite Editor.exe` 与 `resources/` 仍在原目录，桌面快捷方式已重建并指向该可执行文件；脚本结束时清除了卸载注册表项与临时目录。
- 本机清理：删除指向 `%TEMP%\hlex-installer-smoke-*` 的卸载注册表项，以及 `Hexo Lite Editor-*-updater-*`、`hlex-*` 临时残留；用户博客、应用数据与全局缓存未动。

## 发布结果

（发布完成后补录：标签提交、CI 与双平台工作流运行号、公开 Release 时间、14 项资产与哈希/签名复核结果。）
