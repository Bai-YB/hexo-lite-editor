<div align="center">
  <img src=".github/assets/app-icon.png" width="104" height="104" alt="Hexo Lite Editor icon">
  <h1>Hexo Lite Editor</h1>
  <p><strong>A quiet desktop workspace for Hexo writing, images, preview, and publishing.</strong></p>
  <p><a href="README.md">简体中文</a> · <a href="README_EN.md">English</a></p>
  <p>
    <a href="https://github.com/Bai-YB/hexo-lite-editor/releases/latest"><img alt="Release" src="https://img.shields.io/github/v/release/Bai-YB/hexo-lite-editor?display_name=tag&style=flat-square"></a>
    <a href="https://github.com/Bai-YB/hexo-lite-editor/stargazers"><img alt="GitHub Stars" src="https://img.shields.io/github/stars/Bai-YB/hexo-lite-editor?style=flat-square"></a>
    <a href="https://github.com/Bai-YB/hexo-lite-editor/releases"><img alt="Downloads" src="https://img.shields.io/github/downloads/Bai-YB/hexo-lite-editor/total?style=flat-square"></a>
    <a href="LICENSE"><img alt="License" src="https://img.shields.io/github/license/Bai-YB/hexo-lite-editor?style=flat-square"></a>
  </p>
  <p>
    <a href="https://github.com/Bai-YB/hexo-lite-editor/releases/download/v1.0.5/Hexo-Lite-Editor_1.0.5_windows-x64-setup.exe"><strong>Windows setup</strong></a>
    · <a href="https://github.com/Bai-YB/hexo-lite-editor/releases/download/v1.0.5/Hexo-Lite-Editor_1.0.5_windows-x64-portable.zip">Portable ZIP</a>
    · <a href="https://github.com/Bai-YB/hexo-lite-editor/releases/download/v1.0.5/Hexo-Lite-Editor_1.0.5_windows-x64.msi">MSI</a>
    · <a href="https://github.com/Bai-YB/hexo-lite-editor/releases/tag/v1.0.5">Release and SHA-256</a>
  </p>
  <sub>Current public release v1.0.5 · Windows 10/11 x64</sub>
</div>

## Writing and live preview

Open a post from the left, write Markdown in the center, and read sanitized HTML on the right. Hiding the preview gives the editor all space outside the post list; when the actual theme matters, open the real Hexo page from the same toolbar.

<p align="center">
  <img src=".github/assets/writing-workflow.gif" alt="Selecting a post, editing Markdown, updating live preview, and hiding and restoring preview">
</p>

Saving, browser preview, new posts, and publishing all stay in the current context instead of living on unrelated pages. Node.js and Hexo are not bundled; the app only invokes the blog's existing environment for preview, generation, and deployment.

<details>
<summary><strong>Choose a Windows package</strong></summary>

| Package | Best for |
| --- | --- |
| Setup EXE | Everyday use; can install WebView2 Runtime when needed |
| Portable ZIP | Extract and run without writing installation metadata |
| MSI | Managed environments or installations that explicitly require MSI |

The project is not signed with a commercial code-signing certificate, so Windows SmartScreen may show “Unknown publisher.” Download only from this repository and compare files with `SHA256SUMS.txt` in the same Release. There is no public macOS DMG in `v1.0.5`; the repository keeps a universal macOS workflow that can produce Intel and Apple Silicon builds on a Mac.
</details>

## Image organization

Local images, Cloudflare-ImgBed assets, imports, paste, and drag-and-drop use one directory and Markdown prefix model. The image workspace follows real folders and handles images and other files without moving image-bed management into a browser tab.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset=".github/assets/image-bed-dark.png">
  <img src=".github/assets/image-bed-light.png" alt="Hexo Lite Editor image workspace with all local image thumbnails loaded">
</picture>

The article list reads only `cover`, `top_img`, `banner`, `thumbnail`, or `index_img` explicitly declared in Front Matter. It never guesses the first body image and never replaces a broken cover with an unrelated default.

## Sync and publishing

Content can sync through an isolated GitHub branch or a standard WebDAV service you control. WebDAV endpoint, remote directory, username, and password stay visible and editable. Passwords are never returned, and new credentials reach the OS vault only after directory access and a reversible read/write probe succeed.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset=".github/assets/content-sync-dark.png">
  <img src=".github/assets/content-sync-light.png" alt="WebDAV after a real connection test with endpoint, directory, username, and password still editable">
</picture>

Changing the endpoint or directory requires another test and an explicit apply action; it never triggers an automatic upload or download. A first connection asks whether to upload local content or use remote content. Later conflicts are resolved per file, with a local backup before remote content is applied.

## Capabilities and boundaries

| Work | Included | Explicit boundary |
| --- | --- | --- |
| Writing | Posts and drafts, CodeMirror, independently scrolling live preview, light/dark/system themes | Not a WYSIWYG editor |
| Images | Local images under `source/`, Cloudflare-ImgBed, paste/drag/import, visible error states | Local directories cannot escape the current project's `source/` |
| Preview and publishing | Browser-based Hexo preview, clean, generate, deploy, Git status | Does not bundle Node.js, Hexo, or blog dependencies |
| Content sync | Isolated GitHub branch, WebDAV Basic Auth, hash manifest, conflict choices, pre-pull backups | Off by default; excludes drafts, site config, environment files, and credentials |
| HTML safety | Common semantic HTML, tables, images, and restricted inline styles | Scripts, event attributes, iframes, forms, SVG, and escaping styles are removed |
| Systems | Published Windows 10/11 x64 packages; a macOS build workflow in the repository | The `v1.0.5` Release currently has no public macOS installer |

## Get started

1. Download and open the setup, portable ZIP, or MSI package.
2. Select a Hexo root containing `_config.yml`.
3. Open a post or draft from the left and edit Markdown or supported common HTML.
4. Check live preview; use **Browser Preview** when you need the actual theme.
5. Save and choose **Publish**, or run generate and deploy individually from the advanced menu.

Local writing does not require Node.js. Browser preview, generate, and deploy require Node.js, Hexo, and the target blog's dependencies.

## Security and data boundaries

- DOMPurify sanitizes live preview, and production builds use a fixed CSP. HTML inside code fences always stays source code.
- Cloudflare administrator passwords are used only for temporary login. Tokens and WebDAV passwords stay in the OS credential vault and never enter config, logs, or sync manifests.
- Content sync includes only `source/_posts/`, per-post asset folders, and the configured image directory, with a recovery backup before remote content overwrites local files.
- Local image paths reject absolute paths, `..`, and symlink escapes. Remote images load directly in WebView so system caching and connection reuse remain available.

<details>
<summary><strong>Remote images, error states, and HTML details</strong></summary>

HTTP/HTTPS images are not fully downloaded or decoded by the backend before display. Any image body WebView can render is shown as-is, including a valid image returned with a 404 status. Empty responses, network errors, or undecodable content produce a size-preserving error box in the article and an error icon in the post list. They never fall back to a default cover or briefly flash the original image while switching posts.

Live preview supports common typography, tables, `details/summary`, `figure/figcaption`, images, and restricted inline styles. It does not execute scripts and rejects iframes, objects, forms, SVG, and `<style>` blocks.
</details>

<details>
<summary><strong>GitHub and WebDAV content-sync details</strong></summary>

GitHub mode uses an isolated content branch without switching or modifying the project's current branch. WebDAV mode supports HTTPS, Basic Auth, `PROPFIND`, `MKCOL`, `PUT`, `GET`, and `DELETE`. It stores immutable content-addressed objects and conditionally updates `.hexo-lite-sync.json` last so concurrent devices cannot silently overwrite one another.

Candidate WebDAV credentials are saved only after Basic Auth, directory access, upload, download, deletion probe, and remote-manifest validation all succeed. Authentication, network, permission, read-only directory, corrupt manifest, and probe-cleanup failures remain distinct actionable errors.
</details>

<details>
<summary><strong>Keyboard shortcuts</strong></summary>

| Shortcut | Action |
| --- | --- |
| `Ctrl/⌘ + O` | Open a Hexo project |
| `Ctrl/⌘ + S` | Save the current post |
| `Ctrl/⌘ + N` | Create a post |
| `Ctrl/⌘ + F` | Search in the editor |
| `Ctrl/⌘ + Shift + P` | Publish the current blog |
| `Ctrl/⌘ + ,` | Open settings |
| `Ctrl/⌘ + 1…4` | Switch Editor, Images, Settings, and About |
| `↑ / ↓` | Move through the focused post list |
</details>

<details>
<summary><strong>Development, verification, and builds</strong></summary>

You need Node.js, pnpm, Rust, and the platform prerequisites for Tauri 2.

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

README assets and the local Star History can be reproduced with:

```bash
pnpm readme:assets
pnpm readme:stars -- --repository Bai-YB/hexo-lite-editor
```
</details>

## Project activity

<img src=".github/assets/star-history.svg" alt="GitHub Star History for Bai-YB/hexo-lite-editor, generated in the repository and updated weekly">

The chart reads real timestamps from the GitHub API every week and is generated inside this repository. A repository with no stars gets an honest zero state, not a fabricated growth curve or a dependency on an external chart service.

[Open an Issue](https://github.com/Bai-YB/hexo-lite-editor/issues) · [Browse Releases](https://github.com/Bai-YB/hexo-lite-editor/releases) · [Contribute](https://github.com/Bai-YB/hexo-lite-editor/pulls)

Hexo Lite Editor is released under the [MIT License](LICENSE). Never attach credentials or unredacted diagnostics to a public Issue.
