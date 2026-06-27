<script lang="ts">
  import { Play, Trash2, X } from "@lucide/svelte";

  export let projectPath = "";
  export let output: string[] = [];
  export let running = false;
  export let onRun: (command: string) => void | Promise<void> = () => {};
  export let onClear: () => void = () => {};
  export let onClose: () => void = () => {};

  let commandText = "";

  async function run() {
    const value = commandText.trim();
    if (!value || running) return;
    await onRun(value);
    commandText = "";
  }
</script>

<section class="terminal-panel">
  <div class="panel-header compact">
    <div>
      <h2>终端</h2>
      <span>{projectPath || "未打开项目"}</span>
    </div>
    <div class="compact-actions">
      <button title="清空终端" on:click={onClear}><Trash2 size={16} /></button>
      <button title="关闭终端" on:click={onClose}><X size={16} /></button>
    </div>
  </div>

  <pre class="terminal-output">{output.length ? output.join("\n\n") : "输入命令后会在当前 Hexo 项目目录执行。"}</pre>

  <div class="terminal-input-row">
    <span>$</span>
    <input
      value={commandText}
      placeholder={projectPath ? "例如 hexo -v" : "请先打开 Hexo 项目"}
      disabled={!projectPath || running}
      on:input={(event) => (commandText = event.currentTarget.value)}
      on:keydown={(event) => event.key === "Enter" && run()}
    />
    <button disabled={!projectPath || !commandText.trim() || running} on:click={run}>
      <Play size={15} />{running ? "执行中" : "运行"}
    </button>
  </div>
</section>
