# 版本与发布模块

## 版本映射

- 对外版本与标签：`1.0.6.5.3` / `v1.0.6.5.3`。
- Cargo、Tauri、更新器 SemVer：`1.0.6+5.3`。
- Windows MSI：`1.0.6053`（Windows Installer 只比较前三段，第三段编码修订号；该值在下一次 Windows 构建时生效）。
- macOS `CFBundleShortVersionString`：`1.0.6`；`CFBundleVersion`：`1.1.2`。

`package.json` 是发布脚本读取的版本源，`release-version.mjs` 将内部构建元数据（如 `+5.1`）映射为对外修订段（如 `.5.1`）。Cargo、Tauri、WiX、Info.plist、工作流默认值和打包脚本必须同时一致。

## 发布调用链

1. 本地完成类型检查、单元测试、双引擎交互、生产构建、依赖审计、Rust fmt/clippy/test 和版本一致性检查。
   如果审计源在标签构建期间新增安全公告，立即取消该源码 SHA 的双平台任务，更新锁文件并重新完成前端回归；发布标签只指向修复后的统一提交。
2. 提交并推送同一源码 SHA；创建 `v1.0.6.5.2` 标签。
3. Windows 与 macOS 工作流从同一标签构建平台包、更新包、签名、哈希与平台 manifest。
4. finalize 工作流核对两个平台的源码 SHA、文件 SHA256、更新包大小和 Minisign 签名，生成三平台 `latest.json` 后公开 Release。
5. 从公开 Release 下载全部资产到独立临时目录，再次验证数量、哈希、大小、签名、Latest/草稿状态。
6. 把实际运行号、发布提交、资产清单与验证结果写回 `docs/validation-1.0.6.5.3.md` 和 `docs/archive/1.0.6.5.3.md`。
7. 最后删除工作区内 `.svelte-kit`、`build`、`output`、`node_modules`、`src-tauri/target`、`src-tauri/gen/schemas` 与临时发布下载；不清理用户博客、应用数据或全局缓存。

## macOS 专用热修复通道

只影响 macOS 的修复可以不等待 Windows 构建，走单独的通道：

1. 按同一份源码提交推送 `main`，然后手动触发 `Build macOS`（`version` 必须与 `package.json` 一致）。该工作流校验版本后创建草稿 Release 并上传 macOS 资产；草稿 Release 在公开前不会创建标签引用。
2. 手动触发 `Finalize macOS hotfix`，传入同一 `version` 与 `source_commit`。工作流只接受 `Build macOS` 在该提交上的成功结果，拒绝任何 Windows 资产，生成只声明 `darwin-x86_64` 与 `darwin-aarch64` 的 `latest.json`，校验更新包签名与 SHA256 后公开 Release 并置为 Latest。
3. 该通道不写标签：工作流 token 无权创建受保护的 `v*` 标签，公开时由 GitHub 按 Release 的 target commit 自动创建 `v<版本>`，工作流公开后会再校验该标签指向同一源码提交。需要人工打 annotated 标签时，按双平台流程推送标签即可，但会同时触发 Windows 构建。
4. Windows 客户端在本版本为 Latest 期间手动检查更新会提示检查失败（不会下载或安装），`windows-x86_64` 缺失时不会命中任何更新包；下一次双平台发布会写回含 Windows 的 `latest.json`，恢复正常。

## 文档发布门槛

每次版本或功能调整必须同时更新：

- 本目录中受影响模块的职责与调用链；
- `CHANGELOG.md`；
- `docs/archive/<版本>.md`；
- `docs/validation-<版本>.md`；
- `.github/release-notes/v<版本>.md`；
- 中英文 README 的当前版本入口。

`scripts/release-version.test.mjs` 将这些文档与 `package.json.releaseVersion` 绑定，缺少或版本不同会使测试失败。

## Windows 启动响应门槛

Windows 安装版和便携版的冒烟测试不会在窗口第一次出现时立刻判定成功，而是在最多 15 秒的观察期内，要求窗口句柄存在且进程连续 3 秒保持 `Responding`。任一采样点失去响应都会重新计时；应用提前退出或始终无法连续响应会阻止产物上传。该门槛覆盖“窗口已显示，但启动恢复或同步工作阻塞 UI 线程”的回归。
