# Hexo Lite Editor 1.0.6 修复构建验证记录

本记录对应 [2026-09-10 代码审计](code-audit-2026-09-10.md) 的 **36 项问题（9 项 P1、27 项 P2）**。审计基线为 `25c4b9460ce034c77d2ce0d3617127417261565f`；原审计中的缺陷探针、56 个前端测试、79 个 Rust 测试和 28 个浏览器测试属于修复前证据，不能用来证明本次修复通过。

本文逐项核对实际实现和回归测试。最终修复提交已通过 Linux、Windows、macOS CI，安装包已公开发布；下载后的 14 个资产、12 条 SHA-256、3 份更新包签名和三平台更新清单均已验证。逐项证据及验证边界见下表和文末记录。

## 版本与交付约定

- 应用版本维持 **1.0.6**：`package.json`、`src-tauri/Cargo.toml`、`src-tauri/tauri.conf.json` 一致。
- 本次修复使用独立发布标签 **`v1.0.6-r1`**；保留原 `v1.0.6` 标签及发布，不将原标签移动到新提交。
- 已安装原 1.0.6 的用户需要下载本次修复构建后重新安装；应用版本号相同不会触发常规版本比较下的自动更新。
- 安装包、便携包、签名及更新清单必须来自本次修复提交。更新清单中的版本仍为 `1.0.6`，下载 URL 必须指向 `v1.0.6-r1`，不能指向旧 `v1.0.6` 资产。

## 逐项修复映射

### 应用壳与通用交互：A01–A05

| 编号 | 已核对的修复行为 | 实现位置 | 回归证据与边界 |
|---|---|---|---|
| A01 / P1 | 重扫结果校验项目、前后 generation、文档实例及 revision；等待中输入或切文时不加载旧响应。远端删除/更改转为显式恢复状态，提供保留本地、应用远端、另存草稿或关闭入口，并阻止直接自动保存覆盖。 | [projectRescan.ts](../src/app/projectRescan.ts)、[AppShell.svelte](../src/app/AppShell.svelte)、[EditorSessionStore.ts](../src/features/editor/EditorSessionStore.ts) | [projectRescan.test.ts](../src/app/projectRescan.test.ts) 覆盖干净重载、等待中输入、重开同文、切项目、远端删除、旧 generation；Store 测试覆盖外部更改保存保护。恢复弹窗的原生云端完整往返仍需最终手工验证。 |
| A02 / P2 | 移除遍历 DOM 的近似翻译，只在明确的 UI 文本处调用响应式翻译；正文、标题、路径及插值参数保持原值。 | 删除 `src/shared/i18n/legacyDom.ts`；[ui.ts](../src/shared/i18n/ui.ts)、[uiMessages.ts](../src/shared/i18n/uiMessages.ts) 及各页面调用点 | [ui.test.ts](../src/shared/i18n/ui.test.ts) 覆盖用户文本、路径、占位符和未知内容；[shell-audit-regressions.spec.ts](../tests/e2e/shell-audit-regressions.spec.ts) 检查正文及输入保留。 |
| A03 / P2 | 已挂载的界面文字订阅语言变化，中文→英文→中文可恢复；设置草稿取消时恢复已保存语言。 | [ui.ts](../src/shared/i18n/ui.ts)、[SettingsPage.svelte](../src/features/settings/SettingsPage.svelte)、[AppShell.svelte](../src/app/AppShell.svelte) | 同一 UI 单测验证现有订阅往返更新；同一 shell 浏览器用例验证控件文字往返及输入保留。 |
| A04 / P1 | 建立统一模态栈；全局发布/导航快捷键在模态打开时受阻；只有栈顶处理关闭，输入法组合期间不处理提交/关闭。 | [modalStack.ts](../src/shared/components/modalStack.ts)、[ModalDialog.svelte](../src/shared/components/ModalDialog.svelte)、[AppShell.svelte](../src/app/AppShell.svelte) | [ModalDialog.test.ts](../src/shared/components/ModalDialog.test.ts) 覆盖栈顶、销毁与 IME；shell 浏览器用例覆盖新建弹窗阻挡发布和导航。没有把一个新建弹窗用例等同于所有原生弹窗已手工验收。 |
| A05 / P2 | 设置输入与稳定 label/id 关联，说明关联 `aria-describedby`，开关和数字输入有名称。 | [SettingsPage.svelte](../src/features/settings/SettingsPage.svelte)、[CloudflareImageBedSettings.svelte](../src/features/settings/CloudflareImageBedSettings.svelte) | [SettingsPage.test.ts](../src/features/settings/SettingsPage.test.ts) 通过名称获取字号、主题、服务地址并操作；需结合最终浏览器检查。尚未执行真实屏幕阅读器审计。 |

### 文章编辑：E1–E8

| 编号 | 已核对的修复行为 | 实现位置 | 回归证据与边界 |
|---|---|---|---|
| E1 / P1 | 从文件读取/导入开始计入处理中状态；捕获项目与文档实例、插入书签，将插入点映射经过后续输入；晚结果不插入其他文档。处理中阻止切文、新建等冲突操作。 | [EditorPage.svelte](../src/features/editor/EditorPage.svelte)、[EditorSessionStore.ts](../src/features/editor/EditorSessionStore.ts)、[editorChanges.ts](../src/features/editor/editorChanges.ts) | [EditorSessionStore.test.ts](../src/features/editor/EditorSessionStore.test.ts) 覆盖插入位置映射、跨文档拒绝和选区已变；[editor-audit-regressions.spec.ts](../tests/e2e/editor-audit-regressions.spec.ts) 覆盖读取期间换文/新建受阻及后续输入。 |
| E2 / P1 | 上传成功只替换图片 URL，保留描述；系统 URL 变更不生成用户撤销步，撤销/重做中的旧 URL 在事务内归一化。缓存清理前检查已保存文章是否仍有引用。 | [imageHistory.ts](../src/features/editor/imageHistory.ts)、[MarkdownEditor.svelte](../src/features/editor/MarkdownEditor.svelte)、[EditorSessionStore.ts](../src/features/editor/EditorSessionStore.ts)、[images.rs](../src-tauri/src/commands/images.rs) | [imageHistory.test.ts](../src/features/editor/imageHistory.test.ts) 使用真实 CodeMirror history 验证撤销和描述编辑；Store 覆盖 URL 替换及缓存上传恢复；编辑器浏览器用例检查清理后撤销/重做无失效 URL。Rust 全文引用检查为静态链路证据。 |
| E3 / P2 | 切文确认打开时立即清理计时器，自动保存同时检查局部确认和壳级暂停状态；放弃不会被等待期间的自动保存架空。 | [EditorPage.svelte](../src/features/editor/EditorPage.svelte)、[AppShell.svelte](../src/app/AppShell.svelte) | 编辑器浏览器用例“切文章确认期间暂停自动保存，放弃只保留磁盘内容”。 |
| E4 / P2 | 每次成功保存都更新实际落盘基线；等待期间的新输入保留 dirty，丢弃只回到最近成功保存内容；旧文档实例的回执不生效。 | [EditorSessionStore.ts](../src/features/editor/EditorSessionStore.ts) | Store 测试覆盖保存中输入、重复保存串行、按最近基线丢弃、重开同文旧回执、保存中撤销回原内容及外部更改后的失败恢复。 |
| E5 / P2 | 文件名持续跟随完整标题，用户主动编辑文件名后停止自动跟随；`datetime-local` 使用本地年月日时分。 | [EditorPage.svelte](../src/features/editor/EditorPage.svelte) | 编辑器浏览器用例“新建文件名跟随完整标题且日期使用本机年月日时分”。 |
| E6 / P2 | 单独保存读取失败的目标，重试继续请求该文章，不从仍激活的旧文章推断。 | [EditorPage.svelte](../src/features/editor/EditorPage.svelte) | 编辑器浏览器用例确认失败后两次请求均为目标 `tauri`。 |
| E7 / P2 | 草稿/文章互转保留嵌套目录，同名资源目录随 Markdown 一起移动；先检查两个目标冲突，后续移动失败尝试回退资源目录并报告回退失败。 | [article_move.rs](../src-tauri/src/engine/article_move.rs)、[project.rs](../src-tauri/src/commands/project.rs) | Rust `moves_nested_article_and_assets_in_both_directions`、`asset_conflict_does_not_partially_move_article` 使用隔离文件验证双向移动及冲突不产生部分移动。 |
| E8 / P2 | 真实主题预览入口统一等待当前文档保存，再获取当前文章 URL 并打开/刷新预览；失败有通知，忙碌时防止并发启动。 | [EditorPage.svelte](../src/features/editor/EditorPage.svelte)、[AppShell.svelte](../src/app/AppShell.svelte)、[HexoThemePreview.svelte](../src/features/editor/preview/HexoThemePreview.svelte) | 编辑器浏览器用例记录保存调用先于打开真实预览。测试使用 mock IPC，未据此声称真实 Hexo 主题生成或原生 WebView 全链通过。 |

### 同步、设置与更新：S01–S13

| 编号 | 已核对的修复行为 | 实现位置 | 回归证据与边界 |
|---|---|---|---|
| S01 / P1 | 每次延迟同步有独立 schedule ID，旧任务只清理自己的记录，不会移除最新 sender；关闭同步移除待运行调度。 | [sync.rs](../src-tauri/src/commands/sync.rs)、[state.rs](../src-tauri/src/app/state.rs) | [sync_regression_tests.rs](../src-tauri/src/commands/sync_regression_tests.rs) 的 `obsolete_schedule_cleanup_keeps_the_latest_receiver_alive` 检查替换、旧清理和新 receiver 存活。 |
| S02 / P1 | 首次远端初始化与覆盖操作通过清单 schema 选择删除基线；旧版文章清单不将未纳入范围的本地配置、主题等解释为远端删除。 | [sync.rs](../src-tauri/src/commands/sync.rs) 的 `overwrite_base_for_manifest` 及远端应用分支 | 同一 Rust 回归文件的 `legacy_initialization_preserves_config_theme_and_drafts` 验证文章更新、配置/主题/清单外本地草稿保留。这里的“保留草稿”是该 fixture 的结果，不表示所有清单版本均排除草稿。 |
| S03 / P1 | 快照持有不可变文件字节，hash、缓存复制及上传使用同一份内容；同步期间新保存进入后续待同步状态。应用远端前比较当前文件与决策快照，拒绝覆盖之后保存的内容，并通过项目文件锁协调写入。 | [sync.rs](../src-tauri/src/commands/sync.rs)、[project.rs](../src-tauri/src/commands/project.rs)、[state.rs](../src-tauri/src/app/state.rs) | `upload_uses_captured_bytes_after_the_editor_saves_again` 验证字节/hash 一致；`remote_apply_rejects_changes_saved_after_the_decision_snapshot` 验证晚保存不被覆盖。最终同步收尾与原子提交窗口由负责同步的实现检查确认后完成发布 gate。 |
| S04 / P1 | 注册表变更在统一写锁内基于最新文件按项目合并；使用读取基线检测同项目过期写入，旧任务不能恢复已关闭的记录，也不会覆盖其他项目的新状态。 | [sync.rs](../src-tauri/src/commands/sync.rs) 的注册表加载/保存；[state.rs](../src-tauri/src/app/state.rs) | `registry_merges_projects_and_rejects_a_stale_reenable` 验证 A/B 交错保存均保留及旧副本不能重新启用已删除记录。 |
| S05 / P1 | 同步重扫携带起始项目 ID、generation 与 root，在会话提交前复核；同步状态事件携带项目身份，前端忽略不属于当前会话的结果。 | [project.rs](../src-tauri/src/commands/project.rs)、[sync.rs](../src-tauri/src/commands/sync.rs)、[AppShell.svelte](../src/app/AppShell.svelte)、[SettingsPage.svelte](../src/features/settings/SettingsPage.svelte) | `old_project_rescan_cannot_replace_the_active_project` 拒绝 ID/root 不匹配的旧重扫并验证当前会话保持不变；前端 A01 测试覆盖晚返回与 generation。最终收尾取消/重扫窗口需纳入 Rust 全量检查。 |
| S06 / P2 | GitHub 与 WebDAV 共用远端领先、首次方向选择、覆盖确认和逐项冲突决策界面；每个冲突默认未选择，提交前必须明确决定。 | [SettingsPage.svelte](../src/features/settings/SettingsPage.svelte) | [SettingsPage.test.ts](../src/features/settings/SettingsPage.test.ts) 验证 WebDAV 冲突逐项选择、调用 guard，以及远端领先的两个操作入口。 |
| S07 / P2 | 冲突面板提供重新检查，重新拉取冲突后清空旧选择；失败原因保留在面板，用户可以恢复流程。 | [SettingsPage.svelte](../src/features/settings/SettingsPage.svelte) | 同一组件测试确认“重新检查冲突”入口可达，代码链确认重新运行同步并重建冲突。远端 head 再变化后的真实服务往返未在此组件用例中模拟。 |
| S08 / P2 | 每次主题选择都立即预览选中值，选回已保存值也恢复；取消恢复保存主题和语言。 | [SettingsPage.svelte](../src/features/settings/SettingsPage.svelte) | 组件测试“previews the saved theme when the selection returns to its original value”验证 light→dark→light 和无残留 dirty。 |
| S09 / P2 | 保存只更新实际提交成功的基线，通过字段合并保留保存期间的新输入；保存 guard 等待在途保存，仍 dirty 则要求再次保存。 | [settingsDraft.ts](../src/features/settings/settingsDraft.ts)、[SettingsPage.svelte](../src/features/settings/SettingsPage.svelte)、[controller.ts](../src/features/settings/controller.ts) | [settingsDraft.test.ts](../src/features/settings/settingsDraft.test.ts) 验证字段合并；组件受控 Promise 验证保存中字号继续输入保留并显示未完成提示。 |
| S10 / P2 | 获取 Token 只持久化所需图床连接字段；其他分类草稿保持 dirty。Token 删除等凭据动作不隐式全量保存设置。 | [SettingsPage.svelte](../src/features/settings/SettingsPage.svelte) 的 `persistImageBed` 和凭据操作 | helper 测试验证连接 patch 保留无关草稿；组件测试验证准备 Token 时字号未被提交且保存按钮仍存在。 |
| S11 / P2 | 鼠标保存有统一错误处理和持久提示，失败保留草稿；数字配置提交前按实际范围验证并切换到相应分类、聚焦字段。 | [settingsDraft.ts](../src/features/settings/settingsDraft.ts)、[SettingsPage.svelte](../src/features/settings/SettingsPage.svelte) | helper 测试验证数值范围、字段和分类；组件验证字号 40 不调用保存、显示 12–28 提示并聚焦。磁盘写入拒绝处理链已静态核对，需由最终针对性执行结果补齐。 |
| S12 / P2 | 下载前再次检查返回无更新时写入 `upToDate`，结束 downloading；更新检查/下载/安装串行，避免并发状态互相覆盖。 | [update.rs](../src-tauri/src/commands/update.rs)、[AboutPage.svelte](../src/features/about/AboutPage.svelte)、[state.rs](../src-tauri/src/app/state.rs) | 已核对 `None` 的显式终态分支和操作锁；本文建立时没有新增直接驱动原生 updater 的此分支测试，不将 ViewModel 单测替代原生下载验收。 |
| S13 / P2 | 安装失败仍可从 error/install 状态重试缓存包；检查与下载不会清理已有待安装缓存；关于页呈现持续错误和安装操作。 | [updateViewModel.ts](../src/features/update/updateViewModel.ts)、[AboutPage.svelte](../src/features/about/AboutPage.svelte)、[update.rs](../src-tauri/src/commands/update.rs) | [updateViewModel.test.ts](../src/features/update/updateViewModel.test.ts) 覆盖安装失败可重试；后端缓存保留链静态核对。真实签名包的安装与重试由发布/原生验证记录证明。 |

### 图片与插件：IMG-01–IMG-07、PLG-01–PLG-03

| 编号 | 已核对的修复行为 | 实现位置 | 回归证据与边界 |
|---|---|---|---|
| IMG-01 / P2 | 本地导入返回显式 `canceled`，取消不覆盖列表、不显示成功。 | [images.rs](../src-tauri/src/commands/images.rs)、[ImageBedPage.svelte](../src/features/image-bed/ImageBedPage.svelte)、[app.ts](../src/shared/types/app.ts) | [ImageBedPage.test.ts](../src/features/image-bed/ImageBedPage.test.ts) 验证取消保留原图片且无成功通知。 |
| IMG-02 / P2 | 删除后重新查询；当前 offset 超出总数或末页为空时回到最近有效页。 | [ImageBedPage.svelte](../src/features/image-bed/ImageBedPage.svelte) | 同一组件测试验证第二页删空后返回有效页。 |
| IMG-03 / P2 | 查询携带请求序号和会话/来源身份；结果、错误及 loading 收尾都只由当前请求更新。 | [ImageBedPage.svelte](../src/features/image-bed/ImageBedPage.svelte) | 同一组件测试受控乱序完成，旧搜索不能覆盖新结果。 |
| IMG-04 / P2 | 重命名统一提交入口，函数开始同步检查 `working`，等待期间阻止重复提交与取消。 | [ImageBedPage.svelte](../src/features/image-bed/ImageBedPage.svelte)、[ModalDialog.svelte](../src/shared/components/ModalDialog.svelte) | 同一组件测试验证 Enter 只发一次请求，并在 mutation 等待中不可取消。 |
| IMG-05 / P2 | 删除开始捕获固定目标及会话，回包按捕获 ID 更新列表；在途操作不因 Escape 丢失目标。 | [ImageBedPage.svelte](../src/features/image-bed/ImageBedPage.svelte) | 同一组件测试验证删除期间 Escape 后成功结果仍移除正确资源。 |
| IMG-06 / P2 | 批量导入逐项收集失败并返回成功数和当前列表；重试相同文件时识别已有相同字节，避免重复导入已成功项。 | [image_local.rs](../src-tauri/src/commands/image_local.rs)、[images.rs](../src-tauri/src/commands/images.rs)、[ImageBedPage.svelte](../src/features/image-bed/ImageBedPage.svelte) | Rust `partial_import_reports_each_failure_and_retry_does_not_duplicate_successes` 验证部分失败及重试；图片组件测试验证成功项可见并说明失败文件。 |
| IMG-07 / P2 | 图床统一 HTTP client 设置 10 秒连接、60 秒总请求期限；读取和 mutation 超时区分提示，写操作超时提示先刷新核对结果，避免盲目重复。 | [client.rs](../src-tauri/src/platform/cloudflare_imgbed/client.rs)、[images.rs](../src-tauri/src/commands/images.rs)、[imgbed_auth.rs](../src-tauri/src/commands/imgbed_auth.rs) | Rust `stalled_requests_have_a_finite_budget_and_mutation_timeout_guidance` 使用回环停滞服务验证超时及结果未确认提示，测试把超时缩短为 50ms；未请求真实图床。 |
| PLG-01 / P2 | schema 默认值先实体化进入设置对象；提交前验证必填、类型、枚举和数值/长度范围，缺项不保存。 | [settings.ts](../src/shared/plugins/settings.ts)、[PluginManagerPage.svelte](../src/features/plugins/PluginManagerPage.svelte) | [settings.test.ts](../src/shared/plugins/settings.test.ts) 验证 false/0/显式值及验证规则；[PluginManagerPage.test.ts](../src/features/plugins/PluginManagerPage.test.ts) 验证所见默认值真实保存、必填缺失阻止提交。 |
| PLG-02 / P2 | Worker 超时/崩溃终止时拒绝所有待处理请求并清除运行实例，下一次调用重新创建；插件禁用、卸载和生命周期结束清理 Worker。 | [PluginWorkerRuntime.ts](../src/shared/plugins/PluginWorkerRuntime.ts)、[PluginProviderRuntime.ts](../src/shared/plugins/PluginProviderRuntime.ts)、[PluginManagerPage.svelte](../src/features/plugins/PluginManagerPage.svelte)、[AppShell.svelte](../src/app/AppShell.svelte) | [PluginWorkerRuntime.test.ts](../src/shared/plugins/PluginWorkerRuntime.test.ts) 验证超时后重建、禁用终止、崩溃拒绝并行请求；使用受控 Worker，不声称真实插件服务往返完成。 |
| PLG-03 / P2 | 卸载先确认，默认保留设置，可显式选择删除；失败可见。重装恢复保留设置并默认禁用，安装使用暂存目录避免半安装状态。 | [PluginManagerPage.svelte](../src/features/plugins/PluginManagerPage.svelte)、[host_commands.rs](../src-tauri/src/plugins/host_commands.rs)、[package.rs](../src-tauri/src/plugins/package.rs) | 插件组件测试验证确认与默认保留设置；Rust `uninstall_preserves_settings_and_reinstall_starts_disabled` 验证保留/删除及重装状态。 |

## 本轮交互优化落点

1. **文章和项目身份贯穿异步流程。** 重扫、图片插入、保存回执都有明确归属；远端变化成为用户能处理的状态，本地输入保留到用户决定。
2. **同步方向与影响可见。** GitHub/WebDAV 使用同一冲突决策，首次选择与覆盖操作有确认；未经选择的冲突不能默认为本地。后台使用固定快照和按项目提交，前台的“同步完成”与实际状态保持一致。
3. **设置采用保存基线与草稿分离。** 主题/语言是即时预览，取消恢复；Token 只作用于连接；保存中继续输入仍是未保存内容。
4. **模态和异步操作有一致的键盘规则。** 栈顶拥有键盘处理权，IME 不误提交，mutation 同步防重；等待期间不会让取消按钮形成“看似取消、实际仍执行”的状态。
5. **图片与插件失败可恢复。** 取消、部分成功、失败、超时分别反馈；分页和搜索保持查询一致性；Worker 重试创建新实例，卸载保留设置是默认选择。
6. **日常写作入口保持一致。** 标题与文件名跟随明确、时间使用本地值、重试找回失败目标、文章资源随互转移动，真实预览先保存当前内容。

原审计中的完整备份管理器、图库全项目引用分析、可视化发布前变更清单等扩展方案，不因存在本次修复映射而自动视为已经开发；本文只对实际列出的实现和证据作出说明。

### 扩展设计方案与本轮实施边界

以下对应审计中的六组产品方案及图片/插件补充建议。36 项缺陷的实际修改以上表为准；本表保留尚未实施的产品增强方案，不以建议代替修复，也不把建议宣传为已有功能。

| 设计主题 | 本轮已实施或保留 | 后续增强的具体改法与验收 |
|---|---|---|
| 新建与预览 | 文件名只在未手改时跟随完整标题；采用本地时间；统一当前文章保存后打开真实预览。 | 将日期、标签、分类放入可展开区域，并显示日期采用的时区；折叠不丢输入，手改文件名不再被标题覆盖。配置独立博客时区前不要声称已按博客时区转换。 |
| 同步状态、比较与备份 | 底栏使用可读状态，冲突/远端领先可进入设置；两种 provider 共用冲突决策、首次方向与覆盖确认；保留预检统计和打开备份目录。 | 提交确认前按当前快照列出新增/覆盖/删除文件；远端变化使确认失效。增加备份列表，展示时间、项目、范围和恢复预览；恢复失败保留原文件与备份。二进制比较优先显示名称、大小、时间，完整 hash 可展开。 |
| 图库位置、搜索与图片引用 | 取消保留列表，部分成功分别反馈，重试避免重复导入相同成功项，删除回有效页，旧查询不覆盖新查询。标明浏览来源与插件仅用于编辑器上传。 | 建立文章图片引用索引，重命名/移动/删除前展示受影响文章；本地移动可选择同步更新引用，远端不承诺服务端旧链接仍有效。远程空态使用已提交的查询，明确递归范围；加载期间保留来源、目录和查询说明。验收覆盖未提交关键词、多文章引用和未保存正文。 |
| 插件表单与生命周期 | boolean/enum/number/integer/string 控件、默认值、必填和范围校验；不支持的类型明确提示并保留原值；超时后重建，卸载可保留设置，暂存安装。 | 插件列表进一步显示故障/API 不兼容条目及修复动作，不能仅隐藏加载失败目录；权限显示中文能力说明。验收包括坏 manifest、版本不兼容、禁用后重启与权限变化。 |
| 任务、错误和大文件下载 | 保留低干扰摘要，主动打开任务详情并取消任务；任务运行期间阻止直接切项目/退出；图床采用有限超时并区分写操作结果未确认。 | 将远程下载改为直接流式保存，提供保存位置、字节进度、真实取消和完成结果；取消清理临时文件，失败可重试，避免 Rust 与 JS 多份全量字节复制。任务错误继续细分生成失败、部署结果未确认、部署完成但推送失败，以实际执行阶段显示下一步。 |
| 语言和可访问性 | 显式响应式翻译保留用户正文；设置控件关联名称和说明；数值错误可见并聚焦。 | 抽取统一字段组件，把错误通过 `aria-describedby` 与 `aria-invalid` 关联到各控件；补真实屏幕阅读器与 200% 缩放检查，覆盖切换语言后的名称、值、说明和错误。该验收不能由 Svelte 零警告或普通截图替代。 |

恢复交互补强已进入当前实现：[documentRecovery.ts](../src/app/documentRecovery.ts) 将决定绑定到已展示的项目、文档实例、revision 与外部变更类型；远端比较内容变化时要求重新确认。另存恢复稿记住创建结果，失败重试复用同一文件，并在最终读取核验成功前保留原编辑器内容。[documentRecovery.test.ts](../src/app/documentRecovery.test.ts) 覆盖 R1→R2、首次读取失败与空远端、身份/revision 变化、草稿失败重试及保存后被外部改写；原生云端往返仍按下方边界说明。

同步收尾补强的测试源已核对：`rescan_preserves_surviving_article_identity_and_revision_with_real_files` 保留文章 ID/revision，`remote_apply_invalidates_old_saves_before_unlocking_and_keeps_ui_transition` 验证解锁前失效旧保存与前端重扫事件，`failed_remote_rescan_rolls_back_disk_and_leaves_current_session_usable` 验证失败回退，`registry_updates_baseline_after_merge_and_surfaces_persistence_failure` 验证注册表失败不报同步成功。

## 最终检查与发布记录

本地检查、最终提交 CI、原生打包和发布后下载验证均已完成。发布资产来自同一最终修复提交；本记录后续只补充验证结果，没有移动已公开发布的标签。

| 检查项 | 当前结果 | 最终证据 |
|---|---|---|
| 修复提交 SHA | `fc01b5e92f3bc869289a0e137a783a11447e7657` | 主修复提交为 `556dc6a3bae1af4a9f32d61a96f90b1c182c8067`，最终提交另外修正图库焦点时序并预热 CI 页面；后续文档提交仅补充验证证据。 |
| `pnpm check` | 通过：0 errors、0 warnings，退出码 0 | `output/fix/frontend-check-final.log` 与焦点补丁后 `frontend-check-candidate2.log`；最终提交三个 CI 前端检查均通过。 |
| `pnpm test` | 通过：24 个文件、118 个用例，退出码 0 | `output/fix/frontend-full-test-final.log`，包括恢复流程回归。 |
| `pnpm test:e2e` | 通过：35 / 35，退出码 0 | 本地最终 `output/fix/e2e-candidate2.log`，最终提交 Linux 与 Windows CI 均 35/35；浏览器 demo/mock IPC，不是原生全链。 |
| `pnpm build` | 通过，退出码 0 | 本地 `output/fix/frontend-build-candidate2.log` 与三个 CI；生成前端 `build/`。 |
| `pnpm audit --prod` | 通过：No known vulnerabilities，退出码 0 | `output/fix/dependency-audit-final.log`。 |
| `cargo fmt --check` | 通过，退出码 0 | 本地及最终提交 Linux、Windows、macOS CI 均通过。 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 通过，退出码 0 | 同步最终修改后的本地严格 Clippy 命令执行结果。 |
| `cargo test --all-targets --all-features` | 通过，退出码 0；Rust 报告 94 passed | 其中 3 个真实 WebDAV 测试因缺环境内部提前返回，实际执行业务断言为 91 个；不将其计为联网验证。 |
| Linux CI | 通过 | [run 34490551834](https://github.com/Bai-YB/hexo-lite-editor/actions/runs/34490551834)，前端 118、浏览器 35、Rust 93；两个作业均 success。 |
| Windows 构建、便携包/安装包 smoke | 通过 | [run 34490551705](https://github.com/Bai-YB/hexo-lite-editor/actions/runs/34490551705)，success；Rust 94。便携包启动成功、版本 1.0.6；NSIS 安装/启动与 MSI 管理提取成功，生成 Setup EXE、MSI、便携 ZIP。 |
| macOS universal 构建 | 通过 | [run 34490551807](https://github.com/Bai-YB/hexo-lite-editor/actions/runs/34490551807)，success；Rust 93。实际架构 `x86_64 arm64`，生成 DMG、App ZIP、更新 tar.gz 和签名；未执行 macOS 人工运行。 |
| 新标签与原标签 | 已核对远端，原发布保持不变 | `v1.0.6-r1` 标签对象 `f30e9eb20bcea310fdc118cac470adcd37f98f07` 指向最终提交；首轮未公开候选被替换。原 `v1.0.6` 标签对象仍为 `0b4787aa6fb62dfc2dae08881c1299bbb3dd0dfc`，指向 `6a4dd516edab0d294c8b8f6b283774ba21d72762`。 |
| 资产哈希与签名 | 全部通过 | 独立下载目录 `output/release-verify/v1.0.6-r1` 的 14 个资产与 GitHub digest 一致，12 条 SHA-256 全部匹配；Windows EXE/MSI 与 macOS 更新包使用应用内公钥验证签名通过。两份 release-manifest 的 sourceCommit 均为最终提交。 |
| 更新清单 | 全部通过 | 下载的 `latest.json` 包含 `windows-x86_64`、`darwin-x86_64`、`darwin-aarch64`，版本 1.0.6；签名与资产一致，所有下载 URL 精确指向 `v1.0.6-r1`。 |
| GitHub Release | 已公开，latest | [v1.0.6-r1](https://github.com/Bai-YB/hexo-lite-editor/releases/tag/v1.0.6-r1)，`isDraft=false`、`isPrerelease=false`；发布时间 2026-09-10 15:03:40 UTC（北京时间 23:03:40）。 |

### 验证边界

- Svelte 组件测试、Store/CodeMirror 测试和浏览器 demo 可以证明受控竞态与交互行为，不能替代 Tauri 原生对话框、真实 Hexo 主题、云服务、系统凭据或安装器验证。
- Rust 回归以隔离项目、内存状态和回环服务验证删除范围、文件字节、注册表合并及超时，不操作用户真实博客或外部图床数据。
- 真实 GitHub/WebDAV 同步往返、原生更新的下载撤回/安装失败恢复、macOS 手工运行、屏幕阅读器验收若没有本轮独立证据，应继续注明未执行。
- 本轮发布为同版本修复构建，不能以“检查更新未提示新版本”判断安装包未更新；应核对发布标签、文件哈希和提交来源。
- 更新包的 Tauri 签名已验证；Windows 无商业代码签名，macOS 未配置 Apple 代码签名且未公证，二者与更新包签名是不同验证。
