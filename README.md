# Hexo Lite Editor

轻量级桌面端 Hexo 博客编辑器，基于 Tauri + Svelte + TypeScript。

## 当前版本

Version: 1.0.1

## 主界面

应用保持第一版外观：顶部单行工具栏、左侧文章列表、中间 Markdown 编辑器、右侧预览和日志、底部状态栏。

## 核心功能

- 打开并校验已有 Hexo 项目。
- 扫描 `source/_posts` 下的 `.md` / `.markdown` 文章。
- 新建、编辑、自动保存和手动保存 Markdown 文章。
- 使用 CodeMirror 6 编辑 Markdown，支持快捷键保存、搜索和新建。
- 右侧实时 Markdown 预览。
- 本地图片复制到 `source/images` 并插入 Markdown 图片语法。
- 启动 / 停止 `npx hexo server`，并打开 `http://localhost:4000`。
- 执行 `npx hexo generate` / `npx hexo deploy`。
- 查看基础 `git status --short` 输出。

## 1.0.1 设置抽屉

- 顶部设置按钮和 `Ctrl + ,` 打开右侧设置抽屉，`Esc` 关闭。
- 抽屉分类包括：常规、外观、编辑器、Hexo 配置、图床、发布、更新、关于。
- 支持 light / dark / system 主题、紧凑模式、字体缩放。
- 支持编辑器字号、行高、行号、自动换行、高亮当前行和 Tab 宽度。
- 支持读取、编辑、保存、备份、恢复当前项目 `_config.yml`，并可用系统默认编辑器打开。
- 支持本地图床测试上传，并显示 URL 和 Markdown。
- 支持保存发布命令配置。
- 支持 GitHub Releases 或自定义 JSON 手动检查更新。

## 开发运行

```bash
pnpm install
pnpm tauri dev
```

## 构建

```bash
pnpm tauri build
```

Windows 构建产物位于：

```text
src-tauri/target/release/bundle/msi/Hexo Lite Editor_1.0.1_x64_en-US.msi
src-tauri/target/release/bundle/nsis/Hexo Lite Editor_1.0.1_x64-setup.exe
```

## 注意

应用不内置 Node.js、Hexo 或 Hexo 项目依赖。使用预览、生成、发布功能前，请确保目标 Hexo 项目可以在终端中正常执行 `npx hexo server`、`npx hexo generate` 和 `npx hexo deploy`。
