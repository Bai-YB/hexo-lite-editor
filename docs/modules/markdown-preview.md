# Markdown、HTML 与滚动预览模块

## 职责

`safeMarkdown.ts` 把 Markdown 与原始 HTML 转为即时预览所需的安全 HTML。`previewBlocks.ts` 把渲染结果按顶层块拆分并复用上帧未变块，`renderScheduler.ts` 自适应合并渲染请求，`previewSurface.ts` 只替换变化块的 DOM。`EditorPage.svelte` 安排渲染、解析本地图片并维护预览 DOM。`sourceScrollSync.ts` 与 `MarkdownEditor.svelte` 做编辑器到预览的单向定位（预览区自身滚动不再回写编辑器）。

## 渲染调用链

1. `EditorSessionStore` 发布文档内容或图片状态变化。
2. `EditorPage.schedulePreviewRender()` 交给 `createAdaptiveScheduler()`：帧对齐等待后执行，连续输入按内容版本合并请求并自适应跳帧；隐藏即时预览或使用 Hexo 主题预览时不解析。
3. `renderMarkdownPreview()` 单次运行 markdown-it，生成安全 HTML和本地图片源列表，避免此前为图片和正文各解析一次。
4. 原始 `<style>` 内容通过跨 WebView 一致的轻量 CSS 块解析器处理；只保留 `@media`/`@supports`、允许的展示属性和安全自定义变量。每个选择器增加 `.markdown-preview` 前缀，`body/html/head`、外部导入、URL、固定/粘性定位、过大层级和活动表达式被移除。
5. HTML class 保留给限定后的文章 CSS；DOMPurify 继续移除脚本、事件、iframe、表单、SVG、MathML 等活动内容。链接改为受控 `data-external-href`，图片按远端、本地会话资源或错误占位处理。
6. 渲染结果由 `previewBlocks.ts` 拆为顶层块、`hashPreviewText()` 与上帧指纹比对，`previewSurface.ts` 仅替换变化块 DOM 写入 `.markdown-preview`；含裸 HTML 块或拆分失败时回退单块全量渲染（安全优先）。容器使用 layout/paint/style containment，文章 CSS 和绝对定位不能绘制到预览区域之外。

围栏 ` ```html ` 仍表示源码并显示代码，不执行；要渲染的 HTML 必须直接写入 Markdown 正文。

## 单向滚动调用链

1. markdown-it token 的 `map` 写入 `data-source-line`/`data-source-end`，HTML 块把锚点写到实际根元素。
2. `collectPreviewAnchors()` 以预览滚动容器为坐标系收集单调锚点；ResizeObserver 和 MutationObserver 在图片、字体、HTML 展开或 DOM 替换后重建坐标。DOM 替换时立即废弃旧锚点，避免新文章套用上一版坐标。
3. 编辑器滚动（wheel/pointer/touch/key 后发生的 scroll 事件）经 `previewTopForSourceLine()` 局部分段插值驱动预览位置；顶部和底部使用滚动范围边界直接对齐。
4. 预览区自身滚动只移动预览，不再回写编辑器位置，输入不被打断；`sourceLineForPreviewTop()` 仅作保留 API。
5. `scrollToLine()` 先让 CodeMirror 渲染目标 viewport，再精确跳转；异步跳转期间的预览布局重测只按已经映射的源码行定位。单向跟随不存在两个面板互相反馈的问题。

## 回归入口

- `src/shared/markdown/safeMarkdown.test.ts`：HTML/CSS 允许项、限定选择器、危险内容过滤、单次图片源输出。
- `tests/e2e/platform-parity.spec.ts`：Chromium/WebKit 中真实输入带 CSS 的 HTML 卡片并检查计算样式。
- `src/shared/markdown/previewBlocks.test.ts`：分块拆分、指纹复用、HTML 块全量回退。
- `tests/e2e/preview-source-scroll.spec.ts`：长图、换行段落、代码块、迟到布局变化、单向跟随、反复跨段与首尾边界。
