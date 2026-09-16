# Hexo Lite Editor 1.0.6.3 验证记录

对外版本 `1.0.6.3`；Cargo/Tauri/updater `1.0.6+3`；Windows MSI `1.0.6.3`；macOS 短版本 `1.0.6`、bundle `1.0.603`。

## 修改与回归范围

- HTML `align`、`valign`、`bgcolor` 转为受控 CSS；`font` 转为 `span`，保留颜色、字体和 1–7 号字体尺寸，内联样式优先。
- HTML 图片声明尺寸与居中保留；添加逻辑尺寸/间距、溢出、图片适配与宽高比的安全样式支持。本地图片解析、占位和重试使用原流程。
- HTML 源码锚点写入实际元素，避免表格解析将额外 `span` 移到错误位置；保留标准 Markdown 的 HTML 分块语义和围栏代码源码显示。
- 所有平台使用相同表单 CSS，去除 WebKit 原生输入/下拉框额外外观，统一箭头、尺寸、焦点和开关结构。
- 设置页通过 ResizeObserver 测量页头/导航高度，吸顶和定位使用测量值；分类切换回到顶部，横向分类栏显示当前项，键盘焦点不意外滚动页面。
- 数字字段使用 input 事件更新草稿，保存快捷键无需失焦。保留校验、取消、保存期间的新修改及即时连接操作语义。
- 浏览器回归覆盖编辑/图片/文件/插件/同步/预览/更新/语言/主题/对话框、六组设置与 1360/1120/640px 窗口。Demo 图片使用本地响应避免第三方延迟，具体图片异常测试仍覆盖原路径。
- 发布开始前和保存完成后都检查图片上传状态，防止保存过程中启动的上传把临时 `hlex-asset` 地址带入 Hexo 发布；定向回归用两个 Promise 闸门精确覆盖该窗口。
- Playwright 在 CI 使用单 worker，避免两套重量级 WebKit/Chromium 实例同时冷启动造成无关用例超时；本地仍保留双 worker。发布、上传、自动更新测试等待真实状态，不再依赖固定墙钟时限。
- WebKit 源码滚动在 CodeMirror 行边界 1.5 CSS px 内吸附；页面入场只淡入，离场用可逆状态禁用命中，避免页面反向切换后永久不可点击。
- Windows 与 macOS 发布均运行交互检查；macOS 在 macos-14 上运行完整 WebKit 项目。两端构建成功且对应同一源码提交后，集中验证资产与更新签名并公开 Release。

## 本地验证

- `pnpm check`：0 errors、0 warnings。
- `pnpm test`：29 个文件、160 项通过；包含新增 HTML 展示属性、样式优先级、安全过滤及表格/折叠结构回归。
- `pnpm build`：生产构建成功；`pnpm audit --prod`：未发现已知漏洞。
- `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`、`cargo clippy --all-targets --all-features -- -D warnings` 均通过；`cargo test --all-targets --all-features` 115 项通过。
- impeccable detector：运行一次，结果为空。截图复核包含两端设置保存/焦点、吸顶和真实编辑的 HTML 排版；控件额外边框已消除。
- 新增交互回归覆盖不失焦保存/校验/取消、主题预览、1360/1120/640px 六组分类、HTML 折叠/代码显示，以及发布保存过程中才启动图片上传的竞态。
- 最终以 `CI=true`、单 worker 完整运行 Chromium/WebKit 矩阵，118/118 项通过（每个引擎 59 项）；发布/上传/设置/页面过渡/源码滚动关键路径另有双引擎 30 项压力复跑通过，发布竞态与自动更新状态等待各重复 5 次共 20 项通过。

## 设置与前端交互审查

| 问题 | 影响与处理 | 验证 |
| --- | --- | --- |
| WebKit 控件额外原生外观 | 下拉框出现额外边框；共用 CSS 统一 appearance、箭头和焦点背景 | 两引擎截图及全设置交互 |
| 数字字段失焦后才更新 | 保持焦点保存可能提交旧值；改用 input 更新 | Ctrl+S / ⌘S、校验失败、取消 |
| 固定吸顶高度 | 页头换行或保存区出现时可能遮挡导航；测量真实高度 | 三种窗口、未保存状态 |
| WebKit 自定义滚动条宽度变化 | 短分类未预留滚动条空间导致居中内容横移；固定纵向滚动轨道 | 分类切换实际边界/可用宽度断言 |
| 切分类保留旧滚动 | 新分类内容不在可见起始位置；重置顶部并展示当前横向分类 | 六组循环、Home/End/方向键 |
| HTML 锚点额外节点 | 表格解析会移走 span；锚点改为实际节点 | DOM 结构和原有源码同步回归 |

编辑器、文件、图片、插件、同步、更新、主题/语言与模态保护继续走原有共用组件。macOS 系统关闭事件与 Windows 关闭按钮调用同一关闭保护；浏览器测试触发此共用入口，不把隐藏的 Windows 标题栏按钮当作 macOS 可见控件。

## 发布结果

- 发布源码提交：[`cdbe357df32d14d00034eeeb01fcff6d6bd09d58`](https://github.com/Bai-YB/hexo-lite-editor/commit/cdbe357df32d14d00034eeeb01fcff6d6bd09d58)；`main`、`v1.0.6.3`、Windows/macOS 清单和 Release target 均指向该提交。
- [CI 35084070674](https://github.com/Bai-YB/hexo-lite-editor/actions/runs/35084070674)、[Windows 35084075028](https://github.com/Bai-YB/hexo-lite-editor/actions/runs/35084075028)、[macOS 35084074945](https://github.com/Bai-YB/hexo-lite-editor/actions/runs/35084074945) 全部成功；Windows 便携包、NSIS、MSI smoke test 及 macOS WebKit parity 均通过。
- [Finalize 35086218276](https://github.com/Bai-YB/hexo-lite-editor/actions/runs/35086218276) 验证双平台构建、11 项平台/清单文件 SHA256、更新包大小和 Minisign 签名后，于 2026-09-16 18:40（Asia/Shanghai）公开 [v1.0.6.3 Release](https://github.com/Bai-YB/hexo-lite-editor/releases/tag/v1.0.6.3)。
- Release 为 Latest、非草稿、非预发布，共 14 个资产：Windows Setup/MSI/便携 ZIP、macOS Universal DMG/APP ZIP/更新包、签名、双平台清单与校验和，以及三平台 `latest.json`。
- 公开后将全部 14 个资产下载到独立临时目录复核；`latest.json` 三平台条目、Windows/macOS 更新签名、包大小与两份 SHA256 清单均通过，临时下载随后删除。
- 清除开发目录可再生缓存 `.svelte-kit`、`build`、`output`、`node_modules`、`src-tauri/target`、`src-tauri/gen/schemas`，共 8,241,547,248 字节（约 7.68 GiB）；未触及用户博客、应用数据、凭据或全局 pnpm/Cargo/Playwright 缓存。

## 实际边界

- 本机为 Windows。WebKit 本地检查使用 Playwright WebKit，原平台检查在 macOS Actions 执行；浏览器用 demo fixture 验证前端，未声称已人工操作 macOS 安装后的 WKWebView 窗口。
- 两端业务控件、布局规则与操作流程共用代码；系统标题栏、Ctrl/⌘ 快捷键、字体和系统文件选择器保留平台约定，像素输出可不同。
- 不执行文章中的脚本、事件、iframe、表单、SVG 或外部 CSS，不恢复任意路径/命令能力；HTML class 仍只保留代码语言类。主题专属组件与嵌入服务继续交给真实 Hexo 网站预览。
- 本版配置 schema 仍为 3，项目/同步/凭据接口保持兼容。缓存清理仅针对本开发目录生成项，不触及用户博客、操作系统凭据或系统缓存。
