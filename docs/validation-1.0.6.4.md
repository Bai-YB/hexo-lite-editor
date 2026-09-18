# Hexo Lite Editor 1.0.6.4 验证记录

对外版本 `1.0.6.4`；Cargo/Tauri/updater `1.0.6+4`；Windows MSI `1.0.6.4`；macOS 短版本 `1.0.6`、bundle `1.0.604`。

## 修改与回归范围

- 启动首屏不等待更新 IPC、最近项目扫描或其他页面模块；设置页可在模拟的更新与项目延迟期间正常进入。
- 项目选择、自动恢复与最近项目打开的磁盘扫描在 Tauri blocking worker 执行，会话 generation、文件锁和打开后同步调度保持原约束。
- 已存在当前配置或一次性迁移标记时跳过旧应用目录递归扫描；首次迁移仍复制缺失数据且不删除旧目录。
- 原始 HTML 的 class、CSS 自定义变量、渐变、布局、阴影、伪类和响应式规则可以在即时预览中生效；所有规则限定在预览容器，活动内容和越界样式继续过滤。
- Markdown 与图片源只做一次主解析；连续输入取消旧渲染任务，预览状态明确显示正在渲染。
- 编辑器与预览继续使用源码锚点双向同步；预览主控期间的布局重测只按已映射源码行定位，不再读取 CodeMirror 异步跳转前的旧边界。反复向前/向后跳转、长图尺寸变化、段落换行、代码块以及顶部/底部边界均纳入回归。
- 普通设置分类不再触发项目同步扫描；同步页先显示三步规划和 GitHub/WebDAV 互斥选择，只挂载当前 provider 的表单。状态、凭据和关闭同步涉及的原生阻塞工作均在 worker 执行。
- 模块文档覆盖职责、入口与调用链；版本测试检查模块基线、归档、验证记录和发布说明。

## 本地验证

- `pnpm check`：0 errors、0 warnings。
- `pnpm test -- --run`：29 个文件、165 项通过。
- `pnpm build`：生产构建通过。
- `pnpm audit --prod --registry https://registry.npmjs.org`：无已知漏洞；发布前新增的 `devalue` 公告已通过工作区 override 固定到 5.9.2，并经冻结锁文件安装复核。
- `cargo fmt --all -- --check`、`cargo clippy --all-targets --all-features -- -D warnings`：通过。
- `cargo check --all-targets --all-features`：通过；`cargo test --all-targets --all-features`：116 项通过，其中同步定向 44 项通过。
- `CI=true pnpm exec playwright test --workers=1`：Chromium/WebKit 共 124 项通过，覆盖启动延迟、HTML/CSS 真实计算样式、六组设置宽窄屏交互、同步方式选择/慢速检查进度和滚动同步。
- 预览反向同步竞态修复另做双内核各 10 次压力回归，共 20/20 通过；包含正向、反向、反复跨段和首尾边界的滚动专项另为 8/8 通过。
- 首个编辑器冷启动就绪断言在 Chromium/WebKit 各重复 10 次，共 20/20 通过；修复依赖后完整双引擎矩阵再次取得 124/124。
- Chromium/WebKit 在 1360px 与 640px 下人工复核 GitHub/WebDAV 选择页和两套配置表单，布局与交互一致。

## 安全与兼容边界

- 文章样式只能影响 `.markdown-preview` 内部，容器采用 layout/paint/style containment；不允许外部 CSS、`url()`、脚本、事件、iframe、表单、SVG/MathML、固定/粘性定位或过大 z-index。
- 围栏 HTML 代码仍按源码展示；直接写入 Markdown 正文的 HTML 才参与渲染。
- Windows 与 macOS 继续共享相同前端实现和浏览器回归；原平台安装包由对应 GitHub runner 构建。
- 配置 schema、项目会话结构、保存、图片、同步、发布和更新接口保持兼容。

## 发布结果

等待 v1.0.6.4 双平台工作流、集中公开、资产复核和发布后缓存清理完成后补录。
