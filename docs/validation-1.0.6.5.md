# Hexo Lite Editor 1.0.6.5 验证记录

## 修改与回归范围

63 个源码文件：编辑器（EditorPage/MarkdownEditor/EditorSessionStore/预览 preview 目录）、Markdown 渲染（safeMarkdown/previewBlocks/wordCount）、应用外壳（AppShell/ModalDialog/通知）、设置（SettingsPage/SettingsHeader/settingsDraft，controller.ts 删除）、图床与插件页、i18n（uiMessages/errorMessages/语言包）、样式（tokens 与三处特性样式表）、同步状态标签。Rust 侧仅版本号。回归重点：长文输入与打开、滚动跟随、设置落盘与回滚、同步通道切换、主题切换、i18n 词条一致性。

## 本地验证

- `svelte-check`：0 错误 0 警告。
- `vitest run`：33 个文件 193 个用例全绿，含同步通道切换、防抖落盘、校验回滚、词条表等新用例。
- `vite build` 生产构建通过。
- 开发模式 `?demo=1` 冒烟：设置即时生效与状态提示、同步 GitHub/WebDAV 标签切换与回退提示、深浅主题与"跟随系统"实时切换逐项确认。
- 性能以 `window.__previewStats`/`__editorStats` 埋点复核：连续输入下预览仅变化块重绘，重渲染次数较改造前下降。

## 安全与兼容边界

- 含裸 HTML 块的文章走全量渲染回退（安全优先，行为确定）；DOMPurify 与 CSS 限定策略不变。
- 对外命令、事件、插件 API 与配置格式不变，1.0.6.x 可直接升级。
- Windows 产物无商业代码签名，SmartScreen 可能提示未知发布者；更新包仍经验签验证。
- Playwright e2e 本轮由 CI 执行；本地以 vitest + svelte-check + 冒烟为准。

## 发布结果

- 发布标签 `v1.0.6.5`（annotated），源码提交 `main`；Release 于 2026-09-19T12:53:43Z 公开：https://github.com/Bai-YB/hexo-lite-editor/releases/tag/v1.0.6.5。
- 平台工作流运行：Finalize release success run/35424523410；Finalize release skipped run/35424439325；Build macOS success run/35423688551；Release Windows success run/35423688542；CI success run/35423682193。
- 公开资产共 14 项：Hexo-Lite-Editor_1.0.6.5_macos-universal.app.tar.gz、Hexo-Lite-Editor_1.0.6.5_macos-universal.app.tar.gz.sig、Hexo-Lite-Editor_1.0.6.5_macos-universal.app.zip、Hexo-Lite-Editor_1.0.6.5_macos-universal.dmg、Hexo-Lite-Editor_1.0.6.5_windows-x64-portable.zip、Hexo-Lite-Editor_1.0.6.5_windows-x64-setup.exe、Hexo-Lite-Editor_1.0.6.5_windows-x64-setup.exe.sig、Hexo-Lite-Editor_1.0.6.5_windows-x64.msi、Hexo-Lite-Editor_1.0.6.5_windows-x64.msi.sig、latest.json、release-manifest-macos.json、release-manifest.json、SHA256SUMS-macos.txt、SHA256SUMS.txt。Finalize 已核对双平台源码 SHA、资产 SHA256、更新包大小与 Minisign 签名，并生成三平台 `latest.json`。
- 本机工作区缓存（`.svelte-kit`、`build`、`output`、`src-tauri/target` 等）按发布流程清理要求处理；用户博客、应用数据与全局缓存未动。
