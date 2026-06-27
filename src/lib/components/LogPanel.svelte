<script lang="ts">
  export let logs: string[] = [];
  export let onClear: () => void = () => {};
</script>

<section class="log-panel">
  <div class="panel-header compact">
    <div>
      <h2>日志</h2>
      <span>Hexo / Git 输出</span>
    </div>
    <button on:click={onClear}>清空</button>
  </div>
  <pre>{logs.length ? logs.join("\n\n") : "暂无命令输出。"}</pre>
</section>
