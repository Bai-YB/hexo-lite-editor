# 版本与发布模块

## 版本映射

- 对外版本与标签：`1.0.6.5.1` / `v1.0.6.5.1`。
- Cargo、Tauri、更新器 SemVer：`1.0.6+5.1`。
- Windows MSI：`1.0.6051`（Windows Installer 只比较前三段，第三段编码修订号）。
- macOS `CFBundleShortVersionString`：`1.0.6`；`CFBundleVersion`：`1.1.0`。

`package.json` 是发布脚本读取的版本源，`release-version.mjs` 将内部构建元数据（如 `+5.1`）映射为对外修订段（如 `.5.1`）。Cargo、Tauri、WiX、Info.plist、工作流默认值和打包脚本必须同时一致。

## 发布调用链

1. 本地完成类型检查、单元测试、双引擎交互、生产构建、依赖审计、Rust fmt/clippy/test 和版本一致性检查。
   如果审计源在标签构建期间新增安全公告，立即取消该源码 SHA 的双平台任务，更新锁文件并重新完成前端回归；发布标签只指向修复后的统一提交。
2. 提交并推送同一源码 SHA；创建 `v1.0.6.5.1` 标签。
3. Windows 与 macOS 工作流从同一标签构建平台包、更新包、签名、哈希与平台 manifest。
4. finalize 工作流核对两个平台的源码 SHA、文件 SHA256、更新包大小和 Minisign 签名，生成三平台 `latest.json` 后公开 Release。
5. 从公开 Release 下载全部资产到独立临时目录，再次验证数量、哈希、大小、签名、Latest/草稿状态。
6. 把实际运行号、发布提交、资产清单与验证结果写回 `docs/validation-1.0.6.5.1.md` 和 `docs/archive/1.0.6.5.1.md`。
7. 最后删除工作区内 `.svelte-kit`、`build`、`output`、`node_modules`、`src-tauri/target`、`src-tauri/gen/schemas` 与临时发布下载；不清理用户博客、应用数据或全局缓存。

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
