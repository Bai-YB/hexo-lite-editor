<div align="center">
  <img src=".github/assets/app-icon.png" width="96" height="96" alt="Hexo Lite Editor 图标">
  <h1>Hexo Lite Editor</h1>
  <p>用于管理本地 Hexo 博客的桌面编辑器。</p>
  <p><a href="README.md">简体中文</a> · <a href="README_EN.md">English</a></p>
  <p>
    <a href="https://github.com/Bai-YB/hexo-lite-editor/releases/latest"><img alt="Release" src="https://img.shields.io/github/v/release/Bai-YB/hexo-lite-editor?display_name=tag&style=flat-square"></a>
    <a href="https://github.com/Bai-YB/hexo-lite-editor/stargazers"><img alt="GitHub Stars" src="https://img.shields.io/github/stars/Bai-YB/hexo-lite-editor?style=flat-square"></a>
    <a href="https://github.com/Bai-YB/hexo-lite-editor/releases"><img alt="Downloads" src="https://img.shields.io/github/downloads/Bai-YB/hexo-lite-editor/total?style=flat-square"></a>
    <a href="LICENSE"><img alt="License" src="https://img.shields.io/github/license/Bai-YB/hexo-lite-editor?style=flat-square"></a>
  </p>
  <p>
    <a href="https://github.com/Bai-YB/hexo-lite-editor/releases/download/v1.0.5/Hexo-Lite-Editor_1.0.5_windows-x64-setup.exe"><strong>下载安装版</strong></a>
    · <a href="https://github.com/Bai-YB/hexo-lite-editor/releases/download/v1.0.5/Hexo-Lite-Editor_1.0.5_windows-x64-portable.zip">便携版</a>
    · <a href="https://github.com/Bai-YB/hexo-lite-editor/releases/download/v1.0.5/Hexo-Lite-Editor_1.0.5_windows-x64.msi">MSI</a>
    · <a href="https://github.com/Bai-YB/hexo-lite-editor/releases/tag/v1.0.5">v1.0.5 Release</a>
  </p>
</div>

<p align="center">
  <img src=".github/assets/writing-workflow.gif" alt="选择文章、编辑 Markdown、更新预览并隐藏和恢复预览">
</p>

## 功能

- 编辑文章和草稿，支持 Markdown、常用 HTML 与即时预览。
- 隐藏预览后，编辑器自动使用文章列表之外的全部空间。
- 管理 `source/` 下的图片，也可连接 Cloudflare-ImgBed；支持粘贴、拖放和导入。
- 调用博客现有环境运行 Hexo 浏览器预览、clean、generate 和 deploy。
- 通过独立 GitHub 内容分支或标准 WebDAV 服务同步文章和图片。
- 支持浅色、深色和跟随系统主题。

## 界面

### 图片管理

<picture>
  <source media="(prefers-color-scheme: dark)" srcset=".github/assets/image-bed-dark.png">
  <img src=".github/assets/image-bed-light.png" alt="Hexo Lite Editor 图片管理界面">
</picture>

### 内容同步

<picture>
  <source media="(prefers-color-scheme: dark)" srcset=".github/assets/content-sync-dark.png">
  <img src=".github/assets/content-sync-light.png" alt="Hexo Lite Editor WebDAV 内容同步界面">
</picture>

WebDAV 地址、目录和用户名在启用后仍可修改。新凭据通过目录访问和读写测试后才会保存。

## 安装

`v1.0.5` 提供 Windows 10/11 x64 版本。

| 安装包 | 用途 |
| --- | --- |
| [Setup EXE](https://github.com/Bai-YB/hexo-lite-editor/releases/download/v1.0.5/Hexo-Lite-Editor_1.0.5_windows-x64-setup.exe) | 推荐；按向导安装 |
| [Portable ZIP](https://github.com/Bai-YB/hexo-lite-editor/releases/download/v1.0.5/Hexo-Lite-Editor_1.0.5_windows-x64-portable.zip) | 解压后直接运行 |
| [MSI](https://github.com/Bai-YB/hexo-lite-editor/releases/download/v1.0.5/Hexo-Lite-Editor_1.0.5_windows-x64.msi) | 企业或批量部署 |

安装包尚未使用商业代码签名证书，SmartScreen 可能显示“未知发布者”。校验值见 [SHA256SUMS.txt](https://github.com/Bai-YB/hexo-lite-editor/releases/download/v1.0.5/SHA256SUMS.txt)。当前 Release 没有 macOS 安装包，Mac 用户可使用仓库中的 [macOS 构建工作流](.github/workflows/release-macos.yml)。

## 使用

1. 打开包含 `_config.yml` 的 Hexo 博客目录。
2. 从左侧选择文章或草稿，在编辑器中修改内容。
3. 使用即时预览检查正文，需要主题效果时打开浏览器预览。
4. 保存后点击“发布”，或在高级菜单中单独运行生成与部署。

## 说明

- 应用不捆绑 Node.js、Hexo 或博客依赖。本地编辑不需要 Node.js，预览和发布使用项目已有环境。
- 即时预览会清理脚本、iframe、表单等不安全 HTML；代码块中的 HTML 保持源码显示。
- Token 和 WebDAV 密码保存在操作系统凭据库，不写入项目配置、日志或同步清单。
- 内容同步默认关闭，只处理文章、文章资源目录和指定图片目录；拉取覆盖前会创建本地备份。
- 远程图片直接由 WebView 加载。空响应或无法显示时保留错误提示，不替换成默认封面，也不会在切换文章时闪回原图。

<details>
<summary><strong>开发与构建</strong></summary>

需要 Node.js、pnpm、Rust 和 Tauri 2 对应平台的构建依赖。

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
</details>

## Star History

<img src=".github/assets/star-history.svg" alt="Bai-YB/hexo-lite-editor 的 GitHub Star History">

[提交 Issue](https://github.com/Bai-YB/hexo-lite-editor/issues) · [查看 Releases](https://github.com/Bai-YB/hexo-lite-editor/releases) · [参与贡献](https://github.com/Bai-YB/hexo-lite-editor/pulls)

本项目使用 [MIT License](LICENSE)。
