<div align="center">
  <img src=".github/assets/app-icon.png" width="96" height="96" alt="Hexo Lite Editor app icon">
  <h1>Hexo Lite Editor</h1>
  <p><strong>Writing, images, sync, and publishing for your local Hexo blog — in one quiet desktop window.</strong></p>
  <p>
    <a href="README.md">简体中文</a> ·
    <a href="README_EN.md">English</a>
  </p>
  <p>
    <a href="https://github.com/Bai-YB/hexo-lite-editor/releases/latest"><img alt="Latest release" src="https://img.shields.io/github/v/release/Bai-YB/hexo-lite-editor?display_name=tag&style=flat-square"></a>
    <a href="https://github.com/Bai-YB/hexo-lite-editor/stargazers"><img alt="Stars" src="https://img.shields.io/github/stars/Bai-YB/hexo-lite-editor?style=flat-square"></a>
    <a href="https://github.com/Bai-YB/hexo-lite-editor/releases"><img alt="Total downloads" src="https://img.shields.io/github/downloads/Bai-YB/hexo-lite-editor/total?style=flat-square"></a>
    <a href="https://github.com/Bai-YB/hexo-lite-editor/actions/workflows/ci.yml"><img alt="CI status" src="https://img.shields.io/github/actions/workflow/status/Bai-YB/hexo-lite-editor/ci.yml?style=flat-square&label=CI"></a>
    <a href="LICENSE"><img alt="License" src="https://img.shields.io/github/license/Bai-YB/hexo-lite-editor?style=flat-square"></a>
    <img alt="Supported platforms" src="https://img.shields.io/badge/platform-Windows%20%7C%20macOS-2f6f5e?style=flat-square">
  </p>
  <p>
    <a href="https://github.com/Bai-YB/hexo-lite-editor/releases/download/v1.0.6.5.2/Hexo-Lite-Editor_1.0.6.5.2_windows-x64-setup.exe"><strong>Download for Windows</strong></a>
    ・ <a href="https://github.com/Bai-YB/hexo-lite-editor/releases/download/v1.0.6.5.2/Hexo-Lite-Editor_1.0.6.5.2_windows-x64-portable.zip">Portable ZIP</a>
    ・ <a href="https://github.com/Bai-YB/hexo-lite-editor/releases/download/v1.0.6.5.3/Hexo-Lite-Editor_1.0.6.5.3_macos-universal.dmg">macOS</a>
    ・ <a href="https://github.com/Bai-YB/hexo-lite-editor/releases/tag/v1.0.6.5.3">v1.0.6.5.3</a>
  </p>
</div>

<p align="center">
  <img src=".github/assets/writing-workflow.gif" alt="Selecting a post, editing Markdown, updating the live preview, then hiding and restoring the preview">
</p>

## Features

- ✍️ **Writing**: Markdown and common HTML, post/draft conversion, renaming keeps the file name in step
- 👀 **Live preview**: repaints only changed blocks, scrolling driven by the editor alone
- 🖥️ **Browser preview**: opens the project's real theme; the background server stops on demand
- 🗂️ **All files**: edit friend links, YAML, JSON, and theme config with a disk check before saving
- 🖼️ **Images**: local `source/` or Cloudflare-ImgBed; pasted images land locally first, then upload
- 🚀 **Publishing**: always save → `clean` → `generate` → `deploy`, and stops on any error
- ☁️ **Sync**: GitHub content branch or WebDAV, conflicts resolved file by file, backup before overwrite
- 🎨 **Themes**: light, dark, or follow system, with body text at WCAG AA contrast
- 🧩 **Plugins**: Plugin API v0.1 in an isolated Worker, limited to declared permissions

It never takes over your project or changes how Hexo builds. Files stay where they are, and the command line is always one step away.

## Screenshots

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset=".github/assets/editor-dark.png">
    <img src=".github/assets/editor-light.png" alt="Hexo Lite Editor: post list, Markdown editor, and live preview">
  </picture>
</p>

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset=".github/assets/image-bed-dark.png">
    <img src=".github/assets/image-bed-light.png" width="49%" alt="Images: folder-based local images and Cloudflare-ImgBed assets">
  </picture>
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset=".github/assets/content-sync-dark.png">
    <img src=".github/assets/content-sync-light.png" width="49%" alt="Content sync: GitHub and WebDAV">
  </picture>
</p>

## Download

Windows builds are `v1.0.6.5.2` (Windows 10/11 x64). `v1.0.6.5.3` is a macOS-only fix (universal for Intel and Apple Silicon); Windows users stay on 1.0.6.5.2.

| Package | Use |
| --- | --- |
| [Windows setup (EXE)](https://github.com/Bai-YB/hexo-lite-editor/releases/download/v1.0.6.5.2/Hexo-Lite-Editor_1.0.6.5.2_windows-x64-setup.exe) | Recommended, wizard install |
| [Windows portable (ZIP)](https://github.com/Bai-YB/hexo-lite-editor/releases/download/v1.0.6.5.2/Hexo-Lite-Editor_1.0.6.5.2_windows-x64-portable.zip) | Extract and run |
| [Windows MSI](https://github.com/Bai-YB/hexo-lite-editor/releases/download/v1.0.6.5.2/Hexo-Lite-Editor_1.0.6.5.2_windows-x64.msi) | Managed or bulk deployment |
| [macOS DMG](https://github.com/Bai-YB/hexo-lite-editor/releases/download/v1.0.6.5.3/Hexo-Lite-Editor_1.0.6.5.3_macos-universal.dmg) | Universal for Intel and Apple Silicon; fixes content sync and Node.js detection |

## Getting started

1. Open a Hexo blog directory containing `_config.yml`
2. Pick a post or draft on the left, edit in the middle, and watch the live preview follow
3. Press `Ctrl + S` to save and **Publish**; generate or deploy can also run on their own

| Shortcut | Action |
| --- | --- |
| `Ctrl + 1` … `Ctrl + 6` | Editor, images, settings, about, plugins, all files |
| `Ctrl + O` / `Ctrl + N` | Open project / new post |
| `Ctrl + S` / `Ctrl + Shift + P` | Save / publish |
| `Ctrl + \` / `Ctrl + ,` | Show or hide live preview / open settings |

<details>
<summary><strong>Notes and limits</strong></summary>

- Node.js, Hexo, and blog dependencies are not bundled. Local editing works without Node.js; preview and publishing use the environment the project already has.
- The live preview strips unsafe HTML such as scripts, iframes, and forms. HTML inside code blocks stays visible as source.
- Tokens and WebDAV passwords live in the OS credential vault and are never written to project configuration, logs, or sync manifests.
- Project sync is off by default. It covers posts, drafts, `source/`, themes, scaffolds, and Hexo/theme configuration, and excludes `.git`, `public`, `node_modules`, caches, environment files, and common credentials.
- Remote images load directly through the WebView. Empty or unrenderable responses keep an error state instead of falling back to a default cover, and they do not flash the previous image while switching posts.
- Windows packages do not carry a commercial code-signing certificate, so SmartScreen may report an unknown publisher. Checksums are in [SHA256SUMS.txt](https://github.com/Bai-YB/hexo-lite-editor/releases/download/v1.0.6.5.2/SHA256SUMS.txt), and updates are signature-verified before install.
</details>

<details>
<summary><strong>Stack and development</strong></summary>

| Layer | Choice |
| --- | --- |
| Desktop shell | [Tauri 2](https://tauri.app/) (Rust) with WebView2 on Windows and WKWebView on macOS |
| Interface | Svelte 5 + SvelteKit + TypeScript + Vite |
| Editor | CodeMirror 6 |
| Markdown | markdown-it + DOMPurify |
| Tests | Vitest, Testing Library, Playwright (Chromium / WebKit), `cargo test` / `cargo clippy` |

You need Node.js, pnpm, Rust, and the Tauri 2 platform prerequisites.

```bash
pnpm install
pnpm check && pnpm test && pnpm test:e2e
pnpm tauri dev      # develop
pnpm tauri build    # package
```

Releases are produced by the Windows and macOS workflows in `.github/workflows`: both platforms build, sign, and hash their packages, a shared update manifest is generated, and the release is published only after every check passes.
</details>

Docs: [module responsibilities](docs/modules/README.md) · [plugin development](docs/plugin-development.md) · [Plugin API](docs/plugin-api-reference.md) · [changelog](CHANGELOG.md) · [1.0.6.5.3 validation](docs/validation-1.0.6.5.3.md)

## Star History

<img src=".github/assets/star-history.svg" alt="GitHub star history for Bai-YB/hexo-lite-editor">

[Open an issue](https://github.com/Bai-YB/hexo-lite-editor/issues) · [Browse releases](https://github.com/Bai-YB/hexo-lite-editor/releases) · [Contribute](https://github.com/Bai-YB/hexo-lite-editor/pulls) · [MIT License](LICENSE)
