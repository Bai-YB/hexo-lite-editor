<div align="center">
  <img src=".github/assets/app-icon.png" width="96" height="96" alt="Hexo Lite Editor 应用图标">
  <h1>Hexo Lite Editor</h1>
  <p><strong>把本地 Hexo 博客的写作、图片、同步和发布，收进一个安静的桌面窗口。</strong></p>
  <p>
    <a href="README.md">简体中文</a> ·
    <a href="README_EN.md">English</a>
  </p>
  <p>
    <a href="https://github.com/Bai-YB/hexo-lite-editor/releases/latest"><img alt="最新版本" src="https://img.shields.io/github/v/release/Bai-YB/hexo-lite-editor?display_name=tag&style=flat-square"></a>
    <a href="https://github.com/Bai-YB/hexo-lite-editor/stargazers"><img alt="Stars" src="https://img.shields.io/github/stars/Bai-YB/hexo-lite-editor?style=flat-square"></a>
    <a href="https://github.com/Bai-YB/hexo-lite-editor/releases"><img alt="总下载量" src="https://img.shields.io/github/downloads/Bai-YB/hexo-lite-editor/total?style=flat-square"></a>
    <a href="https://github.com/Bai-YB/hexo-lite-editor/actions/workflows/ci.yml"><img alt="CI 状态" src="https://img.shields.io/github/actions/workflow/status/Bai-YB/hexo-lite-editor/ci.yml?style=flat-square&label=CI"></a>
    <a href="LICENSE"><img alt="开源协议" src="https://img.shields.io/github/license/Bai-YB/hexo-lite-editor?style=flat-square"></a>
    <img alt="支持平台" src="https://img.shields.io/badge/platform-Windows%20%7C%20macOS-2f6f5e?style=flat-square">
  </p>
  <p>
    <a href="https://github.com/Bai-YB/hexo-lite-editor/releases/download/v1.0.6.5.1/Hexo-Lite-Editor_1.0.6.5.1_windows-x64-setup.exe"><strong>下载 Windows 安装版</strong></a>
    ・ <a href="https://github.com/Bai-YB/hexo-lite-editor/releases/download/v1.0.6.5.1/Hexo-Lite-Editor_1.0.6.5.1_windows-x64-portable.zip">便携版</a>
    ・ <a href="https://github.com/Bai-YB/hexo-lite-editor/releases/download/v1.0.6.5.1/Hexo-Lite-Editor_1.0.6.5.1_macos-universal.dmg">macOS</a>
    ・ <a href="https://github.com/Bai-YB/hexo-lite-editor/releases/tag/v1.0.6.5.1">v1.0.6.5.1</a>
  </p>
</div>

<p align="center">
  <img src=".github/assets/writing-workflow.gif" alt="选择文章、编辑 Markdown、即时预览跟随更新，再隐藏并恢复预览">
</p>

## 功能

- ✍️ **写作**：Markdown 与常用 HTML，文章 / 草稿互转，改名同步本地文件名
- 👀 **即时预览**：只重绘变化的区块，滚动由编辑器单向驱动
- 🖥️ **浏览器预览**：用项目真实主题打开，后台服务随时可停
- 🗂️ **全部文件**：目录树编辑友链、YAML、JSON、主题配置，保存前比对磁盘
- 🖼️ **图片**：本地 `source/` 或 Cloudflare-ImgBed，粘贴拖放先落本地再后台上传
- 🚀 **发布**：固定执行保存 → `clean` → `generate` → `deploy`，出错即停
- ☁️ **同步**：GitHub 内容分支或 WebDAV，冲突逐项选择，覆盖前先备份
- 🎨 **主题**：浅色 / 深色 / 跟随系统，正文对比度满足 WCAG AA
- 🧩 **插件**：Plugin API v0.1，独立 Worker 运行，只给清单声明的权限

应用不接管项目，也不改变 Hexo 的构建方式：文件还是原来的文件，随时可以回到命令行。

## 界面

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset=".github/assets/editor-dark.png">
    <img src=".github/assets/editor-light.png" alt="Hexo Lite Editor：文章列表、Markdown 编辑器和即时预览">
  </picture>
</p>

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset=".github/assets/image-bed-dark.png">
    <img src=".github/assets/image-bed-light.png" width="49%" alt="图床：按目录管理本地图片与 Cloudflare-ImgBed 资源">
  </picture>
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset=".github/assets/content-sync-dark.png">
    <img src=".github/assets/content-sync-light.png" width="49%" alt="内容同步：GitHub 与 WebDAV">
  </picture>
</p>

## 下载

`v1.0.6.5.1` 支持 Windows 10/11 x64 与 macOS 通用版。

| 安装包 | 用途 |
| --- | --- |
| [Windows 安装版 (EXE)](https://github.com/Bai-YB/hexo-lite-editor/releases/download/v1.0.6.5.1/Hexo-Lite-Editor_1.0.6.5.1_windows-x64-setup.exe) | 推荐，按向导安装 |
| [Windows 便携版 (ZIP)](https://github.com/Bai-YB/hexo-lite-editor/releases/download/v1.0.6.5.1/Hexo-Lite-Editor_1.0.6.5.1_windows-x64-portable.zip) | 解压后直接运行 |
| [Windows MSI](https://github.com/Bai-YB/hexo-lite-editor/releases/download/v1.0.6.5.1/Hexo-Lite-Editor_1.0.6.5.1_windows-x64.msi) | 企业或批量部署 |
| [macOS DMG](https://github.com/Bai-YB/hexo-lite-editor/releases/download/v1.0.6.5.1/Hexo-Lite-Editor_1.0.6.5.1_macos-universal.dmg) | Intel 与 Apple Silicon 通用 |

## 快速开始

1. 打开包含 `_config.yml` 的 Hexo 博客目录
2. 左侧选文章或草稿，中间直接编辑，右侧即时预览跟着走
3. `Ctrl + S` 保存，点**发布**即可；也可以只运行生成或部署

| 快捷键 | 作用 |
| --- | --- |
| `Ctrl + 1` ~ `Ctrl + 6` | 编辑器、图床、设置、关于、插件、全部文件 |
| `Ctrl + O` / `Ctrl + N` | 打开项目 / 新建文章 |
| `Ctrl + S` / `Ctrl + Shift + P` | 保存 / 发布 |
| `Ctrl + \` / `Ctrl + ,` | 显示或隐藏即时预览 / 打开设置 |

<details>
<summary><strong>说明与边界</strong></summary>

- 不捆绑 Node.js、Hexo 或博客依赖。本地编辑不需要 Node.js，预览与发布使用项目已有的运行环境。
- 即时预览会清理脚本、iframe、表单等不安全 HTML；代码块中的 HTML 保持源码显示。
- Token 与 WebDAV 密码保存在操作系统凭据库，不写入项目配置、日志或同步清单。
- 项目同步默认关闭，范围包含文章、草稿、`source/`、主题、脚手架与 Hexo/主题配置；排除 `.git`、`public`、`node_modules`、缓存、环境文件和常见凭据。
- 远程图片由 WebView 直接加载；空响应或无法显示时保留错误提示，不替换成默认封面，也不在切换文章时闪回原图。
- Windows 安装包没有商业代码签名证书，首次运行可能提示"未知发布者"；校验值见 [SHA256SUMS.txt](https://github.com/Bai-YB/hexo-lite-editor/releases/download/v1.0.6.5.1/SHA256SUMS.txt)，更新包在安装前经过签名验证。
</details>

<details>
<summary><strong>技术栈与开发</strong></summary>

| 层 | 选型 |
| --- | --- |
| 桌面外壳 | [Tauri 2](https://tauri.app/)（Rust），Windows 用 WebView2，macOS 用 WKWebView |
| 界面 | Svelte 5 + SvelteKit + TypeScript + Vite |
| 编辑器 | CodeMirror 6 |
| Markdown | markdown-it + DOMPurify |
| 测试 | Vitest、Testing Library、Playwright（Chromium / WebKit）、`cargo test` / `cargo clippy` |

需要 Node.js、pnpm、Rust 和 Tauri 2 的平台构建依赖。

```bash
pnpm install
pnpm check && pnpm test && pnpm test:e2e
pnpm tauri dev      # 开发
pnpm tauri build    # 打包
```

发布由 `.github/workflows` 中的 Windows 与 macOS 工作流完成：双平台构建、签名、哈希校验、生成更新清单，全部通过后统一公开 Release。
</details>

文档：[模块职责与调用链](docs/modules/README.md) · [插件开发](docs/plugin-development.md) · [Plugin API](docs/plugin-api-reference.md) · [更新记录](CHANGELOG.md) · [1.0.6.5.1 验证记录](docs/validation-1.0.6.5.1.md)

## Star History

<img src=".github/assets/star-history.svg" alt="Bai-YB/hexo-lite-editor 的 GitHub Star 趋势">

[提交 Issue](https://github.com/Bai-YB/hexo-lite-editor/issues) · [查看 Releases](https://github.com/Bai-YB/hexo-lite-editor/releases) · [参与贡献](https://github.com/Bai-YB/hexo-lite-editor/pulls) · [MIT License](LICENSE)
