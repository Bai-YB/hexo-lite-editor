<script lang="ts">
  import { onDestroy, onMount, tick } from "svelte";
  import { fade } from "svelte/transition";
  import {
    ChevronDown,
    ChevronLeft,
    ChevronRight,
    Copy,
    File,
    FileArchive,
    FileText,
    Folder,
    FolderOpen,
    Image as ImageIcon,
    Import,
    Maximize2,
    MoreHorizontal,
    Music,
    RefreshCw,
    Search,
    Trash2,
    Upload,
    Video,
    X,
    ZoomIn,
    ZoomOut
  } from "@lucide/svelte";
  import PageHeader from "$shared/components/PageHeader.svelte";
  import EmptyState from "$shared/components/EmptyState.svelte";
  import ErrorState from "$shared/components/ErrorState.svelte";
  import ModalDialog from "$shared/components/ModalDialog.svelte";
  import { normalizeError, platform } from "$platform/tauri";
  import { assetKindLabel, nextLightboxIndex, shouldLoadCloudflare } from "./model";
  import type {
    AppConfigV3,
    CredentialStatus,
    LocalImage,
    ProjectSessionView,
    RemoteAssetBreadcrumb,
    RemoteAssetItem,
    RemoteAssetKind
  } from "$shared/types/app";

  export let session: ProjectSessionView | null;
  export let config: AppConfigV3;
  export let onNotice: (message: string) => void = () => {};
  export let onOpenSettings: () => void = () => {};

  type Provider = "local" | "cloudflare-imgbed";
  type AssetView =
    | { source: "local"; id: string; kind: "image"; name: string; reference: string; previewUrl: string; size: number; item: LocalImage }
    | { source: "remote"; id: string; kind: RemoteAssetKind; name: string; reference?: string; previewUrl?: string; size?: number; directory: string; item: RemoteAssetItem };

  let provider: Provider = config.imageBed.defaultProvider;
  let sourceMenuOpen = false;
  let localImages: LocalImage[] = [];
  let remoteAssets: RemoteAssetItem[] = [];
  let breadcrumbs: RemoteAssetBreadcrumb[] = [{ name: "根目录", directory: "" }];
  let remoteTotal = 0;
  let remoteOffset = 0;
  const pageSize = 48;
  let directory = "";
  let query = "";
  let appliedQuery = "";
  let loading = false;
  let working = false;
  let error = "";
  let credential: CredentialStatus = { configured: false };
  let credentialReady = false;
  let deleting: AssetView | null = null;
  let context: { asset: AssetView; x: number; y: number; opener: HTMLElement } | null = null;
  let contextMenu: HTMLDivElement;
  let loadedKey = "";
  let lightboxIndex = -1;
  let lightboxZoom = 1;
  let lightboxX = 0;
  let lightboxY = 0;
  let lightboxDragging = false;
  let lightboxStart = { x: 0, y: 0, imageX: 0, imageY: 0 };
  let lightboxCloseButton: HTMLButtonElement;
  let lightboxDialog: HTMLDivElement;
  let lightboxStage: HTMLDivElement;
  let lightboxImage: HTMLImageElement;
  let lightboxReturnFocus: HTMLElement | null = null;
  let pageElement: HTMLDivElement;

  $: localAssets = localImages.map((image): AssetView => ({
    source: "local",
    id: image.imageId,
    kind: "image",
    name: image.name,
    reference: image.markdownUrl,
    previewUrl: image.previewUrl,
    size: image.size,
    item: image
  }));
  $: filteredLocal = localAssets.filter((asset) => {
    const needle = query.trim().toLocaleLowerCase();
    return !needle || asset.name.toLocaleLowerCase().includes(needle) || (asset.reference ?? "").toLocaleLowerCase().includes(needle);
  });
  $: visibleAssets = provider === "local"
    ? filteredLocal
    : remoteAssets.map((item): AssetView => ({
        source: "remote",
        id: item.assetId,
        kind: item.kind,
        name: item.fileName || item.name,
        reference: item.url,
        previewUrl: item.previewUrl,
        size: item.size,
        directory: item.directory,
        item
      }));
  $: previewableAssets = visibleAssets.filter((asset) => asset.kind === "image" && asset.previewUrl);
  $: lightboxAsset = lightboxIndex >= 0 ? previewableAssets[lightboxIndex] : undefined;
  $: if (session && credentialReady) {
    const key = `${session.projectId}:${session.generation}:${provider}:${credential.configured}:${config.imageBed.localImageDir}:${config.imageBed.localMarkdownPrefix}:${config.imageBed.cloudflareApiUrl}:${directory}:${appliedQuery}:${remoteOffset}`;
    if (loadedKey !== key) {
      loadedKey = key;
      void loadCurrent();
    }
  } else if (!session) {
    loadedKey = "";
    localImages = [];
    remoteAssets = [];
  }

  onMount(async () => {
    window.addEventListener("pointerdown", closeFloatingMenus);
    window.addEventListener("keydown", handleWindowKeydown);
    await refreshCredential();
  });

  onDestroy(() => {
    window.removeEventListener("pointerdown", closeFloatingMenus);
    window.removeEventListener("keydown", handleWindowKeydown);
  });

  function closeFloatingMenus(event: Event) {
    const target = event.target as HTMLElement;
    if (!target.closest?.(".asset-context-menu")) context = null;
    if (!target.closest?.(".source-switcher-wrap")) sourceMenuOpen = false;
  }

  function handleWindowKeydown(event: KeyboardEvent) {
    if (lightboxAsset) {
      if (event.key === "Tab") {
        const focusable = Array.from(lightboxDialog?.querySelectorAll<HTMLButtonElement>("button:not(:disabled)") ?? []);
        if (focusable.length) {
          const current = focusable.indexOf(document.activeElement as HTMLButtonElement);
          const next = event.shiftKey ? (current <= 0 ? focusable.length - 1 : current - 1) : (current + 1) % focusable.length;
          event.preventDefault();
          focusable[next].focus();
        }
      } else if (event.key === "Escape") closeLightbox();
      else if (event.key === "ArrowLeft") stepLightbox(-1);
      else if (event.key === "ArrowRight") stepLightbox(1);
      return;
    }
    if (context && contextMenu) {
      const items = Array.from(contextMenu.querySelectorAll<HTMLButtonElement>('button:not(:disabled)'));
      const current = items.indexOf(document.activeElement as HTMLButtonElement);
      if (event.key === "ArrowDown" || event.key === "ArrowUp") {
        event.preventDefault();
        const offset = event.key === "ArrowDown" ? 1 : -1;
        items[(current + offset + items.length) % items.length]?.focus();
        return;
      }
      if (event.key === "Home" || event.key === "End") {
        event.preventDefault();
        items[event.key === "Home" ? 0 : items.length - 1]?.focus();
        return;
      }
      if (event.key === "Tab") {
        event.preventDefault();
        const opener = context.opener;
        context = null;
        requestAnimationFrame(() => opener.focus());
        return;
      }
    }
    if (event.key === "Escape") {
      const opener = context?.opener;
      context = null;
      sourceMenuOpen = false;
      requestAnimationFrame(() => opener?.focus());
    }
  }

  async function refreshCredential() {
    try {
      credential = await platform.credentialStatus(
        config.imageBed.cloudflareConnectionId,
        config.imageBed.cloudflareApiUrl
      );
    } catch {
      credential = { configured: false };
    } finally {
      credentialReady = true;
      loadedKey = "";
    }
  }

  async function loadCurrent() {
    if (!session) return;
    loading = true;
    error = "";
    try {
      if (provider === "local") {
        localImages = await platform.listLocalImages(session.projectId, session.generation);
      } else if (shouldLoadCloudflare({ sessionReady: true, credentialReady, credentialConfigured: credential.configured, apiUrl: config.imageBed.cloudflareApiUrl })) {
        const page = await platform.listCloudflareAssets(
          session.projectId,
          session.generation,
          remoteOffset,
          pageSize,
          appliedQuery,
          directory
        );
        remoteAssets = page.items;
        breadcrumbs = page.breadcrumbs;
        remoteTotal = page.totalCount;
        directory = page.currentDirectory;
      } else {
        remoteAssets = [];
        remoteTotal = 0;
      }
    } catch (value) {
      error = normalizeError(value).message;
    } finally {
      loading = false;
    }
  }

  function selectProvider(next: Provider) {
    provider = next;
    sourceMenuOpen = false;
    directory = "";
    query = "";
    appliedQuery = "";
    remoteOffset = 0;
    loadedKey = "";
  }

  async function importOrUpload() {
    if (!session || working) return;
    working = true;
    try {
      if (provider === "local") {
        localImages = await platform.importLocalImages(session.projectId, session.generation);
        onNotice(`图片已导入 ${config.imageBed.localImageDir}。`);
      } else {
        const result = await platform.uploadCloudflareImage(session.projectId, session.generation);
        if (result) {
          onNotice("图片已上传到 Cloudflare-ImgBed。");
          loadedKey = "";
        }
      }
    } catch (value) {
      onNotice(normalizeError(value).message);
    } finally {
      working = false;
    }
  }

  function clearSearch() {
    query = "";
    appliedQuery = "";
    remoteOffset = 0;
    loadedKey = "";
  }

  function applySearch() {
    if (provider === "local") return;
    appliedQuery = query.trim();
    remoteOffset = 0;
    loadedKey = "";
  }

  function enterDirectory(next: string) {
    directory = next;
    appliedQuery = "";
    query = "";
    remoteOffset = 0;
    loadedKey = "";
  }

  function activateAsset(asset: AssetView, opener: HTMLElement) {
    if (asset.kind === "folder" && asset.source === "remote") {
      enterDirectory(asset.directory);
    } else if (asset.kind === "image" && asset.previewUrl) {
      openLightbox(asset, opener);
    }
  }

  function handleAssetKeydown(event: KeyboardEvent, asset: AssetView) {
    if ((event.shiftKey && event.key === "F10") || event.key === "ContextMenu") {
      showContext(event, asset, event.currentTarget as HTMLElement);
    } else if (event.key === "Enter") {
      event.preventDefault();
      activateAsset(asset, event.currentTarget as HTMLElement);
    }
  }

  async function showContext(event: MouseEvent | KeyboardEvent, asset: AssetView, opener: HTMLElement) {
    event.preventDefault();
    event.stopPropagation();
    const rect = opener.getBoundingClientRect();
    const x = event instanceof MouseEvent && event.clientX ? event.clientX : rect.right - 20;
    const y = event instanceof MouseEvent && event.clientY ? event.clientY : rect.top + 28;
    context = { asset, x: Math.min(x, window.innerWidth - 220), y: Math.min(y, window.innerHeight - 210), opener };
    await tick();
    contextMenu?.querySelector<HTMLButtonElement>("button")?.focus();
  }

  function markdownFor(asset: AssetView) {
    return `![${asset.name.replace(/]/g, "\\]")}](${asset.reference ?? ""})`;
  }

  async function copyText(value: string, message: string) {
    try {
      await platform.writeClipboard(value);
      const opener = context?.opener;
      context = null;
      opener?.focus();
      onNotice(message);
    } catch (value) {
      onNotice(normalizeError(value).message);
    }
  }

  async function revealLocal(asset: AssetView) {
    if (!session || asset.source !== "local") return;
    context = null;
    try {
      await platform.revealLocalImage(session.projectId, session.generation, asset.item.imageId);
    } catch (value) {
      onNotice(normalizeError(value).message);
    }
  }

  async function removeAsset() {
    if (!session || !deleting) return;
    try {
      if (deleting.source === "local") {
        await platform.deleteLocalImage(session.projectId, session.generation, deleting.item.imageId);
        localImages = localImages.filter((image) => image.imageId !== deleting?.id);
        onNotice("图片已移动到系统回收站。");
      } else {
        await platform.deleteCloudflareAsset(session.projectId, session.generation, deleting.item.assetId);
        remoteAssets = remoteAssets.filter((asset) => asset.assetId !== deleting?.id);
        remoteTotal = Math.max(0, remoteTotal - 1);
        onNotice("远程资源已删除。");
      }
      deleting = null;
    } catch (value) {
      onNotice(normalizeError(value).message);
    }
  }

  async function openLightbox(asset: AssetView, opener: HTMLElement) {
    const index = previewableAssets.findIndex((item) => item.id === asset.id);
    if (index < 0) return;
    lightboxReturnFocus = opener;
    lightboxIndex = index;
    resetLightboxView();
    await tick();
    lightboxCloseButton?.focus();
  }

  function closeLightbox() {
    lightboxIndex = -1;
    lightboxZoom = 1;
    requestAnimationFrame(() => lightboxReturnFocus?.focus());
  }

  function stepLightbox(delta: number) {
    if (!previewableAssets.length) return;
    lightboxIndex = nextLightboxIndex(lightboxIndex, delta, previewableAssets.length);
    resetLightboxView();
  }

  function resetLightboxView() {
    lightboxZoom = 1;
    lightboxX = 0;
    lightboxY = 0;
  }

  function zoomLightbox(delta: number, event?: WheelEvent) {
    const previousZoom = lightboxZoom;
    const nextZoom = Math.min(5, Math.max(0.25, Number((lightboxZoom + delta).toFixed(2))));
    if (event && lightboxStage && previousZoom > 0) {
      const bounds = lightboxStage.getBoundingClientRect();
      const pointerX = event.clientX - (bounds.left + bounds.width / 2);
      const pointerY = event.clientY - (bounds.top + bounds.height / 2);
      const ratio = nextZoom / previousZoom;
      lightboxX = pointerX - (pointerX - lightboxX) * ratio;
      lightboxY = pointerY - (pointerY - lightboxY) * ratio;
    }
    lightboxZoom = nextZoom;
    if (lightboxZoom <= 1) {
      lightboxX = 0;
      lightboxY = 0;
    }
    requestAnimationFrame(clampLightboxPosition);
  }

  function toggleLightboxZoom(event: MouseEvent) {
    if (lightboxZoom > 1) {
      resetLightboxView();
      return;
    }
    zoomLightbox(1, event as WheelEvent);
  }

  function clampLightboxPosition() {
    if (!lightboxStage || !lightboxImage || lightboxZoom <= 1) {
      if (lightboxZoom <= 1) {
        lightboxX = 0;
        lightboxY = 0;
      }
      return;
    }
    const stage = lightboxStage.getBoundingClientRect();
    const image = lightboxImage.getBoundingClientRect();
    const baseWidth = image.width / lightboxZoom;
    const baseHeight = image.height / lightboxZoom;
    const maxX = Math.max(0, (baseWidth * lightboxZoom - stage.width) / 2);
    const maxY = Math.max(0, (baseHeight * lightboxZoom - stage.height) / 2);
    lightboxX = Math.min(maxX, Math.max(-maxX, lightboxX));
    lightboxY = Math.min(maxY, Math.max(-maxY, lightboxY));
  }

  function startLightboxDrag(event: PointerEvent) {
    if (lightboxZoom <= 1) return;
    lightboxDragging = true;
    lightboxStart = { x: event.clientX, y: event.clientY, imageX: lightboxX, imageY: lightboxY };
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
  }

  function moveLightboxDrag(event: PointerEvent) {
    if (!lightboxDragging) return;
    lightboxX = lightboxStart.imageX + event.clientX - lightboxStart.x;
    lightboxY = lightboxStart.imageY + event.clientY - lightboxStart.y;
    clampLightboxPosition();
  }

  function changeRemotePage(offset: number) {
    remoteOffset = Math.max(0, offset);
    loadedKey = "";
    pageElement?.scrollTo({ top: 0 });
  }

  function formatBytes(value?: number) {
    if (value == null) return "";
    if (value < 1024) return `${value} B`;
    if (value < 1024 * 1024) return `${(value / 1024).toFixed(1)} KB`;
    return `${(value / 1024 / 1024).toFixed(1)} MB`;
  }

  function iconFor(kind: RemoteAssetKind) {
    if (kind === "folder") return Folder;
    if (kind === "archive") return FileArchive;
    if (kind === "document") return FileText;
    if (kind === "audio") return Music;
    if (kind === "video") return Video;
    if (kind === "image") return ImageIcon;
    return File;
  }
</script>

<div bind:this={pageElement} class="workspace-page image-bed-page">
  <PageHeader title="图床" description="按目录浏览本地图片和 Cloudflare-ImgBed 资源。">
    <div class="source-switcher-wrap">
      <button class="source-switcher" type="button" aria-expanded={sourceMenuOpen} on:click={() => (sourceMenuOpen = !sourceMenuOpen)}><span>{provider === "local" ? "本地图片" : "Cloudflare-ImgBed"}</span><ChevronDown size={14} /></button>
      {#if sourceMenuOpen}<div class="source-menu quiet-menu"><button class:active={provider === "local"} type="button" on:click={() => selectProvider("local")}>本地图片<small>{config.imageBed.localImageDir}</small></button><button class:active={provider === "cloudflare-imgbed"} type="button" on:click={() => selectProvider("cloudflare-imgbed")}>Cloudflare-ImgBed<small>远程目录与文件</small></button></div>{/if}
    </div>
    <button class="icon-button" type="button" disabled={!session || loading} title="刷新" aria-label="刷新资源" on:click={() => { loadedKey = ""; }}><RefreshCw size={16} /></button>
    <button class="button primary" type="button" disabled={!session || working || (provider === "cloudflare-imgbed" && !credential.configured)} on:click={importOrUpload}>{#if working}<RefreshCw size={16} class="spin" />{provider === "local" ? "导入中" : "上传中"}{:else if provider === "local"}<Import size={16} />导入{:else}<Upload size={16} />上传图片{/if}</button>
  </PageHeader>

  {#if !session}
    <EmptyState title="请先打开项目" description="图片工作区与当前 Hexo 项目会话绑定。" />
  {:else}
    <form class="image-toolbar" on:submit|preventDefault={applySearch}>
      <div class="search-control"><Search size={15} /><input bind:value={query} aria-label="搜索资源" placeholder="搜索文件名（远程搜索会递归目录）" />{#if query}<button class="search-clear" type="button" aria-label="清除搜索" on:click={clearSearch}><X size={14} /></button>{/if}</div>
      {#if provider === "cloudflare-imgbed"}<button class="button quiet" type="submit">搜索</button>{/if}
      <span class="image-count">{provider === "local" ? filteredLocal.length : remoteTotal} 项</span>
    </form>

    {#if provider === "cloudflare-imgbed" && breadcrumbs.length > 1 && !appliedQuery}
      <nav class="asset-breadcrumbs" aria-label="远程资源路径">{#each breadcrumbs as crumb, index (crumb.directory)}{#if index}<ChevronRight size={13} />{/if}<button class:current={index === breadcrumbs.length - 1} type="button" on:click={() => enterDirectory(crumb.directory)}>{crumb.name}</button>{/each}</nav>
    {:else if provider === "cloudflare-imgbed" && appliedQuery}
      <div class="asset-search-state"><span>搜索：{appliedQuery}</span><button type="button" on:click={clearSearch}>清除</button></div>
    {/if}

    {#if provider === "cloudflare-imgbed" && credentialReady && (!credential.configured || !config.imageBed.cloudflareApiUrl.trim())}
      <EmptyState title="Cloudflare 尚未配置完成" description="请设置 HTTPS API 地址，并将 Token 保存到系统凭据库。"><button class="button primary" type="button" on:click={onOpenSettings}>打开图床设置</button></EmptyState>
    {:else if loading && !visibleAssets.length}
      <div class="image-grid image-grid-skeleton" aria-label="正在读取资源" aria-busy="true">{#each Array(12) as _}<div class="asset-skeleton"><span></span><i></i><i></i></div>{/each}</div>
    {:else if error}
      <ErrorState message={error}><button class="button" type="button" on:click={() => { loadedKey = ""; }}>重试</button></ErrorState>
    {:else if !visibleAssets.length}
      <EmptyState title={query ? "没有匹配的资源" : "当前目录为空"} description={query ? "换一个关键词，或清空搜索条件。" : "可导入或上传 PNG、JPEG、GIF、WebP，单张不超过 25 MB。"}><button class="button primary" type="button" on:click={importOrUpload}>{provider === "local" ? "导入图片" : "上传图片"}</button></EmptyState>
    {:else}
      <div class:loading class="image-results">
      <div class="image-grid" aria-busy={loading}>
        {#each visibleAssets as asset (asset.id)}
          <div class:folder={asset.kind === "folder"} class="asset-item">
            <button class="asset-primary" type="button" aria-label={`${asset.name}，Enter 打开，Shift+F10 打开菜单`} on:click={(event) => activateAsset(asset, event.currentTarget)} on:contextmenu={(event) => showContext(event, asset, event.currentTarget)} on:keydown={(event) => handleAssetKeydown(event, asset)}>
              <span class="asset-thumb">
                {#if asset.kind === "image" && asset.previewUrl}<img src={asset.previewUrl} alt="" loading="lazy" />{:else}<svelte:component this={iconFor(asset.kind)} size={asset.kind === "folder" ? 46 : 38} strokeWidth={1.35} />{/if}
              </span>
              <strong title={asset.name}>{asset.name}</strong><span class="asset-meta">{asset.kind === "folder" ? "文件夹" : formatBytes(asset.size) || assetKindLabel(asset.kind)}</span>
            </button>
            <button class="asset-more" type="button" tabindex="-1" aria-label={`打开 ${asset.name} 菜单`} on:click={(event) => showContext(event, asset, event.currentTarget)}><MoreHorizontal size={16} /></button>
          </div>
        {/each}
      </div>
      {#if provider === "cloudflare-imgbed" && remoteTotal > pageSize}
        <div class="pagination"><button class="button quiet" type="button" disabled={loading || remoteOffset === 0} on:click={() => changeRemotePage(remoteOffset - pageSize)}>上一页</button><span>{Math.floor(remoteOffset / pageSize) + 1} / {Math.ceil(remoteTotal / pageSize)}</span><button class="button quiet" type="button" disabled={loading || remoteOffset + pageSize >= remoteTotal} on:click={() => changeRemotePage(remoteOffset + pageSize)}>下一页</button></div>
      {/if}
      </div>
    {/if}
  {/if}
</div>

{#if context}
  <div bind:this={contextMenu} class="asset-context-menu quiet-menu" role="menu" style={`left:${context.x}px;top:${context.y}px`}>
    {#if context.asset.kind === "image" && context.asset.reference}<button type="button" role="menuitem" on:click={() => copyText(markdownFor(context!.asset), "Markdown 已复制。") }><Copy size={14} />复制 Markdown</button>{/if}
    {#if context.asset.reference}<button type="button" role="menuitem" on:click={() => copyText(context!.asset.reference!, context!.asset.source === "remote" ? "链接已复制。" : "图片引用已复制。") }><Copy size={14} />{context.asset.source === "remote" ? "复制链接" : "复制 Markdown 路径"}</button>{/if}
    {#if context.asset.source === "local"}<button type="button" role="menuitem" on:click={() => revealLocal(context!.asset)}><FolderOpen size={14} />在文件夹中显示</button>{/if}
    {#if context.asset.kind === "image"}<button type="button" role="menuitem" on:click={() => { const asset = context!.asset; const opener = context!.opener; context = null; void openLightbox(asset, opener); }}><Maximize2 size={14} />查看大图</button>{/if}
    {#if context.asset.kind !== "folder"}<div class="menu-separator"></div><button class="danger" type="button" role="menuitem" on:click={() => { deleting = context!.asset; context = null; }}><Trash2 size={14} />{context.asset.source === "local" ? "移到回收站" : "删除远程资源"}</button>{/if}
  </div>
{/if}

{#if lightboxAsset}
  <div bind:this={lightboxDialog} class="image-lightbox" role="dialog" aria-modal="true" aria-label={`查看 ${lightboxAsset.name}`} transition:fade={{ duration: 120 }}>
    <div class="lightbox-toolbar"><span>{lightboxAsset.name}</span><div><span class="lightbox-position">{lightboxIndex + 1} / {previewableAssets.length}</span><button class="icon-button inverse" type="button" aria-label="缩小" on:click={() => zoomLightbox(-0.25)}><ZoomOut size={18} /></button><button class="lightbox-zoom" type="button" on:click={resetLightboxView}>{Math.round(lightboxZoom * 100)}%</button><button class="icon-button inverse" type="button" aria-label="放大" on:click={() => zoomLightbox(0.25)}><ZoomIn size={18} /></button><button bind:this={lightboxCloseButton} class="icon-button inverse" type="button" aria-label="关闭" on:click={closeLightbox}><X size={19} /></button></div></div>
    {#if previewableAssets.length > 1}<button class="lightbox-nav previous" type="button" aria-label="上一张" on:click={() => stepLightbox(-1)}><ChevronLeft size={28} /></button><button class="lightbox-nav next" type="button" aria-label="下一张" on:click={() => stepLightbox(1)}><ChevronRight size={28} /></button>{/if}
    <div bind:this={lightboxStage} class:dragging={lightboxDragging} class="lightbox-stage" role="presentation" on:dblclick={toggleLightboxZoom} on:wheel|preventDefault={(event) => zoomLightbox(event.deltaY < 0 ? 0.15 : -0.15, event)} on:pointerdown={startLightboxDrag} on:pointermove={moveLightboxDrag} on:pointerup={() => (lightboxDragging = false)} on:pointercancel={() => (lightboxDragging = false)}>
      <img bind:this={lightboxImage} src={lightboxAsset.previewUrl} alt={lightboxAsset.name} draggable="false" style={`transform:translate(${lightboxX}px, ${lightboxY}px) scale(${lightboxZoom})`} />
    </div>
  </div>
{/if}

{#if deleting}
  <ModalDialog title={deleting.source === "local" ? "移到回收站？" : "删除远程资源？"} description={deleting.source === "local" ? `${deleting.name} 将被移动到系统回收站。` : `${deleting.name} 将从 Cloudflare-ImgBed 永久删除。`} onClose={() => (deleting = null)}>
    <svelte:fragment slot="actions"><button class="button" type="button" data-autofocus={deleting.source === "remote" ? "" : undefined} on:click={() => (deleting = null)}>取消</button><button class="button danger" type="button" data-autofocus={deleting.source === "local" ? "" : undefined} on:click={removeAsset}>{deleting.source === "local" ? "移到回收站" : "确认删除"}</button></svelte:fragment>
  </ModalDialog>
{/if}
