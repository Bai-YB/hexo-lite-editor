# Markdown、HTML 与滚动预览模块

## 职责

`safeMarkdown.ts` 把 Markdown 与原始 HTML 转为即时预览所需的安全 HTML。`EditorPage.svelte` 安排渲染、解析本地图片并维护预览 DOM。`sourceScrollSync.ts` 与 `MarkdownEditor.svelte` 在 CodeMirror 源码行和预览块之间做双向定位。

## 渲染调用链

1. `EditorSessionStore` 发布文档内容或图片状态变化。
2. `EditorPage.schedulePreviewRender()` 在下一帧后延迟 32 ms 执行；连续输入会取消旧任务，隐藏即时预览或使用 Hexo 主题预览时不解析。
3. `renderMarkdownPreview()` 单次运行 markdown-it，生成安全 HTML和本地图片源列表，避免此前为图片和正文各解析一次。
4. 原始 `<style>` 内容通过跨 WebView 一致的轻量 CSS 块解析器处理；只保留 `@media`/`@supports`、允许的展示属性和安全自定义变量。每个选择器增加 `.markdown-preview` 前缀，`body/html/head`、外部导入、URL、固定/粘性定位、过大层级和活动表达式被移除。
5. HTML class 保留给限定后的文章 CSS；DOMPurify 继续移除脚本、事件、iframe、表单、SVG、MathML 等活动内容。链接改为受控 `data-external-href`，图片按远端、本地会话资源或错误占位处理。
6. 渲染结果通过 Svelte `{@html}` 写入 `.markdown-preview`。容器使用 layout/paint/style containment，文章 CSS 和绝对定位不能绘制到预览区域之外。

围栏 ` ```html ` 仍表示源码并显示代码，不执行；要渲染的 HTML 必须直接写入 Markdown 正文。

## 双向滚动调用链

1. markdown-it token 的 `map` 写入 `data-source-line`/`data-source-end`，HTML 块把锚点写到实际根元素。
2. `collectPreviewAnchors()` 以预览滚动容器为坐标系收集单调锚点；ResizeObserver 和 MutationObserver 在图片、字体、HTML 展开或 DOM 替换后重建坐标。DOM 替换时立即废弃旧锚点，避免新文章套用上一版坐标。
3. 编辑器获得 wheel/pointer/touch/key 输入后成为 owner；`sourceLineAtScroll()` 读取 CodeMirror 顶部锚点行，`previewTopForSourceLine()` 进行局部分段插值并设置预览位置。
4. 预览收到 wheel/pointer/touch/key 输入时立即成为 owner，并记录待处理滚动；锚点暂未重建时等待当前代次坐标，完成后再用 `sourceLineForPreviewTop()` 反向插值。这样 HTML 重渲染不能把刚发生的用户滚动推回编辑器旧位置。
5. `scrollToLine()` 先让 CodeMirror 渲染目标 viewport，再用实际行高精确定位；这段异步跳转期间的预览布局重测只按已经映射的源码行定位，不读取编辑器尚未更新的旧顶部/底部边界。
6. 顶部和底部使用滚动范围边界直接对齐；程序化滚动不会夺取 owner，避免两个面板互相反馈。重复跨段滚动不会积累上一次的坐标误差。

## 回归入口

- `src/shared/markdown/safeMarkdown.test.ts`：HTML/CSS 允许项、限定选择器、危险内容过滤、单次图片源输出。
- `tests/e2e/platform-parity.spec.ts`：Chromium/WebKit 中真实输入带 CSS 的 HTML 卡片并检查计算样式。
- `tests/e2e/preview-source-scroll.spec.ts`：长图、换行段落、代码块、迟到布局变化、双向滚动、反复跨段与首尾边界。
