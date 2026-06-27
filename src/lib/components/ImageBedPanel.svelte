<script lang="ts">
  import { onMount } from "svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import { ClipboardPaste, Copy, FileImage, KeyRound, RefreshCw, Search, Trash2, UploadCloud, X } from "@lucide/svelte";
  import {
    deleteCloudflareImgBedImage,
    listCloudflareImgBedImages,
    openCloudflareImgBedAdmin,
    uploadImagePathToCloudflareImgBed
  } from "$lib/api/uploader";
  import type { AppSettings } from "$lib/types/settings";
  import type { ImageBedItem } from "$lib/types/uploader";

  export let settings: AppSettings;
  export let onClose: () => void = () => {};
  export let onInsertMarkdown: (markdown: string) => void = () => {};
  export let onChange: (settings: AppSettings) => void | Promise<void> = () => {};
  export let onLog: (message: string) => void = () => {};

  let loading = false;
  let uploading = false;
  let deletingId = "";
  let message = "";
  let search = "";
  let files: ImageBedItem[] = [];
  let totalCount = 0;
  let contextMenu:
    | {
        x: number;
        y: number;
        item: ImageBedItem;
      }
    | null = null;

  $: configured =
    settings.uploader.defaultType === "cloudflare-imgbed" &&
    settings.uploader.apiUrl.trim().length > 0 &&
    settings.uploader.token.trim().length > 0;

  onMount(() => {
    if (configured) void refreshImages();
  });

  async function refreshImages() {
    contextMenu = null;
    if (!configured) {
      message = "请在设置中选择 CloudFlare-ImgBed，并填写 API 地址与 Token。";
      files = [];
      totalCount = 0;
      return;
    }

    loading = true;
    message = "正在读取图床图片...";
    try {
      const result = await listCloudflareImgBedImages(
        settings.uploader.apiUrl,
        settings.uploader.token,
        0,
        50,
        search,
        ""
      );
      files = result.files;
      totalCount = result.totalCount;
      message = files.length ? `已加载 ${files.length} / ${totalCount || files.length} 张图片。` : "图床中暂无图片。";
    } catch (error) {
      message = `读取图床失败: ${error}`;
    } finally {
      loading = false;
    }
  }

  async function uploadImage() {
    if (!configured) {
      message = "请先配置 CloudFlare-ImgBed API 地址与 Token。";
      return;
    }
    const selected = await open({
      multiple: false,
      title: "选择要上传到图床的图片",
      filters: [{ name: "Images", extensions: ["png", "jpg", "jpeg", "gif", "webp", "svg", "ico"] }]
    });
    if (typeof selected !== "string") return;

    uploading = true;
    message = "正在上传图片...";
    try {
      const result = await uploadImagePathToCloudflareImgBed(settings.uploader.apiUrl, settings.uploader.token, selected);
      onLog(`Image uploaded to CloudFlare-ImgBed: ${result.url}`);
      if (settings.uploader.autoInsertMarkdown) onInsertMarkdown(`\n${result.markdown}\n`);
      await refreshImages();
      message = `上传成功: ${result.url}`;
    } catch (error) {
      message = `上传失败: ${error}`;
    } finally {
      uploading = false;
    }
  }

  async function openTokenPage() {
    try {
      await openCloudflareImgBedAdmin(settings.uploader.apiUrl);
      message = "已打开图床后台，请创建具备 upload/list/delete 权限的 API Token。";
      onLog("CloudFlare-ImgBed admin page opened for token creation.");
    } catch (error) {
      message = `打开图床后台失败: ${error}`;
    }
  }

  async function fillTokenFromClipboard() {
    try {
      const token = (await navigator.clipboard.readText()).trim();
      if (!token) {
        message = "剪贴板里没有可用的 Token。";
        return;
      }
      const nextSettings = {
        ...settings,
        uploader: {
          ...settings.uploader,
          defaultType: "cloudflare-imgbed" as const,
          token
        }
      };
      settings = nextSettings;
      await onChange(nextSettings);
      message = "Token 已从剪贴板填入并保存。";
      onLog("CloudFlare-ImgBed token filled from clipboard.");
      if (settings.uploader.apiUrl.trim()) void refreshImages();
    } catch (error) {
      message = `读取剪贴板失败: ${error}`;
    }
  }

  function openContextMenu(event: MouseEvent, item: ImageBedItem) {
    event.preventDefault();
    contextMenu = {
      x: Math.min(event.clientX, window.innerWidth - 190),
      y: Math.min(event.clientY, window.innerHeight - 150),
      item
    };
  }

  async function copyText(value: string, label: string) {
    await navigator.clipboard.writeText(value);
    message = `${label} 已复制。`;
    onLog(`${label} copied from image bed.`);
    contextMenu = null;
  }

  async function deleteImage(item: ImageBedItem) {
    contextMenu = null;
    if (!confirm(`确定从图床中删除这张图片吗？\n${item.name}`)) return;
    deletingId = item.id;
    message = "正在删除图片...";
    try {
      await deleteCloudflareImgBedImage(settings.uploader.apiUrl, settings.uploader.token, item.id);
      files = files.filter((file) => file.id !== item.id);
      totalCount = Math.max(0, totalCount - 1);
      message = "图片已从图床删除。";
      onLog(`Image deleted from CloudFlare-ImgBed: ${item.id}`);
    } catch (error) {
      message = `删除失败: ${error}`;
    } finally {
      deletingId = "";
    }
  }

  function markdownFor(item: ImageBedItem) {
    return `![${item.fileName || "image"}](${item.url})`;
  }

  function displayDate(value: string) {
    if (!value) return "";
    const asNumber = Number(value);
    const date = Number.isFinite(asNumber) && asNumber > 0 ? new Date(asNumber) : new Date(value);
    return Number.isNaN(date.getTime()) ? value : date.toLocaleString();
  }
</script>

<svelte:window
  on:click={() => (contextMenu = null)}
  on:keydown={(event) => event.key === "Escape" && (contextMenu = null)}
/>

<aside class="image-bed-panel">
  <div class="image-bed-header">
    <div>
      <h2>图床</h2>
      <span>CloudFlare-ImgBed 图片管理</span>
    </div>
    <button class="icon-only" title="关闭" on:click={onClose}><X size={18} /></button>
  </div>

  <div class="image-bed-tools">
    <div class="image-bed-search">
      <Search size={16} />
      <input
        value={search}
        placeholder="搜索文件名"
        on:input={(event) => (search = event.currentTarget.value)}
        on:keydown={(event) => event.key === "Enter" && refreshImages()}
      />
    </div>
    <button title="刷新" disabled={loading} on:click={refreshImages}><RefreshCw size={16} />刷新</button>
    <button title="打开图床后台获取 Token" on:click={openTokenPage}><KeyRound size={16} />获取 Token</button>
    <button title="从剪贴板填入 Token" on:click={fillTokenFromClipboard}><ClipboardPaste size={16} />粘贴 Token</button>
    <button title="上传图片" disabled={uploading} on:click={uploadImage}><UploadCloud size={16} />上传</button>
  </div>

  {#if !configured}
    <div class="image-bed-empty">请在设置中选择 CloudFlare-ImgBed，并填写 API 地址与 Token。</div>
  {:else}
    <div class="image-bed-status">{message}</div>
    <div class="image-bed-grid">
      {#each files as item}
        <button
          class="image-bed-card"
          class:deleting={deletingId === item.id}
          title={item.name}
          on:contextmenu={(event) => openContextMenu(event, item)}
          on:dblclick={() => onInsertMarkdown(`\n${markdownFor(item)}\n`)}
        >
          <span class="image-bed-thumb">
            {#if item.url}
              <img src={item.url} alt={item.fileName || item.name} loading="lazy" />
            {:else}
              <FileImage size={28} />
            {/if}
          </span>
          <strong>{item.fileName || item.name}</strong>
          <small>{item.fileSize || item.fileType || "未知大小"}</small>
          <small>{displayDate(item.createdAt)}</small>
          <em>{item.channel || "CloudFlare-ImgBed"}</em>
        </button>
      {/each}
    </div>
  {/if}

  {#if contextMenu}
    <div class="image-bed-menu" role="menu" style:left={`${contextMenu.x}px`} style:top={`${contextMenu.y}px`}>
      <button on:click={() => copyText(contextMenu!.item.url, "图片链接")}><Copy size={15} />复制链接</button>
      <button on:click={() => copyText(markdownFor(contextMenu!.item), "Markdown")}><FileImage size={15} />复制 Markdown</button>
      <button class="danger" on:click={() => deleteImage(contextMenu!.item)}><Trash2 size={15} />删除图片</button>
    </div>
  {/if}
</aside>
