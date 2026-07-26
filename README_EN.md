<div align="center">
  <img src=".github/assets/app-icon.png" width="112" height="112" alt="Hexo Lite Editor icon">
  <h1>Hexo Lite Editor</h1>
  <p>A quiet, native Windows and macOS workspace for Hexo writing, image management, and publishing.</p>
  <p><a href="README.md">简体中文</a> · <a href="README_EN.md">English</a></p>
  <p>
    <a href="https://github.com/Bai-YB/hexo-lite-editor/releases/latest"><img alt="Release" src="https://img.shields.io/github/v/release/Bai-YB/hexo-lite-editor?display_name=tag&style=flat-square"></a>
    <a href="https://github.com/Bai-YB/hexo-lite-editor/releases"><img alt="Downloads" src="https://img.shields.io/github/downloads/Bai-YB/hexo-lite-editor/total?style=flat-square"></a>
    <a href="LICENSE"><img alt="License" src="https://img.shields.io/github/license/Bai-YB/hexo-lite-editor?style=flat-square"></a>
    <a href="https://github.com/Bai-YB/hexo-lite-editor/actions/workflows/release-windows.yml"><img alt="Windows Release" src="https://github.com/Bai-YB/hexo-lite-editor/actions/workflows/release-windows.yml/badge.svg"></a>
    <a href="https://github.com/Bai-YB/hexo-lite-editor/actions/workflows/release-macos.yml"><img alt="macOS Build" src="https://github.com/Bai-YB/hexo-lite-editor/actions/workflows/release-macos.yml/badge.svg"></a>
  </p>
</div>

<picture>
  <source media="(prefers-color-scheme: dark)" srcset=".github/assets/editor-dark.png">
  <img src=".github/assets/editor-light.png" alt="Hexo Lite Editor with the article list, Markdown source, and live preview">
</picture>

Hexo Lite Editor brings article editing, live preview, image organization, browser-based Hexo preview, and publishing tasks into one focused desktop app. Node.js and Hexo are not bundled: the app only invokes the environment already installed in your blog when you preview, generate, or deploy it.

## Download

The current version is **v1.0.5** for Windows 10/11 x64. The universal macOS build supports Intel and Apple Silicon on macOS 11 or newer.

| Package | Best for | Download |
| --- | --- | --- |
| Setup EXE (recommended) | Everyday use; can install WebView2 when needed | [Download setup](https://github.com/Bai-YB/hexo-lite-editor/releases/download/v1.0.5/Hexo-Lite-Editor_1.0.5_windows-x64-setup.exe) |
| Portable ZIP | Extract and run without installation | [Download portable](https://github.com/Bai-YB/hexo-lite-editor/releases/download/v1.0.5/Hexo-Lite-Editor_1.0.5_windows-x64-portable.zip) |
| MSI | Managed environments that require MSI | [Download MSI](https://github.com/Bai-YB/hexo-lite-editor/releases/download/v1.0.5/Hexo-Lite-Editor_1.0.5_windows-x64.msi) |
| macOS DMG | Intel and Apple Silicon; drag to Applications | `Build macOS` GitHub Actions artifact |

[Full release and checksums](https://github.com/Bai-YB/hexo-lite-editor/releases/tag/v1.0.5) · [Changelog](CHANGELOG.md)

> The project is not signed with a commercial code-signing certificate yet, so Windows SmartScreen may show an “Unknown publisher” warning. Download only from this repository and verify files against `SHA256SUMS.txt` in the same release. The installer can bootstrap Microsoft Edge WebView2 Runtime; the portable build links to Microsoft's official download when WebView2 is missing.

## Highlights

- **Focused writing** — CodeMirror editing, post and draft lists, independently scrolling live preview, and light, dark, or system themes.
- **Explicit post covers** — The article list reads only `cover`, `top_img`, `banner`, `thumbnail`, or `index_img` declared in Front Matter; it never guesses the first body image as a cover.
- **Safe native HTML** — Common HTML in article content renders correctly. DOMPurify removes scripts, event handlers, iframes, forms, dangerous URLs, and styles that could escape the preview. HTML inside code fences stays source code.
- **Fast remote images** — Markdown and HTML previews load HTTP/HTTPS images directly from their original URLs so WebView can reuse connections and cache. Empty or failed responses remain explicit.
- **Configurable local image bed** — Keep images under a chosen path inside Hexo `source/` and set the Markdown URL prefix to match the site. Import, paste, drag-and-drop, listing, and references use the same configuration.
- **CloudFlare-ImgBed integration** — Compatible with [CloudFlare-ImgBed v2.7.5](https://github.com/MarSeventh/CloudFlare-ImgBed/tree/v2.7.5). The app can create a token limited to upload, list, and delete; the administrator password is temporary and the token stays in the OS credential vault.
- **Reliable publishing** — Open the real Hexo site in your browser, or run clean, generate, deploy, and Git status checks from the editor.
- **Two sync transports** — Sync posts and assets to either an isolated GitHub content branch or your own WebDAV server, with the same manifest, conflict choices, and local backups.
- **Clear settings** — General, Editing, Images, Hexo & Publishing, Content Sync, and Diagnostics are distinct sections with one visible panel at a time.

<details>
<summary><strong>More real product screenshots</strong></summary>

<table>
  <tr>
    <td width="50%"><strong>Image library</strong><br><img src=".github/assets/image-bed-light.png" alt="Image library"></td>
    <td width="50%"><strong>Six-section settings</strong><br><img src=".github/assets/settings-light.png" alt="Settings in light mode"></td>
  </tr>
  <tr>
    <td width="50%"><strong>Dark writing workspace</strong><br><img src=".github/assets/editor-dark.png" alt="Editor in dark mode"></td>
    <td width="50%"><strong>Dark settings</strong><br><img src=".github/assets/settings-dark.png" alt="Settings in dark mode"></td>
  </tr>
</table>
</details>

## Quick start

1. Download and open the setup, portable, or MSI package.
2. Select a Hexo blog folder containing `_config.yml`.
3. Open a post from the left pane and edit Markdown or supported native HTML.
4. Check the live preview. Use **Browser Preview** when you need the real Hexo theme and route.
5. Save and choose **Publish**, or run individual generate/deploy steps from the advanced menu.

Starting the app and editing local files do not require Node.js. Browser preview, generate, and deploy require Node.js, Hexo, and the target blog's dependencies.

## Images, HTML, and security boundaries

Live preview supports common typography, tables, `details/summary`, `figure/figcaption`, images, and a restricted set of inline styles. It never executes scripts and rejects iframes, objects, forms, SVG, and `<style>` blocks. Production builds also use a fixed Content Security Policy.

HTTP/HTTPS images in live preview load directly from their original URLs so WebView can reuse its cache, connections, and system network settings. The app no longer pre-downloads or decodes remote images. Any response WebView can display is shown, including an image body returned with a 404 status. Empty, failed, or undecodable responses show an error box in the article and an error icon for an article cover instead of falling back to the default cover. Local images still use controlled asset URLs bound to the current project session.

Local image directories must be relative paths under `source/`; absolute paths, `..`, and symlink escapes are rejected. CloudFlare-ImgBed administrator passwords are never written to config, logs, or browser storage. Removing a local token does not revoke its server-side counterpart.

## GitHub and WebDAV content sync

Content sync is off by default and includes only `source/_posts/`, per-post resource folders, and the configured image directory. Drafts, site configuration, environment files, and credentials are excluded. GitHub mode uses an isolated content branch without switching or changing the project's current branch. WebDAV mode writes immutable content-addressed objects under the selected remote directory and conditionally updates `.hexo-lite-sync.json` last, preventing concurrent devices from silently overwriting each other.

The WebDAV transport supports HTTPS with Basic Auth plus `PROPFIND`, `MKCOL`, `PUT`, `GET`, and `DELETE`. Endpoint, remote directory, username, and password remain editable, while passwords are never returned to the UI. Candidate credentials reach the OS credential vault only after real directory access, a reversible read/write/delete probe, and remote-manifest validation all succeed. Enabling sync or changing the connection requires an explicit first direction. Later saves schedule a delayed sync; simultaneous edits become per-file conflicts, and remote changes are backed up before they are applied locally.

## Keyboard shortcuts

| Shortcut | Action |
| --- | --- |
| `Ctrl/⌘ + O` | Open a Hexo project |
| `Ctrl/⌘ + S` | Save the current article |
| `Ctrl/⌘ + N` | Create an article |
| `Ctrl/⌘ + F` | Search in the editor |
| `Ctrl/⌘ + Shift + P` | Publish the current blog |
| `Ctrl/⌘ + ,` | Open settings |
| `Ctrl/⌘ + 1…4` | Switch Editor, Images, Settings, and About |
| `↑ / ↓` | Move through the focused article list |

## Development and builds

You need Node.js, pnpm, Rust, and the Windows prerequisites for Tauri 2.

```bash
pnpm install
pnpm check
pnpm test
pnpm test:e2e
pnpm build
pnpm tauri dev
```

Rust checks and desktop bundles:

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
pnpm tauri build
```

Build the setup, MSI, portable ZIP, SHA-256 file, and release manifest:

```powershell
pnpm release:windows
```

## Feedback and license

For reproducible bugs, open an [Issue](https://github.com/Bai-YB/hexo-lite-editor/issues) with your Windows version, steps, and a relevant redacted diagnostics excerpt. Do not post credentials or unredacted logs in public issues.

Hexo Lite Editor is released under the [MIT License](LICENSE).
