<script lang="ts">
  export let projectName = "未打开项目";
  export let fileName = "未选择文章";
  export let saveStatus = "已保存";
  export let words = 0;
  export let lastSavedAt = "";
</script>

<footer class="status-bar">
  <span>{projectName}</span>
  <span>{fileName}</span>
  <span>{saveStatus}</span>
  <span>{words} 字</span>
  <span>{lastSavedAt ? `最后保存 ${lastSavedAt}` : "尚未保存"}</span>
</footer>
