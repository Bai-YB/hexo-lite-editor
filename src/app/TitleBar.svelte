<script lang="ts">
  import { onMount } from "svelte";
  import { Minus, Square, X } from "@lucide/svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { isTauri } from "$platform/tauri";
  import { isMacOS } from "$platform/os";
  import { translate } from "$shared/i18n";

  export let onRequestClose: () => void;
  export let onMaximizedChange: (value: boolean) => void = () => {};
  export let documentTitle = "";
  export let dirty = false;

  let maximized = false;

  onMount(() => {
    if (!isTauri()) return;
    const window = getCurrentWindow();
    const sync = async () => {
      maximized = await window.isMaximized();
      onMaximizedChange(maximized);
    };
    void sync();
    const unlistenPromise = window.onResized(() => void sync());
    return () => {
      void unlistenPromise.then((unlisten) => unlisten());
    };
  });

  async function minimize() {
    if (isTauri()) await getCurrentWindow().minimize();
  }

  async function toggleMaximize() {
    if (!isTauri()) return;
    await getCurrentWindow().toggleMaximize();
    maximized = await getCurrentWindow().isMaximized();
    onMaximizedChange(maximized);
  }

</script>

<div class:macos={isMacOS} class="titlebar">
  <div
    class="titlebar-brand"
    data-tauri-drag-region
    role="presentation"
    on:dblclick={toggleMaximize}
  >
    <img src="/favicon.png" alt="" />
    <span>Hexo Lite Editor</span>
    {#if documentTitle}<span class="titlebar-document">· {documentTitle}</span>{/if}
    {#if dirty}<span class="titlebar-dirty" aria-label={$translate("window.unsaved")} title={$translate("window.unsaved")}></span>{/if}
  </div>
  <div
    class="titlebar-drag"
    data-tauri-drag-region
    role="presentation"
    on:dblclick={toggleMaximize}
  ></div>
  <div class="window-controls" aria-hidden={isMacOS}>
    <button class="window-control" type="button" aria-label={$translate("window.minimize")} title={$translate("window.minimize")} on:click={minimize}>
      <Minus size={16} />
    </button>
    <button
      class="window-control"
      type="button"
      aria-label={maximized ? $translate("window.restore") : $translate("window.maximize")}
      title={maximized ? $translate("window.restore") : $translate("window.maximize")}
      on:click={toggleMaximize}
    >
      {#if maximized}<span class="restore-window-icon" aria-hidden="true"></span>{:else}<Square size={13} />{/if}
    </button>
    <button class="window-control close" type="button" aria-label={$translate("window.close")} title={$translate("window.close")} on:click={onRequestClose}>
      <X size={16} />
    </button>
  </div>
</div>
