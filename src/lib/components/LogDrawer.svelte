<script lang="ts">
  import { ClipboardCopy, Trash2, X } from "@lucide/svelte";

  export let logs: string[] = [];
  export let onClear: () => void = () => {};
  export let onClose: () => void = () => {};

  $: logText = logs.length ? logs.join("\n\n") : "暂无命令输出。";

  async function copyLogs() {
    await navigator.clipboard.writeText(logText);
  }
</script>

<aside class="log-drawer">
  <div class="settings-panel-header">
    <div>
      <h2>日志</h2>
      <span>Hexo / Git / 图床输出</span>
    </div>
    <button class="icon-only" title="关闭" on:click={onClose}><X size={18} /></button>
  </div>

  <div class="button-row">
    <button on:click={copyLogs}><ClipboardCopy size={16} />复制</button>
    <button on:click={onClear}><Trash2 size={16} />清空</button>
  </div>

  <pre class="drawer-log-output">{logText}</pre>
</aside>
