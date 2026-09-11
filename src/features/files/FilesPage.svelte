<script lang="ts">
  import { onDestroy } from "svelte";
  import { ChevronRight, FileText, Folder, FolderOpen, RefreshCw, Save, Search } from "@lucide/svelte";
  import { ui } from "$shared/i18n/ui";
  import { normalizeError, platform } from "$platform/tauri";
  import ModalDialog from "$shared/components/ModalDialog.svelte";
  import TextFileEditor from "./TextFileEditor.svelte";
  import type { FileSessionStore } from "./FileSessionStore";
  import type { AppConfigV3, ArticleSummary, ProjectFileEntry, ProjectFileSnapshot, ProjectSessionView } from "$shared/types/app";

  export let session: ProjectSessionView | null;
  export let config: AppConfigV3;
  export let articles: ArticleSummary[] = [];
  export let fileStore: FileSessionStore;
  export let onOpenProject: () => void;
  export let onNotice: (message: string) => void = () => {};
  export let onOpenArticle: (articleId: string) => void = () => {};

  let state = fileStore.getState();
  const unsubscribe = fileStore.subscribe(value => state = value);
  let key = "";
  let alive = true;
  let directoryEntries: Record<string, ProjectFileEntry[]> = {};
  let expanded = new Set<string>([""]);
  let loading = new Set<string>();
  let error = "";
  let opening = false;
  let openSequence = 0;
  let pathQuery = "";
  let pendingPath: string | null = null;
  let switchBusy = false;
  let compare: ProjectFileSnapshot | null = null;
  let compareBusy = false;

  $: nextKey = session ? `${session.projectId}:${session.generation}` : "";
  $: if (key !== nextKey) { key = nextKey; directoryEntries = {}; loading = new Set(); opening = false; openSequence += 1; expanded = new Set([""]); if (session) void loadDirectory(""); }
  $: visible = flatten("", 0, directoryEntries, expanded);

  onDestroy(() => { alive = false; openSequence += 1; unsubscribe(); });
  function flatten(directory: string, depth: number, entries: Record<string, ProjectFileEntry[]>, opened: Set<string>): Array<{ entry: ProjectFileEntry; depth: number }> {
    return (entries[directory] ?? []).flatMap(entry => [{ entry, depth }, ...(entry.kind === "directory" && opened.has(entry.path) ? flatten(entry.path, depth + 1, entries, opened) : [])]);
  }
  async function loadDirectory(directory: string) {
    if (!session || loading.has(directory)) return;
    const identity = key; const project = session;
    loading = new Set(loading).add(directory); error = "";
    try {
      const entries = await platform.listProjectFiles(project.projectId, project.generation, directory);
      if (alive && key === identity) directoryEntries = { ...directoryEntries, [directory]: entries };
    } catch (reason) { if (alive && key === identity) error = normalizeError(reason).message; }
    finally { if (alive && key === identity) loading = new Set([...loading].filter(path => path !== directory)); }
  }
  function toggle(entry: ProjectFileEntry) {
    const next = new Set(expanded);
    if (next.has(entry.path)) next.delete(entry.path);
    else { next.add(entry.path); if (!directoryEntries[entry.path]) void loadDirectory(entry.path); }
    expanded = next;
  }
  async function refreshTree() {
    await Promise.all([...expanded].map(path => loadDirectory(path)));
    if (session) await fileStore.refresh(session, (project, path) => platform.loadProjectFile(project.projectId, project.generation, path)).catch(reason => error = normalizeError(reason).message);
  }
  function requestOpen(path: string) {
    path = path.replace(/\\/g, "/").replace(/\/{2,}/g, "/").replace(/^\.\//, "");
    if (switchBusy || compareBusy) return;
    if (path === state.snapshot?.path) return;
    if (fileStore.hasDirty()) { pendingPath = path; return; }
    void openFile(path);
  }
  async function openFile(path: string) {
    if (!session) return;
    const article = articles.find(item => item.relativePath === path);
    if (article) { onOpenArticle(article.articleId); return; }
    const sequence = ++openSequence; const identity = key; const project = session;
    opening = true; error = "";
    try {
      const file = await platform.loadProjectFile(project.projectId, project.generation, path);
      if (!alive || sequence !== openSequence || identity !== key) return;
      fileStore.load(file);
      pathQuery = "";
    } catch (reason) { if (alive && sequence === openSequence && identity === key) error = normalizeError(reason).message; }
    finally { if (sequence === openSequence) opening = false; }
  }
  async function resolveSwitch(choice: "save" | "discard") {
    if (switchBusy || pendingPath === null) return;
    switchBusy = true;
    try {
      if (choice === "save") await fileStore.save(); else await fileStore.discard();
      const path = pendingPath; pendingPath = null; await openFile(path);
    } catch (reason) { error = normalizeError(reason).message; }
    finally { switchBusy = false; }
  }
  async function save() {
    try { await fileStore.save(); }
    catch (reason) { error = normalizeError(reason).message; }
  }
  async function reviewDisk() {
    if (!session || !state.snapshot || compareBusy) return;
    const path = state.snapshot.path; const instance = state.documentInstance; const identity = key;
    compareBusy = true;
    try {
      const disk = await platform.loadProjectFile(session.projectId, session.generation, path);
      if (alive && key === identity && state.documentInstance === instance) compare = disk;
    } catch (reason) { error = normalizeError(reason).message; }
    finally { compareBusy = false; }
  }
  async function resolveComparison(keepLocal: boolean) {
    if (!compare || !session || compareBusy) return;
    compareBusy = true; const shown = compare; const instance = state.documentInstance;
    try {
      const latest = await platform.loadProjectFile(session.projectId, session.generation, shown.path);
      if (!alive || state.documentInstance !== instance) return;
      if (latest.contentHash !== shown.contentHash) { compare = latest; error = "磁盘内容再次变化，请查看最新版本后重新选择。"; return; }
      if (keepLocal) { fileStore.keepLocalAgainst(latest); await fileStore.save(); }
      else fileStore.acceptDisk(latest);
      compare = null; error = "";
    } catch (reason) { error = normalizeError(reason).message; }
    finally { compareBusy = false; }
  }
  function jumpToPath() {
    const path = pathQuery.trim().replace(/\\/g, "/").replace(/^\.\//, "");
    if (path) requestOpen(path);
  }
  async function revealFile() {
    if (!session || !state.snapshot) return;
    try { await platform.revealProjectFile(session.projectId, session.generation, state.snapshot.path); }
    catch (reason) { error = normalizeError(reason).message; onNotice(error); }
  }
</script>

{#if !session}
  <div class="files-empty"><FolderOpen size={32} /><h1>{$ui("全部文件")}</h1><p>{$ui("打开博客后，按目录查找和编辑友链、主题配置与页面文件。")}</p><button class="button primary" type="button" on:click={onOpenProject}>{$ui("打开博客项目")}</button></div>
{:else}
  <div class="files-page">
    <aside class="file-explorer" aria-label={$ui("项目文件")}> 
      <div class="explorer-heading"><strong>{$ui("全部文件")}</strong><button class="icon-button" type="button" aria-label={$ui("刷新文件树")} title={$ui("刷新文件树")} on:click={refreshTree} disabled={loading.size > 0}><RefreshCw size={15} /></button></div>
      <div class="explorer-project" title={session.displayPath}>{session.name}</div>
      <form class="path-search" on:submit|preventDefault={jumpToPath}><Search size={14} /><input class="input" bind:value={pathQuery} aria-label={$ui("打开文件路径")} placeholder={$ui("输入相对路径，回车打开")} /></form>
      <div class="file-tree" aria-label={$ui("项目目录")}> 
        {#each visible as { entry, depth } (entry.path)}
          <button type="button" class="file-tree-row" class:selected={state.snapshot?.path === entry.path} style:padding-left={`${10 + depth * 15}px`} title={entry.path} aria-label={entry.path} aria-expanded={entry.kind === "directory" ? expanded.has(entry.path) : undefined} aria-current={state.snapshot?.path === entry.path ? "true" : undefined} on:click={() => entry.kind === "directory" ? toggle(entry) : requestOpen(entry.path)}>
            {#if entry.kind === "directory"}<ChevronRight size={12} class={expanded.has(entry.path) ? "folder-expanded" : ""} />{#if expanded.has(entry.path)}<FolderOpen size={15} />{:else}<Folder size={15} />{/if}{:else}<span class="file-spacer"></span><FileText size={15} />{/if}
            <span>{entry.name}</span>{#if loading.has(entry.path)}<small>…</small>{/if}
          </button>
        {/each}
        {#if loading.has("")}<p class="explorer-hint">{$ui("正在读取项目文件")}</p>{:else if !visible.length}<p class="explorer-hint">{$ui("目录为空")}</p>{/if}
      </div>
      <p class="explorer-hint">{$ui("博文在写作页打开；其他 UTF-8 文本在此编辑。")}</p>
    </aside>
    <section class="file-workspace" aria-label={$ui("文件编辑器")}>
      {#if state.snapshot}
        <header class="file-editor-heading"><div><strong title={state.snapshot.path}>{state.snapshot.path}</strong><span>{state.saving ? $ui("保存中") : state.dirty ? $ui("未保存") : state.snapshot.editable ? $ui("已保存") : $ui("只读")}</span></div><button class="button" type="button" on:click={reviewDisk} disabled={compareBusy || state.saving}>{$ui("比较磁盘版本")}</button><button class="button primary" type="button" on:click={save} disabled={!state.snapshot.editable || !state.dirty || state.saving || Boolean(state.conflict)}><Save size={15} />{$ui("保存")}</button></header>
      {/if}
      {#if error || state.error}<div class="file-error" role="alert"><span>{$ui(error || state.error || "")}</span>{#if state.snapshot}<button class="button" type="button" disabled={compareBusy || state.saving} on:click={reviewDisk}>{$ui("比较磁盘版本")}</button>{/if}</div>{/if}
      {#if opening}<p class="explorer-hint" role="status">{$ui("正在打开文件")}</p>{/if}
      {#if state.snapshot}
        {#if state.snapshot.readOnlyReason}<div class="file-readonly"><span>{$ui(state.snapshot.readOnlyReason)}</span><button class="button" type="button" on:click={revealFile}>{$ui("在文件管理器中显示")}</button></div>{/if}
        <TextFileEditor path={state.snapshot.path} content={state.content} documentInstance={state.documentInstance} {config} readonly={!state.snapshot.editable || opening || switchBusy || compareBusy} onChange={content => fileStore.update(content)} onSave={save} />
        <footer class="file-footer"><span>{state.snapshot.editable ? "UTF-8" : $ui("只读")} · {state.snapshot.path.split(".").at(-1)?.toUpperCase()}</span><span>{$ui("保存只写入本地，同步仍按设置中的范围进行。")}</span></footer>
      {:else}<div class="files-empty"><FileText size={30} /><h2>{$ui("选择一个文件开始编辑")}</h2><p>{$ui("例如 source/_data/link.yml、_config.yml 或 themes 下的主题文件。")}</p></div>{/if}
    </section>
  </div>
{/if}

{#if pendingPath !== null}
  <ModalDialog title={$ui("有未保存的内容")} description={$ui("打开另一个文件前，请保存或放弃当前文件的修改。")} onClose={() => !switchBusy && (pendingPath = null)}>
    <svelte:fragment slot="actions"><button class="button" type="button" disabled={switchBusy} on:click={() => (pendingPath = null)}>{$ui("取消")}</button><button class="button" type="button" disabled={switchBusy} on:click={() => resolveSwitch("discard")}>{$ui("放弃修改")}</button><button class="button primary" type="button" disabled={switchBusy} on:click={() => resolveSwitch("save")}>{$ui("保存后继续")}</button></svelte:fragment>
  </ModalDialog>
{/if}
{#if compare}
  <ModalDialog title={$ui("比较文件版本")} description={compare.path} onClose={() => !compareBusy && (compare = null)}>
    <label class="field"><span>{$ui("当前编辑内容")}</span><textarea class="input" rows="8" readonly value={state.content}></textarea></label>
    <label class="field"><span>{$ui("磁盘内容")}</span><textarea class="input" rows="8" readonly value={compare.content}></textarea></label>
    {#if error}<p role="alert">{$ui(error)}</p>{/if}
    <svelte:fragment slot="actions"><button class="button" type="button" disabled={compareBusy} on:click={() => (compare = null)}>{$ui("取消")}</button><button class="button" type="button" disabled={compareBusy} on:click={() => resolveComparison(false)}>{$ui("使用磁盘内容")}</button><button class="button primary" type="button" disabled={compareBusy || !compare.editable} on:click={() => resolveComparison(true)}>{$ui("保留编辑并保存")}</button></svelte:fragment>
  </ModalDialog>
{/if}

<style>
  .files-page { display: flex; height: 100%; min-height: 0; }
  .file-explorer { display: flex; flex-direction: column; width: 280px; min-width: 220px; max-width: 36%; flex-shrink: 0; border-right: 1px solid var(--border-subtle); background: var(--bg-control); }
  .explorer-heading { display: flex; align-items: center; justify-content: space-between; padding: 16px 14px 8px; }
  .explorer-heading strong { font-size: 14px; }
  .explorer-project { padding: 0 14px 12px; font-size: 12px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--text-secondary); }
  .path-search { position: relative; display: flex; align-items: center; margin: 0 10px 10px; }
  .path-search :global(svg) { position: absolute; left: 8px; color: var(--text-tertiary); }
  .path-search input { min-width: 0; width: 100%; font-size: 11px; padding-left: 29px; }
  .file-tree { flex: 1; overflow: auto; min-height: 0; padding: 4px 0; }
  .file-tree-row { display: flex; align-items: center; gap: 5px; width: 100%; height: 30px; border: 0; background: transparent; color: var(--text-secondary); text-align: left; padding-right: 10px; cursor: pointer; }
  .file-tree-row:hover { background: var(--bg-control-hover); }
  .file-tree-row.selected { background: var(--accent-soft); color: var(--accent); }
  .file-tree-row span:not(.file-spacer) { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 12px; }
  .file-tree-row :global(svg), .file-spacer { flex-shrink: 0; }
  .file-spacer { width: 12px; }
  .file-tree-row :global(.folder-expanded) { transform: rotate(90deg); }
  .explorer-hint { padding: 10px 14px; color: var(--text-secondary); font-size: 12px; line-height: 1.6; margin: 0; }
  .file-workspace { flex: 1; display: flex; flex-direction: column; min-width: 0; min-height: 0; }
  .file-editor-heading { display: flex; align-items: center; flex-wrap: wrap; gap: 8px; border-bottom: 1px solid var(--border-subtle); padding: 12px 16px; }
  .file-editor-heading > div { flex: 1; min-width: 150px; }
  .file-editor-heading strong { display: block; font-size: 13px; font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .file-editor-heading span { font-size: 11px; color: var(--text-tertiary); }
  .file-error, .file-readonly { display: flex; align-items: center; justify-content: space-between; gap: 10px; padding: 10px 16px; font-size: 12px; background: var(--danger-soft); color: var(--danger); }
  .file-readonly { background: var(--bg-control); color: var(--text-secondary); }
  .file-footer { display: flex; justify-content: space-between; gap: 12px; padding: 8px 16px; border-top: 1px solid var(--border-subtle); font-size: 12px; color: var(--text-secondary); }
  .files-empty { display: flex; flex: 1; height: 100%; min-height: 0; justify-content: center; align-items: center; flex-direction: column; padding: 36px; gap: 12px; text-align: center; color: var(--text-secondary); }
  .files-empty h1, .files-empty h2 { font-size: 19px; margin: 0; color: var(--text-primary); }
  .files-empty p { max-width: 480px; margin: 0; font-size: 13px; line-height: 1.7; }
</style>
