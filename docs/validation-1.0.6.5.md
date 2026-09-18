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

（CI finalize 后补录：源码提交、双平台工作流运行号、资产清单与哈希复核结果。）
