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
    <a href="https://github.com/Bai-YB/hexo-lite-editor/releases/download/v1.0.6.1/Hexo-Lite-Editor_1.0.6.1_windows-x64-setup.exe"><strong>Download setup</strong></a>
    · <a href="https://github.com/Bai-YB/hexo-lite-editor/releases/download/v1.0.6.1/Hexo-Lite-Editor_1.0.6.1_windows-x64-portable.zip">Portable ZIP</a>
    · <a href="https://github.com/Bai-YB/hexo-lite-editor/releases/download/v1.0.6.1/Hexo-Lite-Editor_1.0.6.1_macos-universal.dmg">macOS</a>
    · <a href="https://github.com/Bai-YB/hexo-lite-editor/releases/tag/v1.0.6.1">v1.0.6.1</a>
  </p>
</div>

<p align="center">
  <img src=".github/assets/writing-workflow.gif" alt="Selecting a post, editing Markdown, updating preview, and hiding and restoring preview">
</p>

## Features

- Edit posts and drafts with Markdown, common HTML, and live preview.
- Align editor and preview by source content, including images, code blocks and long paragraphs.
- Browse All files to edit friend links, YAML, JSON and theme configuration with disk-change checks; manage plugins from their own navigation entry.
- Hide the preview to give the editor all space outside the post list.
- Manage images under `source/` or connect Cloudflare-ImgBed; pasted and dropped images appear locally while uploading in the background.
- Publishing always saves, cleans, generates, and deploys to prevent stale output; the background preview server can be stopped at any time.
- Sync Hexo project sources, themes, and configuration through an isolated GitHub branch or a standard WebDAV service, with an explicit remote/local choice when the cloud advances.
- Automatically check daily and download signature-verified updates in the background, then ask before restarting to install.
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

`v1.0.6.1` supports Windows 10/11 x64 and universal macOS. The displayed version is **1.0.6.1**; the internal Tauri-compatible version is `1.0.6+1`. Both 1.0.6 and 1.0.6-r1 can recognize this update.

This release improves background synchronization, source-based preview scrolling, independent plugin navigation and project file editing. Sync displays stages, file progress and a stop action. Opening a blog enters the local workspace before checking the cloud. See the [1.0.6.1 validation record](docs/validation-1.0.6.1.md).

| Package | Use |
| --- | --- |
| [Setup EXE](https://github.com/Bai-YB/hexo-lite-editor/releases/download/v1.0.6.1/Hexo-Lite-Editor_1.0.6.1_windows-x64-setup.exe) | Recommended; install with the setup wizard |
| [Portable ZIP](https://github.com/Bai-YB/hexo-lite-editor/releases/download/v1.0.6.1/Hexo-Lite-Editor_1.0.6.1_windows-x64-portable.zip) | Extract and run |
| [MSI](https://github.com/Bai-YB/hexo-lite-editor/releases/download/v1.0.6.1/Hexo-Lite-Editor_1.0.6.1_windows-x64.msi) | Managed or bulk deployment |
| [macOS DMG](https://github.com/Bai-YB/hexo-lite-editor/releases/download/v1.0.6.1/Hexo-Lite-Editor_1.0.6.1_macos-universal.dmg) | Universal for Intel and Apple Silicon |

The Windows packages do not have a commercial code-signing certificate, so SmartScreen may show “Unknown publisher.” Checksums are in [SHA256SUMS.txt](https://github.com/Bai-YB/hexo-lite-editor/releases/download/v1.0.6.1/SHA256SUMS.txt) and [SHA256SUMS-macos.txt](https://github.com/Bai-YB/hexo-lite-editor/releases/download/v1.0.6.1/SHA256SUMS-macos.txt). Starting with v1.0.6, the app downloads later Tauri signature-verified updates automatically and asks before installation.

## Usage

1. Open a Hexo blog directory containing `_config.yml`.
2. Select a post or draft on the left and edit it.
3. Check the live preview, or open browser preview when you need the actual theme.
4. Save and choose **Publish**, or run generate and deploy separately from the advanced menu.

Open friend links and theme files in **All files** (Ctrl+6), and plugins with Ctrl+5. Project files use explicit saving (Ctrl+S), with unsaved-change prompts when switching files, leaving the workspace or quitting.

For the first sync, check the connection, enable it and confirm the direction. Later saves upload after 30 seconds, or use **Upload changes now**. Stopping waits for the current network request, with a 30-second timeout per request. Completed remote commits remain in place.

## Notes

- Node.js, Hexo, and blog dependencies are not bundled. Local editing works without Node.js; preview and publishing use the project's existing environment.
- Live preview removes unsafe HTML such as scripts, iframes, and forms. HTML in code fences remains source code.
- Tokens and WebDAV passwords stay in the OS credential vault and are not written to project config, logs, or sync manifests.
- Project sync is off by default and includes posts, drafts, `source/`, themes, scaffolds, and Hexo/theme configuration. It excludes `.git`, `public`, `node_modules`, caches, environment files, and common credentials. A local backup is created before cloud content overwrites local files.
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
