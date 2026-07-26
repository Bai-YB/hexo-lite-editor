<div align="center">
  <img src=".github/assets/app-icon.png" width="96" height="96" alt="Hexo Lite Editor icon">
  <h1>Hexo Lite Editor</h1>
  <p>A desktop editor for local Hexo blogs.</p>
  <p><a href="README.md">简体中文</a> · <a href="README_EN.md">English</a></p>
  <p>
    <a href="https://github.com/Bai-YB/hexo-lite-editor/releases/latest"><img alt="Release" src="https://img.shields.io/github/v/release/Bai-YB/hexo-lite-editor?display_name=tag&style=flat-square"></a>
    <a href="https://github.com/Bai-YB/hexo-lite-editor/stargazers"><img alt="GitHub Stars" src="https://img.shields.io/github/stars/Bai-YB/hexo-lite-editor?style=flat-square"></a>
    <a href="https://github.com/Bai-YB/hexo-lite-editor/releases"><img alt="Downloads" src="https://img.shields.io/github/downloads/Bai-YB/hexo-lite-editor/total?style=flat-square"></a>
    <a href="LICENSE"><img alt="License" src="https://img.shields.io/github/license/Bai-YB/hexo-lite-editor?style=flat-square"></a>
  </p>
  <p>
    <a href="https://github.com/Bai-YB/hexo-lite-editor/releases/download/v1.0.5/Hexo-Lite-Editor_1.0.5_windows-x64-setup.exe"><strong>Download setup</strong></a>
    · <a href="https://github.com/Bai-YB/hexo-lite-editor/releases/download/v1.0.5/Hexo-Lite-Editor_1.0.5_windows-x64-portable.zip">Portable ZIP</a>
    · <a href="https://github.com/Bai-YB/hexo-lite-editor/releases/download/v1.0.5/Hexo-Lite-Editor_1.0.5_windows-x64.msi">MSI</a>
    · <a href="https://github.com/Bai-YB/hexo-lite-editor/releases/tag/v1.0.5">v1.0.5 Release</a>
  </p>
</div>

<p align="center">
  <img src=".github/assets/writing-workflow.gif" alt="Selecting a post, editing Markdown, updating preview, and hiding and restoring preview">
</p>

## Features

- Edit posts and drafts with Markdown, common HTML, and live preview.
- Hide the preview to give the editor all space outside the post list.
- Manage images under `source/` or connect Cloudflare-ImgBed; paste, drag, and import are supported.
- Run Hexo browser preview, clean, generate, and deploy with the blog's existing environment.
- Sync posts and images through an isolated GitHub content branch or a standard WebDAV service.
- Use light, dark, or system theme.

## Screenshots

### Images

<picture>
  <source media="(prefers-color-scheme: dark)" srcset=".github/assets/image-bed-dark.png">
  <img src=".github/assets/image-bed-light.png" alt="Hexo Lite Editor image workspace">
</picture>

### Content sync

<picture>
  <source media="(prefers-color-scheme: dark)" srcset=".github/assets/content-sync-dark.png">
  <img src=".github/assets/content-sync-light.png" alt="Hexo Lite Editor WebDAV content sync">
</picture>

The WebDAV endpoint, directory, and username remain editable after sync is enabled. New credentials are saved only after directory access and a read/write test succeed.

## Install

`v1.0.5` is available for Windows 10/11 x64.

| Package | Use |
| --- | --- |
| [Setup EXE](https://github.com/Bai-YB/hexo-lite-editor/releases/download/v1.0.5/Hexo-Lite-Editor_1.0.5_windows-x64-setup.exe) | Recommended; install with the setup wizard |
| [Portable ZIP](https://github.com/Bai-YB/hexo-lite-editor/releases/download/v1.0.5/Hexo-Lite-Editor_1.0.5_windows-x64-portable.zip) | Extract and run |
| [MSI](https://github.com/Bai-YB/hexo-lite-editor/releases/download/v1.0.5/Hexo-Lite-Editor_1.0.5_windows-x64.msi) | Managed or bulk deployment |

The packages do not have a commercial code-signing certificate, so SmartScreen may show “Unknown publisher.” Checksums are in [SHA256SUMS.txt](https://github.com/Bai-YB/hexo-lite-editor/releases/download/v1.0.5/SHA256SUMS.txt). The current Release has no macOS installer; Mac users can run the repository's [macOS build workflow](.github/workflows/release-macos.yml).

## Usage

1. Open a Hexo blog directory containing `_config.yml`.
2. Select a post or draft on the left and edit it.
3. Check the live preview, or open browser preview when you need the actual theme.
4. Save and choose **Publish**, or run generate and deploy separately from the advanced menu.

## Notes

- Node.js, Hexo, and blog dependencies are not bundled. Local editing works without Node.js; preview and publishing use the project's existing environment.
- Live preview removes unsafe HTML such as scripts, iframes, and forms. HTML in code fences remains source code.
- Tokens and WebDAV passwords stay in the OS credential vault and are not written to project config, logs, or sync manifests.
- Content sync is off by default and includes only posts, per-post assets, and the configured image directory. A local backup is created before a pull overwrites files.
- Remote images load directly in WebView. Empty or unrenderable responses keep an error state; they are not replaced by a default cover and do not flash the original image while switching posts.

<details>
<summary><strong>Development and builds</strong></summary>

You need Node.js, pnpm, Rust, and the platform prerequisites for Tauri 2.

```bash
pnpm install
pnpm check
pnpm test
pnpm test:e2e
pnpm tauri dev
```

Production builds and README assets:

```bash
pnpm build
pnpm tauri build
pnpm readme:assets
pnpm readme:stars -- --repository Bai-YB/hexo-lite-editor
```
</details>

## Star History

<img src=".github/assets/star-history.svg" alt="GitHub Star History for Bai-YB/hexo-lite-editor">

[Open an Issue](https://github.com/Bai-YB/hexo-lite-editor/issues) · [Browse Releases](https://github.com/Bai-YB/hexo-lite-editor/releases) · [Contribute](https://github.com/Bai-YB/hexo-lite-editor/pulls)

Released under the [MIT License](LICENSE).
