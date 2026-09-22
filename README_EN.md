<div align="center">
  <img src=".github/assets/app-icon.png" width="104" height="104" alt="Hexo Lite Editor app icon">
  <h1>Hexo Lite Editor</h1>
  <p><strong>Point it at a Hexo folder and write, manage images, sync, and publish from one quiet desktop workspace.</strong></p>
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
    <a href="https://github.com/Bai-YB/hexo-lite-editor/releases/download/v1.0.6.5.1/Hexo-Lite-Editor_1.0.6.5.1_windows-x64-setup.exe"><strong>Download for Windows</strong></a>
    ・ <a href="https://github.com/Bai-YB/hexo-lite-editor/releases/download/v1.0.6.5.1/Hexo-Lite-Editor_1.0.6.5.1_windows-x64-portable.zip">Portable ZIP</a>
    ・ <a href="https://github.com/Bai-YB/hexo-lite-editor/releases/download/v1.0.6.5.1/Hexo-Lite-Editor_1.0.6.5.1_macos-universal.dmg">macOS</a>
    ・ <a href="https://github.com/Bai-YB/hexo-lite-editor/releases/tag/v1.0.6.5.1">v1.0.6.5.1</a>
  </p>
</div>

<p align="center">
  <img src=".github/assets/writing-workflow.gif" alt="Selecting a post, editing Markdown, updating the live preview, then hiding and restoring the preview">
</p>

## What it is

Hexo Lite Editor is a local-first desktop client for Hexo. Open a blog directory that contains `_config.yml`, and you can draft posts, manage images, preview the real theme, and publish the site from a single window instead of reaching for a terminal to change one sentence.

It does not take over your project or change how Hexo builds. The files stay where they are and publishing is still save → `hexo clean` → `generate` → `deploy`; the editor simply puts that pipeline in a window. Sources, themes, and configuration remain in your own directory, and you can go back to the command line whenever you want.

## Features

| Area | What it does |
| --- | --- |
| **Writing** | Edit Markdown and common HTML, convert between posts and drafts, and keep the local file name, title, and image folder in step when you rename. The context menu locates a post in the file manager or moves it to the recycle bin after confirmation. |
| **Live preview** | Renders by top-level content block and repaints only what changed; scrolling the editor drives the preview so images, code blocks, and long paragraphs stay aligned with their source. In-article `<style>`, classes, and CSS variables are scoped into the preview, while scripts, iframes, forms, and dangerous URLs stay filtered. |
| **Browser preview** | Opens the project's real theme in your system browser, with a background Hexo Server you can stop at any time. |
| **All files** | Browse the project tree and edit friend links, YAML, JSON, and theme configuration with a disk-change check before saving. Binary files and files over 2 MB stay read-only, and any file can be revealed in the system file manager. |
| **Images** | Manage local images under `source/` or connect Cloudflare-ImgBed. Pasted and dropped images land in the local cache and are inserted into Markdown immediately, while uploading continues in the background and resumes after an interruption. |
| **Publishing** | Always runs save → `hexo clean` → `generate` → `deploy`. Invalid front matter or a failing generation step stops the deploy, so a site with missing posts is never pushed live. |
| **Content sync** | Sync posts, `source/`, themes, and configuration through an isolated GitHub content branch or a standard WebDAV service. When the cloud moves ahead you choose the direction, conflicts are resolved file by file, and a local backup is created before the cloud overwrites local files. |
| **Updates and themes** | Checks quietly once a day, downloads in the background, verifies the signature, then asks before installing. Light, dark, and follow-system palettes keep body and secondary text at WCAG AA contrast. |
| **Plugins and languages** | Plugin API v0.1 runs in a dedicated Web Worker and receives only the permissions declared in its manifest. The interface ships in Chinese and English and follows the system language. |

## Screenshots

### Writing and live preview

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset=".github/assets/editor-dark.png">
    <img src=".github/assets/editor-light.png" alt="Hexo Lite Editor workspace: post list, Markdown editor, and live preview">
  </picture>
</p>

### Images

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset=".github/assets/image-bed-dark.png">
    <img src=".github/assets/image-bed-light.png" alt="Hexo Lite Editor image page: folder-based local images and Cloudflare-ImgBed assets">
  </picture>
</p>

### Content sync

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset=".github/assets/content-sync-dark.png">
    <img src=".github/assets/content-sync-light.png" alt="Hexo Lite Editor content sync settings">
  </picture>
</p>

The WebDAV endpoint, remote directory, and username stay editable after sync is enabled. New credentials must pass a directory read plus a random read/write probe before they replace the values in the OS credential vault.

## Download

`v1.0.6.5.1` ships for Windows 10/11 x64 and universal macOS. The interface reports **1.0.6.5.1** and the runtime version is `1.0.6+5.1`, so existing 1.0.6.x builds recognize this update.

| Package | Use |
| --- | --- |
| [Windows setup (EXE)](https://github.com/Bai-YB/hexo-lite-editor/releases/download/v1.0.6.5.1/Hexo-Lite-Editor_1.0.6.5.1_windows-x64-setup.exe) | Recommended; wizard install with shortcuts |
| [Windows portable (ZIP)](https://github.com/Bai-YB/hexo-lite-editor/releases/download/v1.0.6.5.1/Hexo-Lite-Editor_1.0.6.5.1_windows-x64-portable.zip) | Extract the full folder and run |
| [Windows MSI](https://github.com/Bai-YB/hexo-lite-editor/releases/download/v1.0.6.5.1/Hexo-Lite-Editor_1.0.6.5.1_windows-x64.msi) | Managed or bulk deployment |
| [macOS DMG](https://github.com/Bai-YB/hexo-lite-editor/releases/download/v1.0.6.5.1/Hexo-Lite-Editor_1.0.6.5.1_macos-universal.dmg) | Universal for Intel and Apple Silicon |

The Windows packages do not carry a commercial code-signing certificate, so SmartScreen may report an unknown publisher on first launch. Checksums are published in [SHA256SUMS.txt](https://github.com/Bai-YB/hexo-lite-editor/releases/download/v1.0.6.5.1/SHA256SUMS.txt) and [SHA256SUMS-macos.txt](https://github.com/Bai-YB/hexo-lite-editor/releases/download/v1.0.6.5.1/SHA256SUMS-macos.txt). Updates are checked quietly and installed only when you ask; every package is verified against a Tauri signature first.

## Getting started

1. Choose **Open project** and point it at a Hexo blog directory containing `_config.yml`.
2. Pick a post or draft in the list on the left and edit it in the middle.
3. The live preview follows along, and **Browser preview** shows the real theme when you need it.
4. Press `Ctrl + S` to save, then **Publish**; generate and deploy can also be run separately from the advanced menu.

Common shortcuts:

| Shortcut | Action |
| --- | --- |
| `Ctrl + 1` … `Ctrl + 6` | Editor, images, settings, about, plugins, all files |
| `Ctrl + O` | Open project |
| `Ctrl + N` | New post |
| `Ctrl + S` | Save the current file |
| `Ctrl + \` | Show or hide the live preview |
| `Ctrl + ,` | Open settings |
| `Ctrl + Shift + P` | Publish the current post |

Project files use explicit saving: switching files, leaving the workspace, or quitting prompts you about unsaved work. The first sync walks through connection check, enablement, and direction; later saves upload automatically after 30 seconds, or you can choose **Upload changes now**. Stopping waits for the current network request, and completed uploads are never rolled back.

## Notes and limits

- Node.js, Hexo, and blog dependencies are not bundled. Local editing works without Node.js; preview and publishing use the environment the project already has.
- The live preview strips unsafe HTML such as scripts, iframes, and forms. HTML inside code blocks stays visible as source.
- Tokens and WebDAV passwords live in the OS credential vault and are never written to project configuration, logs, or sync manifests.
- Project sync is off by default. It covers posts, drafts, `source/`, themes, scaffolds, and Hexo/theme configuration, and excludes `.git`, `public`, `node_modules`, caches, environment files, and common credentials.
- Remote images load directly through the WebView. Empty or unrenderable responses keep an error state instead of falling back to a default cover, and they do not flash the previous image while switching posts.

## Built with

| Layer | Choice |
| --- | --- |
| Desktop shell | [Tauri 2](https://tauri.app/) (Rust) with WebView2 on Windows and WKWebView on macOS |
| Interface | Svelte 5 + SvelteKit + TypeScript + Vite |
| Editor | CodeMirror 6 |
| Markdown | markdown-it + DOMPurify |
| Tests | Vitest, Testing Library (jsdom), Playwright (Chromium / WebKit), `cargo test` and `cargo clippy` |

## Documentation

- [Module responsibilities and call chains](docs/modules/README.md)
- [Plugin development guide](docs/plugin-development.md) and [Plugin API reference](docs/plugin-api-reference.md), with an example in [`examples/plugins/imagebed-example`](examples/plugins/imagebed-example)
- [Changelog](CHANGELOG.md) and [version archives](docs/archive)
- [1.0.6.5.1 release validation record](docs/validation-1.0.6.5.1.md)

<details>
<summary><strong>Development and builds</strong></summary>

You need Node.js, pnpm, Rust, and the Tauri 2 platform prerequisites.

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

Releases are produced by the Windows and macOS workflows in `.github/workflows`: both platforms build, sign, and hash their packages, a shared update manifest is generated, and the release is published only after every check passes.
</details>

## Star History

<img src=".github/assets/star-history.svg" alt="GitHub star history for Bai-YB/hexo-lite-editor">

[Open an issue](https://github.com/Bai-YB/hexo-lite-editor/issues) · [Browse releases](https://github.com/Bai-YB/hexo-lite-editor/releases) · [Contribute](https://github.com/Bai-YB/hexo-lite-editor/pulls)

Released under the [MIT License](LICENSE).
