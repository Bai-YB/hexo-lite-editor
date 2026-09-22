<div align="center">
  <img src=".github/assets/app-icon.png" width="104" height="104" alt="Hexo Lite Editor 应用图标">
  <h1>Hexo Lite Editor</h1>
  <p><strong>选中一个 Hexo 目录，就能安静地写完、整理图片、同步项目，并把站点发出去。</strong></p>
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

## 这是什么

Hexo Lite Editor 是一个本地优先的 Hexo 桌面客户端。打开包含 `_config.yml` 的博客目录，就能在同一个窗口里写文章、管理图片、预览主题效果并把站点发出去，不必为了改一句话去开终端。

它不接管你的项目，也不改变 Hexo 的构建方式。文件还是原来的文件，发布仍然是保存 → `hexo clean` → `generate` → `deploy`，只是这条链路被放进了窗口里：源文件、主题和配置始终留在你自己的目录中，随时可以回到命令行。

## 功能一览

| 方向 | 能力 |
| --- | --- |
| **写作** | Markdown 与常用 HTML 编辑，文章和草稿可互转；改名时同步本地文件名、标题与图床目录；右键可在文件夹中定位，或二次确认后移入回收站。 |
| **即时预览** | 按顶层内容块拆分渲染，只重绘变化的区块；编辑器滚动单向驱动预览跟随，图片、代码块和长段落都按源码位置对齐。文章内 `<style>`、class 与 CSS 变量经限定后可用于预览，脚本、iframe、表单与危险 URL 继续被过滤。 |
| **浏览器预览** | 用项目真实主题在系统浏览器中打开，后台 Hexo Server 可随时关闭。 |
| **全部文件** | 目录树浏览并编辑友链、YAML、JSON 和主题配置，保存前比对磁盘改动；二进制与大于 2 MB 的文件保持只读，可直接在系统文件管理器中定位。 |
| **图片** | 管理 `source/` 下的本地图片，或接入 Cloudflare-ImgBed；粘贴与拖放先写入本地缓存并立即插入 Markdown，上传在后台完成，中断后仍可续传。 |
| **发布** | 固定执行保存 → `hexo clean` → `generate` → `deploy`；生成报错或文章头部非法时中止后续部署，避免把缺文章的旧站点推上线。 |
| **内容同步** | 用独立 GitHub 内容分支或标准 WebDAV 同步文章、`source/`、主题与配置；云端前进时可明确选择覆盖方向，同文件冲突逐项处理，云端覆盖本地前先创建备份。 |
| **更新与主题** | 每日静默检查更新，后台下载并校验签名后再询问安装；浅色、深色与跟随系统三套配色，正文与次级文字对比度满足 WCAG AA。 |
| **插件与语言** | Plugin API v0.1 在独立 Web Worker 中运行，只授予清单里声明的权限；界面提供中英文并跟随系统语言。 |

## 界面

### 写作与即时预览

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset=".github/assets/editor-dark.png">
    <img src=".github/assets/editor-light.png" alt="Hexo Lite Editor 编辑器：文章列表、Markdown 编辑器与即时预览">
  </picture>
</p>

### 图片

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset=".github/assets/image-bed-dark.png">
    <img src=".github/assets/image-bed-light.png" alt="Hexo Lite Editor 图床页：按目录管理本地图片与 Cloudflare-ImgBed 资源">
  </picture>
</p>

### 内容同步

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset=".github/assets/content-sync-dark.png">
    <img src=".github/assets/content-sync-light.png" alt="Hexo Lite Editor 内容同步设置">
  </picture>
</p>

WebDAV 地址、目录与用户名在启用后仍然可以修改。新凭据必须先通过目录访问与随机读写探针，才会覆盖系统凭据库中的旧值。

## 下载

`v1.0.6.5.1` 提供 Windows 10/11 x64 与 macOS 通用版本。界面显示 **1.0.6.5.1**，运行时版本为 `1.0.6+5.1`，1.0.6.x 可以直接识别这次更新。

| 安装包 | 用途 |
| --- | --- |
| [Windows 安装版 (EXE)](https://github.com/Bai-YB/hexo-lite-editor/releases/download/v1.0.6.5.1/Hexo-Lite-Editor_1.0.6.5.1_windows-x64-setup.exe) | 推荐；按向导安装并创建快捷方式 |
| [Windows 便携版 (ZIP)](https://github.com/Bai-YB/hexo-lite-editor/releases/download/v1.0.6.5.1/Hexo-Lite-Editor_1.0.6.5.1_windows-x64-portable.zip) | 解压完整目录后直接运行 |
| [Windows MSI](https://github.com/Bai-YB/hexo-lite-editor/releases/download/v1.0.6.5.1/Hexo-Lite-Editor_1.0.6.5.1_windows-x64.msi) | 企业或批量部署 |
| [macOS DMG](https://github.com/Bai-YB/hexo-lite-editor/releases/download/v1.0.6.5.1/Hexo-Lite-Editor_1.0.6.5.1_macos-universal.dmg) | Intel 与 Apple Silicon 通用 |

Windows 安装包尚未使用商业代码签名证书，首次运行时 SmartScreen 可能提示“未知发布者”。校验值见 [SHA256SUMS.txt](https://github.com/Bai-YB/hexo-lite-editor/releases/download/v1.0.6.5.1/SHA256SUMS.txt) 与 [SHA256SUMS-macos.txt](https://github.com/Bai-YB/hexo-lite-editor/releases/download/v1.0.6.5.1/SHA256SUMS-macos.txt)。应用默认静默检查更新，由用户决定何时下载与安装；更新包在安装前经过 Tauri 签名校验。

## 快速开始

1. 选择**打开项目**，指向包含 `_config.yml` 的 Hexo 博客目录。
2. 在左侧文章列表中选择文章或草稿，直接在中间编辑。
3. 右侧即时预览随内容更新；需要真实主题效果时点**浏览器预览**。
4. `Ctrl + S` 保存，随后点**发布**；也可以在高级菜单里单独运行生成与部署。

常用快捷键：

| 快捷键 | 作用 |
| --- | --- |
| `Ctrl + 1` ~ `Ctrl + 6` | 编辑器、图床、设置、关于、插件、全部文件 |
| `Ctrl + O` | 打开项目 |
| `Ctrl + N` | 新建文章 |
| `Ctrl + S` | 保存当前文件 |
| `Ctrl + \` | 显示或隐藏即时预览 |
| `Ctrl + ,` | 打开设置 |
| `Ctrl + Shift + P` | 发布当前文章 |

项目文件采用显式保存：切换文件、离开工作区或退出应用时会提示处理未保存内容。首次配置同步会依次完成连接检查、启用与方向确认，之后保存的内容在 30 秒后自动上传，也可以手动选择立即上传。停止同步会等待当前网络请求结束，已完成的上传不会被撤回。

## 说明与边界

- 应用不捆绑 Node.js、Hexo 或博客依赖。本地编辑不需要 Node.js，预览与发布使用项目已有的运行环境。
- 即时预览会清理脚本、iframe、表单等不安全 HTML；代码块中的 HTML 保持源码显示。
- Token 与 WebDAV 密码保存在操作系统凭据库，不写入项目配置、日志或同步清单。
- 项目同步默认关闭，范围包含文章、草稿、`source/`、主题、脚手架与 Hexo/主题配置；排除 `.git`、`public`、`node_modules`、缓存、环境文件和常见凭据。
- 远程图片由 WebView 直接加载。遇到空响应或无法显示的内容时保留错误提示，不替换成默认封面，也不会在切换文章时闪回原图。

## 技术栈

| 层 | 选型 |
| --- | --- |
| 桌面外壳 | [Tauri 2](https://tauri.app/)（Rust），Windows 使用 WebView2，macOS 使用 WKWebView |
| 界面 | Svelte 5 + SvelteKit + TypeScript + Vite |
| 编辑器 | CodeMirror 6 |
| Markdown | markdown-it + DOMPurify |
| 测试 | Vitest、Testing Library（jsdom）、Playwright（Chromium / WebKit）、`cargo test` 与 `cargo clippy` |

## 文档

- [模块职责与调用链](docs/modules/README.md)
- [插件开发指南](docs/plugin-development.md) 与 [Plugin API 参考](docs/plugin-api-reference.md)；示例见 [`examples/plugins/imagebed-example`](examples/plugins/imagebed-example)
- [更新记录](CHANGELOG.md) 与 [版本归档](docs/archive)
- [1.0.6.5.1 发布验证记录](docs/validation-1.0.6.5.1.md)

<details>
<summary><strong>开发与构建</strong></summary>

需要 Node.js、pnpm、Rust，以及 Tauri 2 在对应平台上的构建依赖。

```bash
pnpm install
pnpm check
pnpm test
pnpm test:e2e
pnpm tauri dev
```

生产构建与 README 素材：

```bash
pnpm build
pnpm tauri build
pnpm readme:assets
pnpm readme:stars -- --repository Bai-YB/hexo-lite-editor
```

发布由 `.github/workflows` 中的 Windows 与 macOS 工作流完成：双平台构建、签名、哈希校验、生成更新清单，全部通过后再统一公开 Release。
</details>

## Star History

<img src=".github/assets/star-history.svg" alt="Bai-YB/hexo-lite-editor 的 GitHub Star 趋势">

[提交 Issue](https://github.com/Bai-YB/hexo-lite-editor/issues) · [查看 Releases](https://github.com/Bai-YB/hexo-lite-editor/releases) · [参与贡献](https://github.com/Bai-YB/hexo-lite-editor/pulls)

本项目使用 [MIT License](LICENSE) 开源。
