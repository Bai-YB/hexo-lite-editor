<div align="center">
  <img src=".github/assets/app-icon.png" width="112" height="112" alt="Hexo Lite Editor 图标">
  <h1>Hexo Lite Editor</h1>
  <p>安静、原生的 Windows 与 macOS Hexo 写作、图片管理与发布工作区。</p>
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
  <img src=".github/assets/editor-light.png" alt="Hexo Lite Editor 编辑器、Markdown 源文与即时预览界面">
</picture>

Hexo Lite Editor 把文章编辑、即时预览、图片整理、Hexo 浏览器预览与发布任务收进一个专注的桌面界面。应用本身不内置 Node.js 或 Hexo；只有启动真实博客预览、生成或部署时，才会调用项目已有的环境。

## 下载

当前版本为 **v1.0.5**，支持 Windows 10/11 x64；macOS 通用构建支持 Intel 与 Apple Silicon，最低 macOS 11。

| 版本 | 适合场景 | 下载 |
| --- | --- | --- |
| 安装版 EXE（推荐） | 日常使用；安装器可补齐 WebView2 | [下载安装版](https://github.com/Bai-YB/hexo-lite-editor/releases/download/v1.0.5/Hexo-Lite-Editor_1.0.5_windows-x64-setup.exe) |
| 便携版 ZIP | 解压即用，不写入安装信息 | [下载便携版](https://github.com/Bai-YB/hexo-lite-editor/releases/download/v1.0.5/Hexo-Lite-Editor_1.0.5_windows-x64-portable.zip) |
| MSI | 企业部署或需要 MSI 的环境 | [下载 MSI](https://github.com/Bai-YB/hexo-lite-editor/releases/download/v1.0.5/Hexo-Lite-Editor_1.0.5_windows-x64.msi) |
| macOS DMG | Intel 与 Apple Silicon；拖入“应用程序”即可 | GitHub Actions 的 `Build macOS` 构建产物 |

[查看完整 Release 与校验文件](https://github.com/Bai-YB/hexo-lite-editor/releases/tag/v1.0.5) · [版本记录](CHANGELOG.md)

> 项目暂未使用商业代码签名证书。Windows SmartScreen 可能显示“未知发布者”；macOS 首次打开可能需要在 Finder 中右键应用并选择“打开”。请只使用本仓库构建产物并校验 SHA-256。

## 为什么使用它

- **专注写作**：CodeMirror 编辑器、文章/草稿列表、独立滚动的安全即时预览，以及浅色、深色和跟随系统主题。
- **显式文章封面**：文章列表只读取 Front Matter 明确声明的 `cover`、`top_img`、`banner`、`thumbnail` 或 `index_img`，不会把正文第一张图片猜成封面。
- **安全 HTML**：正文中的常用原生 HTML 可以正确渲染；脚本、事件属性、iframe、表单、危险 URL 与越界样式会被 DOMPurify 清理。代码围栏中的 HTML 仍按源码显示。
- **快速远程图片**：Markdown 与 HTML 远程图片直接交给 WebView 加载并复用网络缓存，不再由后端完整下载或解码校验；加载失败或空响应会显示明确的错误占位。
- **本地图床可配置**：图片目录限定在 Hexo `source/` 下，Markdown 前缀可以按站点结构调整；导入、粘贴、拖放、列表与引用使用同一套配置。
- **CloudFlare-ImgBed 联动**：兼容 [CloudFlare-ImgBed v2.7.5](https://github.com/MarSeventh/CloudFlare-ImgBed/tree/v2.7.5)，可创建仅含上传、列出和删除权限的 Token。密码只用于临时登录，Token 仅存入系统凭据库。
- **可靠发布**：在编辑器中启动浏览器预览，或运行 clean、generate、deploy 与 Git 状态检查；保存与发布仍遵循项目自己的 Hexo 配置。
- **双通道内容同步**：文章和资源既可同步到隔离的 GitHub 内容分支，也可同步到自己的 WebDAV 服务器；共用哈希清单、冲突选择和拉取前本地备份。
- **清楚的设置**：常规、编辑体验、图片与图床、Hexo 与发布、内容同步、诊断与维护分区展示，一次只显示一组设置。

<details>
<summary><strong>查看更多真实界面截图</strong></summary>

<table>
  <tr>
    <td width="50%"><strong>图床资源管理</strong><br><img src=".github/assets/image-bed-light.png" alt="图床资源管理界面"></td>
    <td width="50%"><strong>六分类设置</strong><br><img src=".github/assets/settings-light.png" alt="设置页面浅色模式"></td>
  </tr>
  <tr>
    <td width="50%"><strong>深色写作界面</strong><br><img src=".github/assets/editor-dark.png" alt="编辑器深色模式"></td>
    <td width="50%"><strong>深色设置界面</strong><br><img src=".github/assets/settings-dark.png" alt="设置页面深色模式"></td>
  </tr>
</table>
</details>

## 快速开始

1. 下载并启动安装版、便携版或 MSI。
2. 选择一个包含 Hexo `_config.yml` 的博客目录。
3. 从左侧文章列表打开文章，编辑 Markdown 或安全的常用 HTML。
4. 使用右侧即时预览检查正文；需要核对主题时，点击“浏览器预览”打开真实 Hexo 页面。
5. 保存后点击“发布”，或在高级菜单中单独运行 generate、deploy 等步骤。

应用启动和本地写作不要求 Node.js。浏览器预览、generate 与 deploy 要求目标博客已经安装 Node.js、Hexo 及项目依赖。

## 图片、HTML 与安全边界

即时预览允许常用排版、表格、`details/summary`、`figure/figcaption`、图片和受限内联样式，但不会执行脚本，也不支持 iframe、对象、表单、SVG 或 `<style>` 块。生产构建同时启用固定 CSP。

即时预览中的 HTTP/HTTPS 远程图片使用原始 URL 直接加载，保留 WebView 的缓存、连接复用和系统网络配置。预览不预先下载或解码校验图片；只要 WebView 能显示返回内容就按原样显示，包括服务器用 404 状态返回的错误图片。空响应、不可解码内容和网络错误会在正文中显示错误框，在文章列表中显示图片错误图标，不会回退为默认文章封面。本地图片仍通过当前项目会话的受控资源地址读取。

本地图片目录必须是 `source/` 下的相对路径，拒绝绝对路径、`..` 和符号链接逃逸。CloudFlare-ImgBed 管理员密码不会写入配置、日志或浏览器存储；删除本地 Token 不会远程撤销服务端 Token。

## GitHub 与 WebDAV 内容同步

内容同步默认关闭，只处理 `source/_posts/`、文章同名资源目录和设置中指定的图片目录，不会上传草稿、站点配置、环境变量或凭据。GitHub 模式使用独立内容分支，不切换或修改项目当前分支；WebDAV 模式在指定远端目录中使用按内容哈希命名的不可变对象，并在最后条件更新 `.hexo-lite-sync.json` 清单，避免并发设备互相覆盖。

WebDAV 支持 HTTPS、Basic Auth、`PROPFIND`、`MKCOL`、`PUT`、`GET` 和 `DELETE`。服务器地址、远端目录、用户名和密码始终可编辑；密码不会返回界面。候选凭据只有在真实目录访问、可逆读写删除探针和远端清单校验全部通过后才写入操作系统凭据库。首次启用或更换连接后必须明确选择“上传本地内容”或“使用远端内容”；以后保存后会延迟同步，遇到两端同时修改同一文件时会要求逐文件选择，并在写入本地前创建可恢复备份。

## 快捷键

| 快捷键 | 操作 |
| --- | --- |
| `Ctrl/⌘ + O` | 打开 Hexo 项目 |
| `Ctrl/⌘ + S` | 保存当前文章 |
| `Ctrl/⌘ + N` | 新建文章 |
| `Ctrl/⌘ + F` | 在编辑器中搜索 |
| `Ctrl/⌘ + Shift + P` | 发布当前博客 |
| `Ctrl/⌘ + ,` | 打开设置 |
| `Ctrl/⌘ + 1…4` | 切换编辑器、图床、设置、关于 |
| `↑ / ↓` | 在聚焦的文章列表中切换文章 |

## 开发与构建

需要 Node.js、pnpm、Rust 和 Tauri 2 对应平台的构建依赖。

```bash
pnpm install
pnpm check
pnpm test
pnpm test:e2e
pnpm build
pnpm tauri dev
```

Rust 检查与桌面安装包构建：

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
pnpm tauri build
```

生成安装版、MSI、便携版、SHA-256 和发行清单：

```powershell
pnpm release:windows
```

在 macOS 14 构建 Intel/Apple Silicon 通用 `.app` 与 `.dmg`：

```bash
rustup target add aarch64-apple-darwin x86_64-apple-darwin
pnpm release:macos -- 1.0.5
```

## 反馈与许可证

遇到可复现的问题，请提交 [Issue](https://github.com/Bai-YB/hexo-lite-editor/issues)，并附上系统版本、操作步骤和诊断日志中已脱敏的相关片段。安全问题请避免在公开 Issue 中提交凭据或未脱敏日志。

Hexo Lite Editor 使用 [MIT License](LICENSE) 发布。
