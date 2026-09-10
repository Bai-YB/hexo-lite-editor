# Hexo Lite Editor 代码审计与交互优化方案

审计日期：2026-09-10。基线：v1.0.6，提交 `25c4b9460ce034c77d2ce0d3617127417261565f`。审计开始时 Git 工作区干净。

本次完成项目源码审阅、调用链核查、现有测试执行及针对性验证。共记录 **36 项问题：9 项 P1、27 项 P2**，另提供 6 组产品交互优化方案。P1 优先处理数据一致性、错项目/错文章操作和关键控制失效；P2 处理局部流程受阻、状态错误和可恢复体验问题。没有将所有问题都定为阻断，也没有把尚未开发的功能计为缺陷。

本报告记录**修复前的审计基线**；文中的“未修改”“未实施”及测试结果均指该审计阶段。后续修复实现、交互优化落实情况和发布验证见 [1.0.6 修复构建验证记录](fix-validation-1.0.6-r1.md)。本报告保留原始触发条件、问题证据和修改建议，便于对照复核。

审计阶段的证据脚本只使用本地 demo、模拟 IPC/Worker、受控异步响应和隔离文件，没有对真实博客执行发布、图床删除、云端覆盖、凭据修改或更新安装。

## 应优先处理的结论

| 优先 | 编号 | 问题与后果 | 验证程度 |
|---|---|---|---|
| P1 | S02 | 首次使用旧版同步清单，完整本地配置和主题被当成远端删除项 | 提取原函数，隔离文件验证 |
| P1 | S05 | A 同步期间切到 B，A 完成后可能把 B 的项目 ID 绑定到 A 目录 | 前后端完整静态调用链 |
| P1 | A01 | 同步重载晚返回直接覆盖刚输入的正文，或恢复已离开的文档 | 提取原函数 + 真实 Store 验证 |
| P1 | S03 | 同步时继续保存，上传字节和清单 hash 不一致，远端后续读取失败 | 隔离时序算法验证 |
| P1 | S01 | 30 秒内连续保存，旧任务清理误取消最新自动同步 | 真实 Tokio 调度验证 |
| P1 | S04 | 后台同步旧注册表写回，可能恢复刚关闭的同步或覆盖其他项目设置 | 静态调用链 |
| P1 | E1 | 图片导入未完成时切文章，结果插到另一篇文章 | 浏览器受控异步验证 |
| P1 | E2 | 图片上传完成后撤销，恢复已经清理的临时图片链接 | 浏览器 + 后台清理链 |
| P1 | A04 | 新建弹窗打开时发布快捷键穿透，后台开始发布 | 浏览器 demo 验证 |

## 范围与测试结论

按 13 个模块覆盖应用壳、编辑器、Markdown/预览、图片、设置/国际化、插件、更新、平台适配、Tauri 状态、文章后台、Hexo 任务、同步及构建测试。团队完整阅读 `src` 和 `src-tauri/src` 下 113 个源码/测试/样式文件，并核对路由 helper、构建配置、脚本、插件示例和文档。40 份模块文档完整阅读；13 份历史/未来需求按章节与相关上下文核对，未声称逐字读完约 12,000 行历史需求。第三方依赖、锁文件依赖内容、生成产物和二进制资源不作为自有业务源码审计。

| 检查 | 本次结果 | 实际证明范围 |
|---|---|---|
| `pnpm check` | 0 errors、0 warnings | Svelte/TS 静态诊断 |
| `pnpm test` | 13 文件、56 用例通过 | 现有前端/脚本单测 |
| `cargo test --lib` | 79 用例通过 | Rust 现有单测；部分真实环境用例内部有条件跳过，未据此声称联网集成已验证 |
| `pnpm test:e2e` | 28 用例通过 | Edge/Chromium 上的浏览器 demo；不是完整 Tauri 原生端到端 |
| 图片/插件审计探针 | 7 / 7 断言确认现状缺陷 | 真实 Svelte 组件，模拟 IPC/Worker |
| 编辑器与应用壳补充 | 浏览器和 Node 受控验证 | 切文、输入、历史、语言、弹窗及重扫边界 |
| 同步/设置补充 | 5 个确定性逻辑场景验证 | Tokio 调度、文件范围、hash、表单状态 |

现有测试全绿并不覆盖本报告的边界情况。验证用的“探针通过”表示**成功确认缺陷**。原生关闭、真实 GitHub/WebDAV 往返、真实插件服务、真实 Hexo 主题生成、签名升级安装和 macOS 运行未在本次端到端验证；相应结论标注为静态或算法验证。未把昨日验收文档的结果当作本次运行结果。

实现的基础是连贯的：页面分区、浅深色 token、原子写文件、已有文章加载序号、退出未保存保护、远程应用备份、发布固定 clean→generate→deploy 都值得保留。问题集中于这些保护没有贯穿全部入口和异步生命周期。

辅助 UI 审计（0–4，仅当前桌面范围）：可访问性 2、性能 3、桌面尺寸适配 3、主题 2、交互实现一致性 2，共 12/20。该分数不是 WCAG 认证或性能基准。1120×720 桌面布局有实际截图支持，但不据此声称 200% 缩放/屏幕阅读器或移动端通过。机械设计扫描唯一提示是 `editor.css:444` 的 blockquote 左边线，这是正文引用语义，判定为误报，不建议删除。

## 全部问题索引

| 编号 | 级别 | 问题 | 主要代码位置 |
|---|---|---|---|
| A01 | P1 | 同步重载覆盖当前编辑/保留已删除文章 | AppShell.svelte:312 |
| A02 | P2 | 英文界面误翻译用户的文章预览 | legacyDom.ts:52 |
| A03 | P2 | 语言切回中文后旧控件仍保留英文 | legacyDom.ts:80 |
| A04 | P1 | 模态弹窗不阻挡全局发布/导航快捷键 | AppShell.svelte:222 |
| A05 | P2 | 设置开关和数值控件没有可访问名称 | SettingsPage.svelte:534 |
| E1 | P1 | 异步导入图片插入错文档 | EditorPage.svelte:451 |
| E2 | P1 | 撤销恢复已删除的图片缓存引用 | MarkdownEditor.svelte:265 |
| E3 | P2 | 切文确认时自动保存使放弃失效 | EditorPage.svelte:272 |
| E4 | P2 | 保存中输入后的丢弃退回过旧基线 | EditorSessionStore.ts:193 |
| E5 | P2 | 新建文件名跟随错误、日期使用 UTC | EditorPage.svelte:389 |
| E6 | P2 | 读取失败重试错误文章 | EditorPage.svelte:1066 |
| E7 | P2 | 草稿互转遗漏文章资源目录 | project.rs:529 |
| E8 | P2 | 两个主题预览入口保存/刷新行为不同 | EditorPage.svelte:661 |
| S01 | P1 | 连续保存取消最新同步调度 | sync.rs:1203 |
| S02 | P1 | 旧版清单初始化删除配置主题 | sync.rs:1460 |
| S03 | P1 | 上传内容与清单 hash 不同版本 | sync.rs:1349 |
| S04 | P1 | 同步注册表旧副本覆盖新设置 | sync.rs:1319 |
| S05 | P1 | 旧同步结果替换当前项目上下文 | project.rs:661 |
| S06 | P2 | WebDAV 冲突/远端更新没有解决入口 | SettingsPage.svelte:620 |
| S07 | P2 | 冲突远端再变化后没有刷新出口 | SettingsPage.svelte:648 |
| S08 | P2 | 主题选回原值不恢复预览 | SettingsPage.svelte:360 |
| S09 | P2 | 保存回包抹掉新输入的设置 | SettingsPage.svelte:367 |
| S10 | P2 | Token 动作顺带保存其他分类草稿 | SettingsPage.svelte:434 |
| S11 | P2 | 鼠标保存失败无界面反馈 | SettingsPage.svelte:367 |
| S12 | P2 | 更新撤回时下载状态不结束 | update.rs:174 |
| S13 | P2 | 安装失败无法重试已缓存包 | AboutPage.svelte:58 |
| IMG-01 | P2 | 取消导入清空图库并误报成功 | ImageBedPage.svelte:257 |
| IMG-02 | P2 | 末页删空后分页入口消失 | ImageBedPage.svelte:558 |
| IMG-03 | P2 | 旧搜索结果覆盖新查询 | ImageBedPage.svelte:211 |
| IMG-04 | P2 | Enter 触发两次重命名请求 | ImageBedPage.svelte:598 |
| IMG-05 | P2 | 删除中取消令计数和列表不同步 | ImageBedPage.svelte:349 |
| IMG-06 | P2 | 批量导入部分失败被当成全部失败 | images.rs:59 |
| IMG-07 | P2 | 内置图床请求缺有限超时/取消 | cloudflare_imgbed/client.rs:14 |
| PLG-01 | P2 | 插件可见默认值没有进入提交数据 | PluginManagerPage.svelte:117 |
| PLG-02 | P2 | Worker 超时后继续复用终止实例 | PluginProviderRuntime.ts:49 |
| PLG-03 | P2 | 插件卸载直接删除配置且失败无反馈 | PluginManagerPage.svelte:99 |

S05 后端跨项目身份问题与 A01 前端晚响应问题是两个不同提交边界，分别修复，未重复计数。E5 为同一新建表单工作项，内部记录两个独立验收点。

## 应用壳与通用交互详情

### A01 / P1：同步重扫晚响应覆盖用户刚输入的正文

- 位置：`src/app/AppShell.svelte:312`，重点 323–326；`src/features/editor/EditorSessionStore.ts:47`。
- 触发：当前文章无修改，远端应用发出重扫，开始异步读取；读取尚未返回时继续输入或切文章/项目。
- 原因：只在请求之前判断 previousState.dirty，返回后无当前 projectId/generation/articleId/revision 校验，直接 `editorStore.load(snapshot)`。该方法重置正文并清 dirty。正常 openArticle 有加载序号和身份检查，此入口没有。
- 证据：`shell-state-repro.mjs` 提取生产 applyProjectRescan + 使用真实 Store：输入的内容被 remote version 替代，dirty=false；切到 project-b 后，壳仍为 B、editor snapshot 回到 A。另外远端删除当前文章时，列表已空而 Store 保留旧文章且显示已保存。
- 修改：以文档实例及请求版本捕获读取目标；回包时只有仍是该文档且 revision 未变才能替换。已有本地修改则显示“远端已更新”，提供比较/保留本地/应用远端，不能自动覆盖。远端删除当前文档时转入“已在远端删除”恢复态，可另存草稿或关闭，不伪装成已保存的正常文档。配合 S05 在后端锁内核对身份。
- 验收：重扫中输入、切文、切项目、重复重扫乱序、当前文章被删除均不会覆盖用户新操作；所有 dirty 与磁盘状态一致。证据只验证生产函数边界，未执行真实远端覆盖。

### A02 / P2：切成英文会改变文章预览正文

- 位置：`src/shared/i18n/legacyDom.ts:52`–64。文字遍历只排除 textarea、contenteditable、CodeMirror、code/pre；没有排除文章预览、文章名、资源名等用户数据。
- 浏览器证据：正文仍为“删除 / 保存 / 上传失败”，预览变成“Delete / Save / Upload failed”。截图 `output/playwright/audit-preview-translated.png`。
- 影响：源 Markdown 没被改写，但预览已经不忠实于待发布正文，文章名/文件名也可能被误展示，干扰用户核对。
- 修改：应用文本用显式翻译键渲染，移除扫描整页 DOM 的猜测翻译。过渡期只处理明确标记的 UI 文本，用户内容区域统一设置“不翻译”，属性翻译同样排除用户数据。
- 验收：任何语言下，正文、标题、文件名、标签和图片描述逐字保留；应用按钮与提示正确翻译，预览和原文一致。

### A03 / P2：语言切回中文不恢复已经替换的控件文字

- 位置：`src/shared/i18n/legacyDom.ts:80`–83、`src/app/AppShell.svelte:78`–81。
- 浏览器证据：中文→英文→中文后 document.lang=zh-CN，常规设置仍显示 Startup、Save & backup、Recent projects。停止 observer 只断开监听，没有还原 DOM；Svelte 静态文案不随 language 重建。
- 修改：所有界面文案直接订阅语言 store 并用翻译键；语言选择只做预览，取消恢复保存语言。不要用重新加载整个应用修复，避免丢失未保存文章。
- 验收：设置每个分类、打开的弹窗、通知和页面往返语言切换均即时恢复；保留表单与编辑器输入，不出现“设置saved”等混合文本。

### A04 / P1：新建/重命名等弹窗没有全局快捷键隔离

- 位置：`src/app/AppShell.svelte:222`–230 仅检查 guardAction；`src/shared/components/ModalDialog.svelte:33`–45 只处理 Escape/Enter/Tab，未建立统一模态状态。
- 浏览器证据：新建文章弹窗仍存在时按 Ctrl+Shift+P，demo 的 taskStarts 从不存在变成 1。仅在内存 demo 触发，未实际发布。
- 影响：用户处于输入/确认流程时可以误触后台发布；Ctrl+数字等还会切页销毁局部弹窗，输入丢失。与 IMG-04 的 Enter 双提交同属事件分层不清，但后果和修复入口不同。
- 修改：建立模态栈/统一快捷键路由，栈顶弹窗拥有键盘处理权；全局发布、新建、切项目/切页快捷键在模态内禁用或显式转交。使用 form submit，每个异步 mutation 同步检查 inFlight；忽略 IME composition，Escape 只影响栈顶。
- 验收：新建、重命名、删除、插件配置、获取 Token、未保存保护、自动更新弹窗均测试发布/导航快捷键；单次 Enter 只提交一次，中文候选确认不触发提交，关闭后焦点回原控件。

### A05 / P2：设置开关和数字输入缺少可访问名称

- 位置：`src/features/settings/SettingsPage.svelte:534`、538–540、549、553–562。
- 证据：浏览器可访问快照中常规设置三个 checkbox 没有名称；DOM 的 label 仅包裹 input 和空 span，真正的“自动保存”等文字是外部 div，未通过 for/id 或 aria-labelledby 关联。数值框同类。
- 影响：屏幕阅读器用户无法区分开关含义；点击说明文字也不能切换开关。类型检查零警告无法发现这类语义缺口。
- 修改：提取 SettingsField/Switch，使用稳定 id，把标题关联 label、说明关联 aria-describedby；选项组提供 group 名称，错误关联字段并设置 aria-invalid。保存状态用合适的 live region。
- 验收：键盘和可访问树能读出每个设置名称、当前值、说明与错误；点击标题可操作关联控件，所有语言下名称正确。对照 WCAG 1.3.1、3.3.2、4.1.2，仍需真实屏幕阅读器最终验证。

## 建议实施顺序与产品方案

### 第一批：先保证内容和项目不会错位

处理 S02/S05/A01/E1/E2/S03，随后 S01/S04。抽出统一项目/文档操作上下文：项目 ID、generation、文档 ID、文档实例、编辑 revision、操作 ID。异步开始捕获，返回前复核；UI 的忙碌禁用不能替代后台检查。同步上传使用不可变暂存版本，读取、hash、上传必须针对同一份字节；注册表提交基于最新配置做项目级 patch。

验收重点是慢请求中继续写作、切项目和重开同文档，不只是顺利保存。先落下能失败的回归测试，再分别修复。备份恢复应作为正常可见功能，覆盖确认展示新增/覆盖/删除范围和最后备份时间。

### 第二批：统一保存、取消、弹窗和错误恢复

处理 A04/E3/E4/S08–S11/IMG-04/IMG-05。编辑器和设置共用操作语义：保存中继续输入保留新 revision；“放弃”只放弃尚未提交内容；对不可取消的已发起操作明确显示“正在完成”或“关闭提示”，不暗示操作被撤回。表单使用一次提交入口和清晰错误，不让鼠标与快捷键走不同反馈。

### 第三批：补全可完成的同步、图库、插件与更新流程

处理 S06/S07、IMG-01–03/06–07、PLG-01–03、S12/S13。连接配置与通用同步决策区分开，实现 provider 无关的“等待 / 正在同步 / 需要选择 / 失败可重试 / 完成”。列表持有查询快照和当前页，取消不变，删空回最近有效页。插件显示的表单就是实际校验/保存对象；Worker 故障可重建。更新安装失败保留包和失败信息，提供直接重试。

### 第四批：六组值得做的交互优化

1. **新建文章和预览**：新建首屏保留标题、可预览的文件名、文章/草稿；日期、标签和分类可渐进展开。用户未手改文件名时才跟随标题；日期按明确的博客时区。两个真实预览入口统一“当前文章的保存→路由→打开/刷新”，维持已经允许的独立受限窗口，不强行改成 iframe。对应 E5/E6/E8。
2. **同步状态可理解、可操作**：底栏把 localPending、remoteAhead、conflict 等枚举改为“等待同步 / 云端有更新 / N 个文件需处理”，点击直达当前问题；不是对冲突状态重复自动同步。预检显示范围、增删改数量和备份；冲突列表突出具体差异，二进制显示大小/时间和可辨识信息，hash 放到详情。对应 S01–07。
3. **图库保持位置和操作结果**：显示当前来源、目录、查询和上传目标；搜索统一即时/提交规则或明确标识区别。取消选择保持列表；批量导入逐项展示结果，只重试失败项。删除/移动回最近有效页，保留查询和目录。图片引用分析作为后续增强，可提示受影响文章，不强行恢复需求明确移除的“插入当前文章”动作。对应 IMG 系列。
4. **插件设置与生命周期**：支持明确的 JSON Schema 字段类型（boolean/enum/number/string、required 和范围），保存前校验；默认值写入真实表单。区分已禁用、可用、故障、不兼容，显示恢复动作；卸载默认保留设置并说明默认图床如何迁移。插件无列表能力时说明“支持编辑器上传”，不把可选能力缺失当错误。对应 PLG 系列。
5. **任务和错误反馈**：保留现有低干扰底部摘要；加入用户主动打开的任务详情和“取消任务”入口，展示项目名、当前阶段、结果及下一步。切项目或关闭应用时若任务仍在运行，给出明确影响选择，避免仅依赖未保存判断就静默中断。错误信息区分生成失败、部署结果未知、部署成功但 Git push 失败，不一律声称“未上传”。这属于建议完善的行为，不是要求恢复自动弹日志窗。
6. **语言与可访问性**：移除 DOM 猜测翻译后，用统一字段组件补名称/说明/错误关联。双向切换语言和取消预览时保留所有草稿。保持浅深色风格和桌面密度，检查 1120×720、1360×860 及 200% 缩放；增大命中区而非无依据改大整套视觉。对应 A02/A03/A05/S08。

以上是可直接拆成开发任务的方案，不包含实施完成的声明。每项具体触发、改法和验收在下方详述；建议按以上四批处理，避免一边重构视觉一边引入新的数据风险。

## 本次不作为问题的产品约定

- 默认不自动展开日志，符合既有低干扰要求。
- 真实主题采用独立受限 WebView，属于需求允许的实现。
- 图库不提供“插入当前文章”、插件资源列表能力可选，符合当前范围。
- 管理员密码用于当次申请 Token 被较新的要求允许。
- v1.0.7 AI 写作和 v1.1 预览反向编辑是未来计划，未实现不算本次 bug。
- 移动端不是当前目标设备，不因固定桌面最小窗口本身扣缺陷。

## 模块详细发现

以下详情保留了各模块的复现依据、边界和逐项修改建议。路径均相对项目根目录；证据位于 `output/audit`，该目录被 Git 忽略，适合本次本机复核，需长期保留时另行归档。

## 编辑器与预览

审计日期：2026-09-10。只读业务代码；创建的脚本均在 `output/audit`。浏览器复现使用开发模式内存数据 `/?demo=1`，异步时序通过页面内替换 `platform` 方法控制，未上传图片、发布站点或修改真实博客。浏览器、开发服务器已关闭。

## 已验证问题

### E1 · P1 · 图片导入晚返回后插入错误文章

- 定位：`src/features/editor/EditorPage.svelte:451`、`:467`、`:474`、`:577`、`:593`；插件路径 `:435`、`:445`；上传计数直到 `:619` 才增加。
- 操作：在文章 A 粘贴图片，导入尚未完成时切到文章 B。导入结果随后插入 B。文件读取、导入，以及插件整个上传阶段都未锁定发起文档；插入采用结果返回时的当前文档、当前选区。新建文章路径 `:408–423` 也绕过普通切文的上传守卫。
- 证据：浏览器挂起 `importEditorImages`，从 `summer` 切到 `tauri` 再放行，输出 `activeArticle="tauri"`，内容末尾出现原本在 `summer` 粘贴的 `![audit-image.png](/images/audit-image.png)`。
- 影响：错误内容可被自动保存到另一篇文章；覆盖当前选区时可能替换 B 中用户选择的文字。跨项目结果还可能写入另一项目无法解析的本地 URL。后两项为同代码链静态推导，不声称已经修改真实文件。
- 调整：在读取文件前建立 `{projectId, generation, articleId, documentInstance, selectionBookmark}`；整个导入生命周期登记 busy；返回时校验令牌与组件存活。图片先作为带 ID 的占位插入发起文档，并映射选区；换文档后后台只更新该文档，或明确等待/取消导入。新建、切文、切项目共用此转换流程。
- 验收：分别挂起读文件、内置导入、插件上传，切文/新建/切项目后都不能污染目标文档；移动光标不改变发起插入位置；多图顺序稳定。

### E2 · P1 · 上传后的撤销恢复已删除缓存地址

- 定位：`src/features/editor/MarkdownEditor.svelte:265–271`；`src/features/editor/EditorPage.svelte:627–633`；`src-tauri/src/commands/images.rs:345–349`。
- 操作：插入图，等远程上传和缓存清理完成，按 Ctrl+Z。后台替换本地 URL 时，编辑器把整份外部内容当普通编辑写入撤销历史；撤销恢复到上传前临时 URL。
- 证据：`editor-image-undo-repro.js` 返回 `cacheFinalized="1"`，`beforeUndo` 含 `https://img.example.com/audit-remote.png`，`afterUndo` 含 `http://hlex-asset.localhost/0f5845c7-a9d8-40e9-97af-f770331f5000`。原生后台 finalize 会删除对应缓存和 asset token。
- 影响：文档留下失效图片引用，并可能继续自动保存。远端图片本体仍存在，不是图片文件永久丢失。
- 调整：区分用户编辑和系统 URL 更新，系统更新使用最小 URL 区间事务及 `Transaction.addToHistory.of(false)`，正确映射历史和选区；保留上传 ID→远程 URL 映射，恢复历史或重新打开文档时可修复旧地址。缓存清理应考虑历史/其他文档引用，不能只依赖一次保存完成。
- 验收：上传前后穿插文字编辑，连续 undo/redo 仅撤销用户意图；任何可回到的版本均不得含已不可解析的临时 URL；关闭再打开仍可显示图片。

### E3 · P2 · 切文章确认框中的“放弃更改”被自动保存架空

- 定位：`src/features/editor/EditorPage.svelte:272–276`、`:325–327`、`:346–360`。
- 操作：编辑 A 后在自动保存延迟内点击 B，停留在“保存当前文章？”确认框超过 2 秒，再点“放弃更改”。
- 证据：浏览器弹框时保存次数 0，等待 2400 ms 后为 1；点击放弃后回到 A，`AUDIT_DISCARD_SHOULD_REMOVE` 仍然存在。
- 原因：自动保存只检查外部 `autoSaveSuspended` 和删除框，没有检查 `showSwitchGuard` / `switchBusy`；保存完成还更新了 discard 基线。
- 影响：用户明确选择放弃，却已经写盘；配置了保存后同步时还可能触发后续同步（未调用真实同步验证）。
- 调整：打开确认框时暂停/清除自动保存，捕获确认时的基线和在途保存；统一退出框与切文框策略。若已开始不可取消保存，等待完成并准确显示已保存状态，避免继续展示失真的“放弃更改”。
- 验收：确认框停留多个自动保存周期时无新保存；取消后恢复自动保存；保存失败仍留原文；放弃仅丢弃未持久化编辑。

### E4 · P2 · 保存中继续输入后，discard 的内容和“已保存”状态不一致

- 定位：`src/features/editor/EditorSessionStore.ts:193–197`、`:156–164`。
- 操作：初始内容 `one`，改成 `two` 并开始保存；保存未返回时改成 `three`；`two` 保存成功；调用 discard。
- 证据：`node output/audit/editor-store-repro.mjs` 输出内容 `one`、`revision=1`、`persistedRevision=1`、`dirty=false`，但已接受保存的内容应该是 `two`。
- 原因：仅当前 revision 等于回执 revision 时才更新 `lastSavedContent`，而 persistedRevision 总会前进。
- 影响：直接 Store 路径已验证。通常切文成功会立即重新读取目标，因此不一定造成真实数据损失；若后续导航失败/取消并留在当前文档，界面会把旧内容当已保存，后续编辑可能将较新的磁盘内容覆盖。
- 调整：每个被接受的保存都推进持久化内容基线；只有回执对应当前内容才清 dirty。保存/丢弃维护文档实例令牌和单调 revision，避免同一文章重新 load 后旧回执修改新会话。
- 验收：上面的时序 discard 必须返回 `two`；分别测试晚成功、晚失败、切项目和重新打开同一篇文章。

### E5 · P2 · 新建文章文件名只取首次输入，日期按 UTC 错填本地时间

- 定位：`src/features/editor/EditorPage.svelte:396–398`、`:389`、`:415`、`:1199`；日期写入 `src-tauri/src/commands/project.rs:281–284`。
- 证据：逐字输入 `hello-world`，标题最终为完整值，文件名为 `h`。本地 `2026-09-10 17:16 GMT+0800` 时，新建框默认日期为 `2026-09-10T09:16`，保存会去掉 `T` 后原样写入 YAML。
- 影响：用户未经注意会产生首字母文件名、后续同名冲突；日期差 8 小时，凌晨新建还可能落到前一天并改变按日期的路由/排序。
- 调整：文件名增加 `fileNameManuallyEdited`；在用户未手改前随标题完整更新，清空后提供显式“由标题生成”。`datetime-local` 用本地年月日时分格式化，并显示博客时区；若博客时区和系统不一致，应按明确约定转换。
- 验收：逐字输入、中文输入法、粘贴、多次修改标题、手改文件名均符合预期；UTC+8/UTC-7 和跨日边界默认日期正确。

### E6 · P2 · 读取失败的“重试”打开上一篇文章

- 定位：`src/features/editor/EditorPage.svelte:284`、`:302`、`:312–315`、`:1066`。
- 操作：当前 A，加载 B 失败后点击“重试”。
- 证据：浏览器强制 `tauri` 读取失败，重试的调用列表为 `["tauri", "summer"]`，最终仍选中 `summer`。
- 原因：失败目标没有留存；activeArticleId 仅在成功后变更，“重试”却按该旧 ID 找文章。首次打开失败时 activeArticleId 为空，按钮完全无效果。
- 调整：单独持有 `requestedArticleId` / `failedLoadTarget`；失败面板写明目标并提供重试目标、返回原文两个动作；目标已删除时刷新列表并解释原因。
- 验收：首次读取失败、A→B 失败、失败目标被删除、连续切文晚响应均重试正确目标。

## 静态确认的高价值设计改进

### E7 · P2 · 草稿互转不移动 Hexo 文章资源目录

- 定位：`src-tauri/src/commands/project.rs:529–542`；相关本地图片查找 `src-tauri/src/commands/preview_images.rs:323–332`。
- 条件：标准 `post_asset_folder` 工作流，例如 `_drafts/foo.md` 和 `_drafts/foo/photo.png`。
- 行为：互转仅 `fs::rename` Markdown 文件到 `_posts/foo.md`，资源目录留在 `_drafts`；图片解析随后查找 `_posts/foo/photo.png`。嵌套文章目录也被压平成目标根目录。
- 调整：文章、同名资源目录作为一个移动事务，保留相对目录；预先校验两个目标冲突、部分失败回滚。删除文章时明确资源目录如何处理，避免静默遗留。
- 验收：带本地图的草稿互转后预览和 Hexo 生成均可用；资源冲突无半完成状态。此项已追踪到明确文件操作，但未操作真实博客，也未启动真实 Hexo 生成。

### E8 · P2 · 两种预览入口对未保存内容行为不一致

- 定位：`src/features/editor/EditorPage.svelte:661–665`、`:1124`；`src/features/editor/preview/HexoThemePreview.svelte:5`；`src-tauri/src/commands/preview_webview.rs:62–69`。
- 行为：真实主题“打开”直接解析磁盘文章，未保存当前内容；“刷新”只刷新已经存在的独立窗口，窗口未开时按钮仍可用，Promise 拒绝也未显示 notice。关闭自动保存后编辑，用户会看到旧内容；切到另一篇文章点刷新仍刷新旧窗口页面。
- 调整：统一一个“预览当前文章”用例：保存或清晰提示待保存状态→确保服务器→解析当前路由→打开/导航窗口；刷新复用此入口并绑定当前文章。面板提供直接启动服务器动作，不只显示“请先启动”。捕获错误并给出可执行恢复动作。
- 验收：自动保存关闭、有脏内容、窗口不存在、切文章、草稿预览未启用时都有明确可预期反馈。未运行真实 Hexo/native webview，仅静态代码链确认。

## 覆盖与验证边界

完整阅读：

- `src/features/editor/EditorPage.svelte`
- `src/features/editor/EditorSessionStore.ts`、`EditorSessionStore.test.ts`
- `src/features/editor/MarkdownEditor.svelte`、`EditorToolbar.svelte`、`WelcomePanel.svelte`
- `src/features/editor/PreviewAssetRegistry.ts`、`PreviewAssetRegistry.test.ts`
- `src/features/editor/previewModel.ts`、`previewModel.test.ts`
- `src/features/editor/preview/previewSession.ts`、`HexoThemePreview.svelte`、`PreviewModeSwitcher.svelte`
- `src/shared/markdown/safeMarkdown.ts`、`safeMarkdown.test.ts`
- `src-tauri/src/engine/articles.rs`（含单测）
- `src-tauri/src/commands/project.rs`（含单测）
- `src-tauri/src/commands/preview.rs`、`preview_images.rs`、`preview_webview.rs`（含各自单测）
- `src-tauri/resources/resolve-hexo-route.cjs`

补充读取：`src-tauri/src/commands/images.rs:267–364`；平台适配、browserMock 与 E2E 中相关编辑器场景；`package.json`、`vite.config.js`。

新增证据脚本：`editor-browser-repro.js`（E1、E3、E5、E6）、`editor-image-undo-repro.js`（E2）、`editor-store-repro.mjs`（E4）。浏览器通过 Playwright CLI 在本地 demo 验证，后端影响通过相应文件操作代码确认。根审已运行全套现有检查，子审未重复运行相同测试。

AppShell 重扫晚响应、国际化修改用户正文的跨界问题已交由根审验证与报告，此处不重复。


## 设置、同步与更新

审计日期：2026-09-10。只读业务源码；只在 `output/audit/` 写入审计报告、复现程序和隔离文件。未连接真实 GitHub/WebDAV、未改系统凭据、未下载或安装更新。

严重度：P1 为数据损坏、同步可靠性或跨项目上下文问题；P2 为功能受阻、丢失设置草稿、行为与承诺不符。以下“算法已验证”不是完整 Tauri/浏览器端到端验证。

## 优先修复

### S01 / P1：连续保存取消了最新的自动同步任务

- 位置：`src-tauri/src/commands/sync.rs:1203`、`:1204`、`:1228`、`:1232`。
- 触发：已启用同步的项目，在 30 秒内保存两次，然后停止编辑。
- 代码链：第二次保存将新取消句柄插入 `sync_schedules[key]`，发送取消信号给旧任务；旧任务退出时无条件 `remove(key)`，实际移除并 Drop 新任务的 sender；新 receiver 收到关闭，`_ = cancelled` 同样走取消分支。两次保存对应任务都不推送，界面仍可能停在等待同步。
- 验证：真实 Tokio current-thread runtime、等价原调度逻辑，把 30 秒缩成 100 ms；结果 `completed=[]; cancelled=[1, 2]`。见 `sync-logic-repro.rs`、`sync-logic-repro-result.txt`。
- 交互目标：每次保存重置倒计时，最后一次保存后的 30 秒执行一次同步，界面显示等待、执行、完成或失败。
- 代码方案：调度项保存递增任务 ID 与 sender；完成清理仅当 ID 仍等于当前项才移除。也可取消前先完成旧任务清理，或单一每项目 worker 持有可重置 deadline。显式区分收到取消与 sender 被意外 Drop。
- 验收：单次保存执行一次；两次/十次连续保存仅最新任务执行一次；旧任务取消或执行结束均不能删除后来创建的任务；关闭同步后不再推送。

### S02 / P1：同步旧版清单首次选“使用远端内容”会删除本地配置、主题和依赖声明

- 位置：`src-tauri/src/commands/sync.rs:1460`、`:1462`、`:1463`、`:1771`；对比同文件 `:1431` 的旧版保护。界面入口 `src/features/settings/SettingsPage.svelte:644`，WebDAV 同类入口 `:632`。
- 触发：新设备已有完整 Hexo 项目，启用已有合法 schemaVersion=1 内容同步分支，首次选择“使用远端内容”。旧版清单合法范围仅已发布文章、文章资源及配置图片目录。
- 代码链：初始化以完整本地快照作为 `local_base`；应用远端时删除所有 base 中而远端清单没有的文件，导致 `_config.yml`、`package.json`、主题文件、草稿等被当作远端删除内容。`overwriteLocal` 已区分 schema v1/v2，首次初始化未复用该保护。
- 影响：本地项目不再完整，后续预览、发布、重新打开可能失败。虽有备份，用户必须手动恢复；首次方向按钮未明确告知会删除这些文件。
- 验证：提取当前原始 `apply_remote_operations` 和 `ensure_safe_apply_target`，隔离 fixture 验证后只余远端文章，3 个配置/主题文件被删除。见 `sync-logic-repro.rs`、`sync-logic-repro-result.txt`。草稿删除由同一调用链静态推导；`sync.rs:2165` 的 v1 范围及 `:4185` 的现有测试明确排除草稿路径。
- 交互目标：旧版内容同步仅接管旧版已覆盖的范围，首次选远端时显示增加、覆盖和删除清单及备份入口。
- 代码方案：将覆盖范围计算提取为按 manifest schema 统一使用的函数；v1 初始化不能以全项目快照为删除基线。将首次同步方向确认复用正常覆盖确认和预检摘要。
- 验收：v1 初始化更新文章但保留配置、主题、草稿；v2 按完整项目确认范围执行；空远端、只远端文章、同名本地修改均正确，且可恢复备份。

### S03 / P1：同步等待网络期间保存文章，可能生成哈希与实际文件不一致的远端清单

- 位置：`src-tauri/src/commands/sync.rs:1349`、`:1358`、`:1600`、`:1602`、`:1610`、`:2261`、`:2281`；校验位置 `:2110`。
- 触发：同步先扫描本地，然后远端 fetch 较慢；这时继续编辑并保存文章，fetch 完成后同步上传。
- 代码链：快照只保存文件路径/hash，未保存字节；网络返回后 copy 函数重新读取当前磁盘文件，manifest 仍使用网络等待前的 hash。GitHub 上传新内容加旧 hash；WebDAV 甚至用旧 hash 对象名放入新字节。后续读取被 `snapshot_from_manifest` 拒绝为哈希不匹配。
- 验证：隔离文件按该时间顺序扫描 hash、保存新内容、copy，再比较 manifest hash 与 copied hash，确定不一致。见 `settings-sync-state-repro.mjs`、`settings-sync-state-repro-result.txt`。未真实上传。
- 交互目标：写作过程中同步不阻碍保存，每轮同步严格对应一个一致版本，后续修改进入下一轮。
- 代码方案：快照保留不可变字节，或先复制到独立暂存目录并对暂存字节生成 hash，再只从该版本上传。协调保存与快照短临界区，上传完成后如有新 revision 保留 pending 状态并续调度。
- 验收：用受控延迟阻塞远端 fetch，在此期间保存；上传的每个文件/对象都与清单 hash 匹配；下一轮最终同步最新版本；不能把仍有待同步更改标成已同步。

### S04 / P1（静态）：运行中同步可覆盖刚关闭的同步设置，全局注册表更新也存在丢失

- 位置：`src-tauri/src/commands/sync.rs:1319`、`:1644`、`:941`、`:945`、`:3318`、`:3329`；锁仅每项目 `:1291`。
- 触发：后台自动同步已读取注册表并等待网络，用户到设置关闭同步；随后旧任务完成。同类情况为 A 项目同步进行时切换到 B，并修改 B 同步设置。
- 代码链：运行任务持有整个注册表旧副本，网络结束再整文件写回；关闭、启用、连接更新不持同一项目同步锁；不同项目锁也不能保护同一注册表文件。旧副本可恢复用户刚删除的记录或覆盖其他项目的新状态。
- 影响：用户收到“同步已关闭”后可能再次自动上传；跨项目设置或 base_files 回退，使同步状态不可信。
- 交互目标：关闭成功即停止后续自动同步，进行中任务必须有明确完成/取消状态；修改其他项目不影响当前项目设置。
- 代码方案：所有注册表变更用统一锁读取最新内容并按项目 patch，避免跨网络持有全表旧副本；增加每项目配置 generation，运行任务提交前核对启用状态与 generation。关闭操作取消待运行调度，对在途任务明确处理。
- 验收：挂起网络→关闭→释放网络后记录仍关闭；A、B 交错更新互不丢失；连接更换后旧连接结果不能恢复旧配置。

### S05 / P1（静态，跨模块）：旧项目同步完成会把当前项目 ID 绑定回旧项目根目录

- 位置：`src-tauri/src/commands/sync.rs:965`、`:980`、`:1583`；`src-tauri/src/commands/project.rs:661`、`:669`、`:679`、`:686`、`:691`。
- 代码链：sync 命令只在开头核对会话，长期操作后直接对捕获的旧 root 触发 rescan；rescan 从当前项目取 ID，却把旧 root 的扫描结果写成当前 session，没有比较 root 或 generation。项目切换的 `cancel_project_work` 仅取消预览与任务，未取消同步。
- 触发：A 的手动远端应用/冲突解决等待网络时打开 B；A 完成并触发 rescan。可能把 B.id 绑定到 A.root、清空远端资源映射、取消 B 的任务/预览。同步状态事件也没有项目身份，AppShell 不能筛除。
- 方案：rescan 必须携带起始 projectId/generation/root，扫描前和提交写锁内都核对当前 session；旧项目结果只更新旧项目磁盘/注册表，不替换当前 session。事件包含明确项目身份。本报告保留为 S05；与 A01 分别对应后端身份与前端提交校验，实施时共同处理。
- 验收：A 同步→切 B→A 完成，B 的 ID、根目录、文章、预览与任务均不变化，A 的结果仍可在打开 A 时正确反映。

## 同步交互

### S06 / P2（静态）：WebDAV 遇到远端更新或冲突后没有解决入口

- 位置：`src/features/settings/SettingsPage.svelte:620`、`:632`、`:637`、`:645`、`:648`；后台 `src-tauri/src/commands/sync.rs:1505`、`:1515`。
- 触发：已初始化 WebDAV，另一设备修改文件；当前设备点“立即同步”，返回 remoteAhead；或两端修改同一文件返回 conflict。
- 问题：后端两种 provider 使用同一状态机，但 WebDAV 专用分支仅显示“立即同步/关闭同步”，比较版本、覆盖方向、逐项冲突与备份入口都位于 GitHub 的 else 分支。重复 auto 只返回同一待处理状态。
- 方案：将连接表单与通用同步决策区分离；所有 provider 共用 remoteAhead/conflict/初次方向面板，显示差异与已选方向；未选择冲突版本前不允许提交。
- 验收：WebDAV remoteAhead 可拉取或明确覆盖；Markdown 与二进制冲突都可逐项解决；凭据表单仍可编辑；不会让用户通过关闭再启用绕过冲突。

### S07 / P2（静态）：冲突期间远端再次变化后，GitHub 冲突面板没有刷新/恢复路径

- 位置：`src-tauri/src/commands/sync.rs:1012`、`:1072`；`src/features/settings/SettingsPage.svelte:648`–`:662`，方法 `:265`、`:276`。
- 触发：已显示冲突列表，其他设备再次提交；当前点击“提交冲突选择”，后端拒绝旧 head，提示重新检查。
- 问题：冲突面板没有“重新检查”或“关闭同步”；离开重进也只是根据旧 record 读冲突，不执行更新 head 的 runSync，仍报错。用户无法在该界面完成恢复。
- 方案：显示“远端已变化，重新加载比较”，调用同步检查重建冲突集合/head，再丢弃或仅保留 hash 未变项的选择。错误不应只弹短提示，应留在面板。
- 验收：远端在展示前/提交前变化两种情况均可刷新恢复；新冲突必须重新确认；不能用旧选择覆盖新远端。

## 设置草稿

### S08 / P2：主题选回原值后视觉仍停在新主题，保存/取消同时消失

- 位置：`src/features/settings/SettingsPage.svelte:360`、`:363`、`:87`；`SettingsHeader.svelte:14`。
- 触发：已保存浅色→选深色→再选浅色。
- 问题：preview 仅当 next 不等于 saved 才调用；返回 saved 时不调用，dirty 已变 false，按钮消失。销毁时也因不 dirty 不恢复主题。
- 验证：直接提取生产 `change` 函数执行，preview 只有 dark，draft/saved 均 light。见 `settings-sync-state-repro-result.txt`。
- 方案：按上一个 draft 的主题变化触发 preview，或每次主题选择无条件应用选中模式；取消/销毁恢复逻辑与 dirty 状态分别设计。
- 验收：light→dark→light、system→dark→system 都立即显示所选模式，dirty 与按钮正确；取消与离开恢复保存值。

### S09 / P2：保存中的后续编辑被保存响应覆盖

- 位置：`src/features/settings/SettingsPage.svelte:367`、`:370`–`:372`；普通表单控件 `:530`、`:549` 等没有 saving 禁用。
- 触发：修改一项并保存；在保存响应尚未返回时修改另一项。
- 问题：请求提交前 clone，一旦返回无条件把 draft 重置成旧提交值，后续输入丢失且被标为已保存。
- 验证：直接提取生产 persistConfig/change，受控 Promise：等待时选择 en-US，完成后回到 zh-CN。
- 方案：draft revision 与提交 revision 分离，saved 更新为实际成功版本；draft 若后来变化则保留并保持 dirty。也可保存期间禁用整个表单，但应防止导航/子组件动作绕过；保存 controller 应等待正在进行的保存而非立即返回。
- 验收：延迟保存期间二次输入不丢失，失败保留全部草稿；保存后立即导航的 guard 等到保存完成且无剩余 dirty 才离开。

### S10 / P2（静态）：点击获取/删除 Token 会默默保存其他分类所有草稿

- 位置：`src/features/settings/SettingsPage.svelte:434`–`:439`、`:492`–`:497`；承诺 `SettingsHeader.svelte:12`。
- 触发：更改自动保存/主题等尚未点保存，切到图床点击获取 Token，然后在弹窗取消；或删除本地 Token。
- 问题：`persistConfig(draft)` 全量提交且重置 saved/draft，其他分类的未确认更改被持久化，弹窗取消无法回滚。反馈却仅说图床基础配置已保存。
- 方案：只 patch 必须提交的图床连接字段，并保留其他草稿的 dirty；或者操作前明确展示本次要保存的所有分类并由用户确认。Token 本身是即时操作，应在文案与保存范围上清楚区分。
- 验收：修改常规设置→开 Token 弹窗→取消，常规修改仍未保存且可撤销；删除 Token 不顺带改变其他配置。

### S11 / P2：鼠标保存失败没有用户反馈，也没有字段校验提示

- 位置：`src/features/settings/SettingsPage.svelte:367`–`:380`、`:516`、`:553`；`SettingsHeader.svelte:16`；后端校验 `src-tauri/src/domain/mod.rs:515`。
- 触发：手动输入字号 40、清空数字输入，或配置磁盘写失败，点击顶部保存。
- 问题：页面不是提交验证的 form，min/max 不会阻止该按钮；saveDraft/persistConfig 只有 finally 没 catch，直接从按钮调用的 Promise 拒绝未转为 onNotice。用户看见“正在保存”消失，却不知原因；键盘保存由 AppShell catch，体验不一致。
- 验证：浏览器实际保存按钮配合模拟配置写入失败，收到 pageerror，但 notice=0，页面保留“有未保存更改”；见 `browser-settings-error-result.txt`。字段提交验证部分为静态核查。
- 方案：统一保存入口处理错误并展示持久、可关联字段的错误；前端提交前按契约验证数字/URL，聚焦第一个错误字段；后端错误仍须可见，保留草稿。
- 验收：非法数字不提交且有明确字段提示；磁盘失败有重试和原因；鼠标与快捷键提示一致，失败时不显示已保存。

## 更新

### S12 / P2（静态）：下载前再次检查发现更新已撤回，会永久停在 downloading

- 位置：`src-tauri/src/commands/update.rs:174`–`:179`、`:181`–`:191`；`src/features/about/AboutPage.svelte:58`。
- 触发：先检查发现更新，再下载时服务端已撤回或更新清单暂时指向当前版本，第二次 check 返回 None。
- 问题：download_update 已先 store downloading；None 分支直接返回 AppError，没有写终止状态。关于页在 downloading 禁止检查，进度条一直存在，无法从页面恢复。
- 方案：所有下载出口都通过统一状态迁移，None 写 upToDate/availableChanged 并说明该版本暂不可用；重新检查始终有可恢复路径。下载元数据应绑定此次检查实际得到的版本。
- 验收：available→download check None 后停止进度，显示可理解原因并能重新检查；下载错误、签名错误均不残留 busy。

### S13 / P2（静态）：安装失败后无法重试已下载安装包，只能重新检查并重下

- 位置：`src-tauri/src/commands/update.rs:268`–`:272`、`:141`–`:142`；`src/features/update/updateViewModel.ts:12`；`src/features/about/AboutPage.svelte:58`。
- 触发：更新下载成功，安装遇到可恢复错误后再进入关于页。
- 问题：后端保留 downloaded_update 供重试，但状态变 error/install；UI 只在 downloaded 显示安装按钮，错误状态只显示检查，检查又清空下载缓存。实际错误信息也未常驻呈现（errorLabel 已计算未渲染）。
- 方案：error/install 且缓存存在时提供“重试安装”，同屏显示失败原因及已下载版本；只在选择新版本时替换缓存。采用统一 updateViewModel 的 canCheck/canDownload/canInstall 渲染操作。
- 验收：模拟安装失败后无需重新下载即可重试；离开重进错误与操作仍存在；后台检查不会清除待安装缓存。

## 阅读覆盖与边界

完整阅读：

- `src/features/settings/` 的全部 5 个 Svelte/TS 文件（包括 controller），`src/features/update/` 全部 4 个文件（含测试），`src/features/about/AboutPage.svelte`。
- `src-tauri/src/commands/config.rs`、`sync.rs` 全部 4679 行（含测试/测试服务器）、`sync_validation.rs`、`update.rs`（含测试）、`app_info.rs`。
- `src-tauri/src/data/config_repository.rs`（含迁移与测试）、`src-tauri/src/platform/credentials.rs`（含测试）。
- 模块文档 05 设置与国际化、07 更新关于诊断、12 内容同步后台，各自作用/内部/对外接口全部 9 篇。

针对调用链阅读：AppShell 配置保存、导航、更新安装、同步订阅；AppState 字段/会话/锁；project rescan 与 cancel；domain config validate；platform tauri 相关方法；desktop e2e 中设置/同步/更新用例；Cargo.toml 和 package.json。

验证共 5 个确定性逻辑场景：自动同步防抖、v1 初始化删除范围、同步 hash/字节一致性、主题往返、保存中继续编辑。其余是带可达触发路径的静态结论。现有同步 e2e 使用 browserMock，WebDAV 主要覆盖连接/启用；没有覆盖本文后台并发竞态或 WebDAV 冲突决策。未执行带真实外部环境的 WebDAV 测试。

未修改业务源码，未提交修复。主审可按影响合并 S05 与其他跨项目/编辑器问题，并将 UI 相邻问题合并为较少实施任务。


## 图片与插件

审计日期：2026-09-10。业务源码未修改。所有验证均使用本地模拟 IPC、固定资源和假 Worker，没有真实图床请求、上传、删除或插件安装。

## 验证方式

执行：

```powershell
npx vitest run --config output/audit/images-plugins.vitest.config.mjs --reporter=json --outputFile=output/audit/images-plugins-results.json
```

最终结果：7 / 7 探针通过，无未处理异常。**通过表示断言确认了当前缺陷，不能解释为这些功能正确。** 测试实际挂载项目 Svelte 组件，并保留 ModalDialog 的事件处理；只替换平台 IPC、Worker 和 jsdom 不具备的滚动、视觉动画 API。证明范围是前端组件行为及其与静态核查的 IPC 契约组合，不包含原生 WebView 端到端和真实 Cloudflare 服务兼容性。

## 已验证问题

### IMG-01 · P2：取消本地导入会清空资源列表，并提示导入成功

- 位置：`src-tauri/src/commands/images.rs:54` 返回 `Ok(Vec::new())`；`src/features/image-bed/ImageBedPage.svelte:257` 把结果整体赋给 `localImages`，258 行无条件提示成功。
- 触发：本地图库已有图片 → 点「导入」→ 在系统选择器取消。
- 实际：已有图片全部从页面消失，显示「当前目录为空」，提示「图片已导入」。磁盘图片未被删除，刷新后恢复。
- 证据：`cancelled local import clears the displayed list and reports success` 实际挂载组件，mock 与 Rust 取消分支一致的空数组。
- 方案：IPC 返回可区分取消与成功的结果，例如 `{ canceled, importedCount, images }`；取消时保持列表、搜索、滚动位置，不提示成功。成功后按实际数量提示。
- 验收：已有图片情况下取消选择，项目列表和计数不变；空图库取消仍为空但没有成功通知；正常导入显示实际新增数。

### IMG-02 · P2：远程最后一页删空后无法回到上一页

- 位置：`ImageBedPage.svelte:357`–359 删除只过滤当前数组和递减总量；558 行空数组直接显示空态；575 行分页在非空分支内部，且只在 `remoteTotal > 48` 出现。
- 触发：目录有 49 项 → 翻到第 2 页 → 删除唯一资源。
- 实际：计数仍为「48 项」，内容显示「当前目录为空」，上一页按钮消失。刷新沿用 `remoteOffset=48`，不能恢复第一页；需要清搜索/切来源等隐含操作。
- 证据：`deleting the sole item on page two hides pagination while 48 items remain`。
- 方案：变更成功后重新查询；当 `offset >= total` 时回退到有效页；分页组件放在列表空态分支之外，使 `offset > 0` 时始终有返回入口。移动导致页空、服务端外部删除同样处理。
- 验收：49→48、97→96、末页全部移动、服务端外部删除后刷新都自动展示有效页，无假空目录。

### IMG-03 · P2：旧搜索响应能覆盖最新查询，目录导航也没有响应版本保护

- 位置：`ImageBedPage.svelte:211`–238；请求完成后无请求 ID / 查询快照检查，直接更新列表、目录、错误、loading。搜索按钮 542 行在 loading 时仍可操作。
- 触发：搜索 older，未完成时再搜索 newer；newer 先返回、older 后返回。
- 实际：搜索栏与标签仍是 newer，显示的资源却是 older。旧响应中的 `currentDirectory` 还能覆盖已切换的目录并触发新一轮加载。旧请求 finally 也会提前取消新请求的加载状态。
- 证据：`an older search response overwrites the newer query results`，使用两个独立 deferred Promise，明确控制返回顺序。
- 方案：每次加载捕获 projectId、generation、provider、目录、查询、offset 和递增 requestId；仅最新请求可提交状态。切项目/来源与销毁时失效旧请求；同样保护 catch/finally。必要时增加取消信号。
- 验收：两次查询、目录返回、切来源的响应顺序倒置时，只呈现最后一次意图；loading 持续到当前请求完成；旧错误不覆盖新结果。
- 限定：搜索乱序已实测；跨项目影响需结合父组件挂载生命周期，不据此声称已复现跨项目修改。

### IMG-04 · P2：重命名输入框按一次 Enter 会发出两次修改请求

- 位置：`ImageBedPage.svelte:598`（输入框自己的 Enter 处理）与 `src/shared/components/ModalDialog.svelte:33`–45（window 监听再点击主按钮）；`ImageBedPage.svelte:369` 的函数入口未检查 `working`。
- 触发：打开远程图片重命名，输入名称，按一次 Enter。
- 实际：先由 input handler 请求一次；事件冒泡至 window 后 ModalDialog 再 click 确认一次。Svelte 尚未把 working 更新到 DOM disabled，第二次调用进入函数。
- 证据：`one Enter in rename submits the same mutation twice` 断言同一事件发出两次 `renameCloudflareAsset`。
- 影响：重复修改远程资源，可能同时出现成功/失败提示，并发刷新不稳定。移动框 605 行具有相同事件链；其函数入口 382 行也无 working 检查，为静态确认的同类问题。
- 方案：使用真正的 form submit 统一提交入口；ModalDialog 不自行劫持所有 input Enter，或子组件取消冒泡。所有变更函数入口都检查进行中状态；按钮禁用只作为显示层。
- 验收：Enter、单击、按住 Enter、快速双击各次用户确认最多创建一个进行中请求；输入法组合态 Enter 不提交；移动同测。

### IMG-05 · P2：删除进行中仍允许取消，完成后列表与计数不一致

- 位置：`ImageBedPage.svelte:349`–365，await 后使用可变的 `deleting?.id`；621–622 行取消、关闭、再次确认均未受进行中状态保护。
- 触发：点击删除确认，IPC 未完成时点击取消；随后删除成功返回。
- 实际：取消已令 `deleting=null`，过滤条件变成 `asset.assetId !== undefined`，被删项保留；总数仍减一，出现「0 项」但仍展示图片。取消按钮也误导用户认为可取消已经执行的永久删除。
- 证据：`closing pending delete reports success but leaves the deleted item visible`。
- 方案：开始时捕获 `const target` 和所属会话；变更中禁止重复提交。若后台不支持撤销，将关闭语义明确为「关闭」而非「取消」，后台完成独立更新捕获目标；或者在完成前禁用取消/Escape/遮罩关闭。完成后重查当前页。
- 验收：慢删除期间双击只调用一次；关闭对话框不改变待处理目标；完成后实际删除项不再显示且计数正确；打开另一个删除框不被旧操作清空。

### PLG-01 · P2：插件设置显示了默认值，保存和测试却收到空配置

- 位置：`src/features/plugins/PluginManagerPage.svelte:54`–57 直接使用已保存值；117 行只在 input 的 value 中显示 `schema.default`；66、75、77 行传递的仍是原 settings。
- 触发：首次打开官方示例插件设置，输入框已显示 `https://example.com`，不编辑就保存或测试。
- 实际：保存的是 `{}`，测试校验收到缺少 endpoint 的对象，官方示例 `examples/plugins/imagebed-example/index.js:15` 判无效。用户看到填写完的表单却被告知缺必填项。
- 证据：`the settings dialog displays the schema default but saves an empty object` 明确断言可见 input 与实际保存参数不同。官方示例的校验行为静态核查。
- 方案：加载时将支持字段的 schema 默认值与已保存值合成真实表单状态；显示、校验、测试、保存使用同一份规范化数据；保存前执行 schema/插件校验，并把错误关联到字段。
- 验收：首次打开直接测试/保存收到默认 endpoint；修改值后持久化一致；重新打开显示实际保存值；空字符串与缺失值按 schema 区分。

### PLG-02 · P2：插件一次超时后无法通过重试或重新启用恢复

- 位置：`src/shared/plugins/PluginWorkerRuntime.ts:14` 超时 terminate；`src/shared/plugins/PluginProviderRuntime.ts:49`–55 仍从按插件 ID 缓存的 map 返回同一个已终止 Worker。
- 触发：连接测试或上传超过 10 秒 → 再次测试/上传。
- 实际：新请求投递给已终止 Worker，再等 10 秒超时；没有自动重建。后端 HTTP 超时为 15 秒（`src-tauri/src/plugins/host_commands.rs:223`），正常 10–15 秒请求就可能把 Worker 永久置于此状态。
- 证据：`a timed out worker remains cached so subsequent requests cannot recover` 使用假 Worker / 假时钟，确认两次请求只有一个 Worker 实例且第一次已终止。
- 生命周期补充（静态）：`disposePluginWorkers` 仅有定义、全项目无调用；`PluginManagerPage.svelte:36`–43 禁用/卸载没有终止缓存 Worker，重新安装同 ID 也可能继续使用旧实例及旧 manifest。这里只描述生命周期不一致，未做任何权限绕过验证。
- 方案：运行时维护明确状态；超时/崩溃触发移除 map 项并统一拒绝待处理请求，下次重建。按插件版本/入口标识缓存；禁用、卸载和应用退出按 ID 终止。统一 Worker/Host 超时预算，前端给「重试」和进行中反馈。
- 验收：首次超时后下一次新建 Worker 并能成功；并发请求收到一致的终止结果；禁用/卸载后旧 Worker 已终止，重新启用加载新实例。

## 静态确认的补充问题

### IMG-06 · P2：批量导入中途失败会留下已经复制的文件，重试可能制造重复文件

- 位置：`src-tauri/src/commands/images.rs:59`–71 在同一循环逐个验证、逐个 fs::copy，任何后续错误立即返回；前端 `ImageBedPage.svelte:266` 只提示错误、不重读目录。
- 触发：先选择合法图片 A，再选择超过 25 MB 的 B；复制 A 后 B 校验失败。
- 影响：用户收到整体失败且列表没有 A，但磁盘已存在 A；重试时 unique_target 为 A 生成新名称，留下重复资源。
- 方案：返回逐文件结果与明确成功数；至少失败后刷新列表。更佳做法是全量预检后执行，仍以逐文件结果处理运行期 I/O 错误，重试只重试失败项。
- 验收：混合合法/超限/读取失败文件时准确呈现每项结果；重复点击失败项重试不重复导入成功项。未执行真实文件操作验证。

### IMG-07 · P2：图床加载、上传、连接测试缺少有限超时与取消反馈

- 位置：`src-tauri/src/commands/images.rs:498,685,755` 使用无配置的 `reqwest::Client::new()`；`src-tauri/src/platform/cloudflare_imgbed/client.rs:14` 同样用于重命名、移动和删除；`src-tauri/src/commands/imgbed_auth.rs:58,161` 的 builder 未配置 timeout。
- 触发：服务已建立连接却一直不返回/不结束响应。
- 影响：前端 loading/working/测试状态依赖 Promise finally，可能长时间不恢复，上传按钮无取消；重试入口对 loading 状态被禁用。批量插件请求虽有超时，却与 Host 超时预算不一致，见 PLG-02。
- 方案：统一 HTTP client，配置 connect/read/overall 超时；取消请求要有真实取消句柄。读取与写入操作区别提示：写入超时提示「结果未确认，请刷新确认」，避免诱导重复上传或删除。
- 验收：使用本机模拟超时服务或 mock 时钟校验所有请求在预算内结束；状态恢复，用户可重试；写操作超时不假报失败后自动重做。未连接外部服务。

### PLG-03 · P2：卸载插件会立即删除设置，没有确认；禁用/卸载失败也没有界面反馈

- 位置：`PluginManagerPage.svelte:99` 直接调用卸载；`src-tauri/src/plugins/host_commands.rs:97` 删除整个插件目录，其中包括 `settings.json`；前端36–43行缺少 try/catch/busy。
- 触发：误点卸载；或卸载/禁用遇到文件占用、权限/I/O 失败。
- 影响：插件配置随包被永久删除，没有审阅影响和恢复入口；失败时只有未处理 Promise 拒绝，用户看不出原因，并可能重复点击。
- 方案：卸载前展示插件名、关联图床和设置处置选项，默认保留用户设置；明确卸载与禁用的区别。统一异步状态、错误提示及进行中禁用，成功后确认默认图床落到有效 provider。
- 验收：误点可取消且磁盘不变；保留设置后重装恢复；模拟 I/O 失败有中文错误并维持一致列表；快速连点只执行一次。

## 高价值交互优化建议

1. **资源管理操作提示文章影响。** 当前删除提示仅说回收站/永久删除，移动/重命名也未说明既有 Markdown URL 是否失效。为当前项目建立引用查询，操作前显示「被 N 篇文章引用」及文章入口；本地路径改变支持统一更新引用，远程路径变更要说明服务端链接语义，不能承诺已自动修复。验收包括被多篇文章引用的图片及尚未保存的文章。
2. **插件配置按真实 schema 控件渲染。** 117 行除 number 外一律 text，boolean 变成字符串、enum 无选项、object/array 显示无效文本；当前保存仅检查 JSON 是对象。支持明确的 schema 子集（string/number/integer/boolean/enum、required、范围），不支持的类型明确说明，不把无法正确编辑的字段伪装成普通输入框。
3. **图片来源与默认上传位置保持可理解。** 图床页只支持本地和内置 Cloudflare；启用插件后编辑器上传可走插件，但资源页的导入/上传仍是两个内置源。标注当前浏览来源及上传目标，插件无列表能力时说明「支持编辑器上传，暂不支持浏览」，避免把插件默认来源静默呈现为本地图库。
4. **搜索空态和加载状态携带真实上下文。** 远程搜索需要提交，而本地输入即过滤；用标签明确「当前目录递归搜索」，空态使用 appliedQuery 而不是尚未提交的 query。已有结果加载时保留布局并显示正在查哪个目录，避免只降低透明度。
5. **下载改为可追踪任务。** 后端完整读取任意远程文件成 Vec，再转为 JS number[]/Uint8Array/Blob，下载完成才提示「下载已开始」。对视频、压缩包显示保存位置、字节进度、取消和完成结果；大文件使用流式保存而非多次全量复制内存。
6. **插件生命周期向用户暴露可恢复状态。** 列表区分未启用、运行中、故障、API不兼容；不要把无法载入的插件完全隐藏成未安装。安装复制使用暂存目录，成功后提交，失败后清理，避免部分安装阻止重试。权限给中文能力说明而非只有原始字符串。

## 阅读覆盖

以下 35 个指定文件均完整阅读，共 4,066 行（含测试、文档、示例）：

- `src/features/image-bed/`：ImageBedPage.svelte、model.ts、model.test.ts。
- `src/features/plugins/`：PluginManagerPage.svelte。
- `src/shared/plugins/`：PluginHost.ts、PluginHost.test.ts、PluginProviderRuntime.ts、PluginProviderRuntime.test.ts、PluginWorkerRuntime.ts、permissions.ts、protocol.ts、types.ts。
- `src-tauri/src/plugins/`：errors.rs、host_commands.rs、manifest.rs、mod.rs、package.rs、permissions.rs、registry.rs、tests.rs。
- `src-tauri/src/commands/`：images.rs、image_local.rs、imgbed_auth.rs。
- `src-tauri/src/platform/cloudflare_imgbed/`：client.rs、endpoints.rs、mod.rs、normalize.rs、tests.rs、types.rs。
- `examples/plugins/imagebed-example/`：index.js、manifest.json、README.md、settings.schema.json。
- `docs/`：plugin-api-reference.md、plugin-development.md。

边界交叉核查：完整阅读 ModalDialog.svelte；定向读取 EditorPage.svelte 的插件加载、插图、文件处理；SettingsPage 的插件管理接入；平台 Tauri / browserMock 插件入口；domain 的 RemoteAssetCapabilities；CSS 的图片加载交互规则；package.json、Vite 与 TS 测试配置。编辑器插图切文章竞态已转交负责编辑器的审计者，不在这里重复归因。

现有测试阅读发现：image-bed/model.test.ts 仅覆盖凭据就绪判定、预览索引、文件类型文本；插件现有测试主要覆盖权限判定和 provider 映射，未覆盖上述组件生命周期。新增证据探针独立置于 output/audit，不影响默认项目测试集。

## 证据索引与后续回归要求

- `output/audit/browser-shell-result.txt`：语言往返、文章预览误翻译、弹窗发布快捷键。
- `output/audit/browser-settings-error-result.txt`：实际按钮保存失败产生 pageerror，但 notice=0，确认 S11 鼠标路径；错误为注入的本机模拟。
- `output/audit/shell-state-results.txt`：A01 三种晚响应/删除状态。
- `output/audit/editor-browser-repro.js`、`editor-image-undo-repro.js`、`editor-store-repro.mjs`：编辑器证据脚本（前两者原使用 localhost:1422，运行时需启动 demo 服务）。
- `output/audit/images-plugins.test.ts`、`images-plugins-results.json`：7 个组件/Worker 证据探针。
- `output/audit/sync-logic-repro.rs`、`sync-logic-repro-result.txt`：Tokio 调度和 v1 文件范围。
- `output/audit/settings-sync-state-repro.mjs`、`settings-sync-state-repro-result.txt`：hash/字节、主题、设置草稿。
- `output/playwright/audit-language-roundtrip.png`、`audit-preview-translated.png`：本次关键截图。
- `output/audit/design-detector.json`：机械扫描提示，blockquote 边线经核实排除。

修复后的测试不应继续断言“缺陷存在”。将探针改为上述期望，纳入正常测试目录；补齐延迟/乱序/取消/组件卸载/切项目矩阵。最后在隔离的真实 Tauri 项目上验证原生选择器取消、文件移动与回收站、真实 Hexo 预览、两个同步 provider 及更新状态恢复，并对 Windows/macOS 分别验收。
