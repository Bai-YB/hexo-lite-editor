import { getResolvedLanguage } from "./index";

const ATTRIBUTE_NAMES = ["aria-label", "title", "placeholder"] as const;
let observer: MutationObserver | undefined;

const exact: Record<string, string> = {
  "图床": "Images", "设置": "Settings", "关于": "About", "编辑器": "Editor", "插件": "Plugins",
  "常规": "General", "编辑体验": "Editing", "图片与插件": "Images & plugins", "图片与图床": "Images & plugins", "Hexo 与发布": "Hexo & publishing", "内容同步": "Content sync", "项目同步": "Project sync", "维护": "Maintenance",
  "保存": "Save", "取消": "Cancel", "确认": "Confirm", "关闭": "Close", "重试": "Retry", "删除": "Delete", "卸载": "Uninstall", "启用": "Enable", "禁用": "Disable", "设置插件": "Plugin settings", "测试连接": "Test connection",
  "安装插件": "Install plugin", "设为图床": "Use as image host", "当前图床": "Current image host", "插件设置已保存。": "Plugin settings saved.",
  "确认启用插件": "Enable plugin?", "确认启用": "Enable", "此插件将获得以下权限。只启用你信任的插件：": "This plugin will receive the following permissions. Enable only plugins you trust:",
  "此插件没有可编辑的设置。": "This plugin has no editable settings.", "尚未安装插件。将插件目录安装到应用配置目录后会显示在这里。": "No plugins are installed. Install a plugin directory to add it here.",
  "正在读取插件…": "Loading plugins…", "插件在隔离 Worker 中运行，所有 Host API 请求都需要声明权限。": "Plugins run in isolated workers and every Host API request requires a declared permission.",
  "图片工作流": "Image workflow", "默认来源": "Default provider", "本地图片": "Local images", "本地图片目录": "Local image directory", "插件图床": "Plugin image host", "Cloudflare 连接": "Cloudflare connection",
  "图床名称": "Image host name", "服务地址": "Service URL", "访问 Token": "Access token", "图片插入方式": "Image insertion", "自动": "Automatic",
  "启动": "Startup", "保存与备份": "Save & backup", "启动时打开最近项目": "Open recent project at startup", "自动保存": "Auto save", "自动保存延迟": "Auto-save delay", "保存前创建备份": "Back up before saving", "最近项目": "Recent projects",
  "外观": "Appearance", "主题模式": "Theme mode", "跟随系统": "System", "浅色": "Light", "深色": "Dark", "正文排版": "Typography", "字号": "Font size", "行高": "Line height", "编辑辅助": "Editing aids", "显示行号": "Show line numbers", "自动换行": "Line wrapping", "突出当前行": "Highlight active line", "文章列表封面": "Article covers",
  "浏览器预览": "Browser preview", "预览端口": "Preview port", "打开项目后自动启动预览": "Start preview after opening a project", "预览草稿": "Preview drafts", "发布流水线": "Publishing pipeline", "发布前保存": "Save before publishing", "始终重新生成": "Always regenerate", "部署后 Git Push": "Git push after deploy", "已启用": "Enabled",
  "检查更新": "Check for updates", "检查中": "Checking", "下载更新": "Download update", "重启并安装": "Restart and install", "自动检查并下载更新": "Automatically check and download updates", "稍后": "Later", "当前已经是最新版本。": "You are up to date.",
  "项目同步分支": "Project sync branch", "使用云端最新版本": "Use latest cloud version", "用本机项目覆盖云端": "Replace cloud with this project", "确认同步完整项目": "Confirm full project sync", "全部选择本地": "Choose all local", "全部选择远端": "Choose all remote", "打开备份目录": "Open backups", "确认覆盖": "Confirm replacement", "使用云端最新项目？": "Use the latest cloud project?", "用本机项目覆盖云端？": "Replace the cloud project?",
  "导入": "Import", "导入中": "Importing", "上传中": "Uploading", "上传图片": "Upload images", "搜索": "Search", "清除": "Clear", "上一页": "Previous", "下一页": "Next", "文件夹": "Folder", "图片": "Image", "压缩包": "Archive", "文档": "Document", "音频": "Audio", "视频": "Video", "文件": "File",
  "复制 Markdown": "Copy Markdown", "复制链接": "Copy link", "复制 Markdown 路径": "Copy Markdown path", "在文件夹中显示": "Show in folder", "查看大图": "Preview", "下载": "Download", "重命名": "Rename", "移动": "Move", "删除远程资源": "Delete remote asset", "移到回收站": "Move to Trash",
  "快速预览": "Quick preview", "真实主题": "Theme preview", "真实 Hexo 主题预览": "Real Hexo theme preview", "打开真实预览": "Open real preview", "刷新": "Refresh", "新建文章": "New article", "文章": "Posts", "草稿": "Drafts", "全部": "All", "发布": "Publish", "预览": "Preview", "保存文章": "Save article",
  "请先打开项目": "Open a project first", "请先打开一个 Hexo 项目。": "Open a Hexo project first.", "请先打开博客项目。": "Open a blog project first.", "请先打开一篇文章。": "Open an article first.",
  "有未保存的内容": "Unsaved changes", "退出 Hexo Lite Editor？": "Quit Hexo Lite Editor?", "不保存退出": "Quit without saving", "放弃": "Discard", "处理中": "Working", "保存并退出": "Save and quit", "保存并继续": "Save and continue", "关闭通知": "Dismiss notification",
  "可用": "Available", "不可用": "Unavailable", "打开": "Open", "位置不可用": "Unavailable", "尚无最近项目。": "No recent projects.", "清空": "Clear",
  "根目录": "Root", "目标目录": "Target directory", "新名称": "New name", "确认删除": "Delete", "重命名远程资源": "Rename remote asset", "移动远程资源": "Move remote asset", "删除远程资源？": "Delete remote asset?", "移到回收站？": "Move to Trash?"
};

const replacements: Array<[RegExp, string]> = [
  [/正在加载/g, "Loading"], [/正在处理项目/g, "Processing project"], [/没有匹配的资源/g, "No matching assets"], [/当前目录为空/g, "This directory is empty"],
  [/请先填写/g, "Please enter "], [/正在读取/g, "Loading "], [/正在上传/g, "Uploading"], [/上传失败/g, "Upload failed"], [/操作没有完成/g, "The operation did not complete"],
  [/张图片/g, " images"], [/项$/g, " items"], [/第 (\d+) 页/g, "Page $1"], [/打开 (.+) 菜单/g, "Open $1 menu"], [/查看 (.+)/g, "View $1"], [/移除 (.+)/g, "Remove $1"],
  [/未保存/g, "unsaved"], [/失败/g, " failed"], [/已保存/g, "saved"], [/已删除/g, "deleted"], [/已上传/g, "uploaded"], [/已完成/g, "completed"]
];

export function translateLegacyUiText(value: string): string {
  if (getResolvedLanguage() !== "en-US" || !/[\u3400-\u9fff]/.test(value)) return value;
  const leading = value.match(/^\s*/)?.[0] ?? "";
  const trailing = value.match(/\s*$/)?.[0] ?? "";
  const core = value.trim();
  let translated = exact[core] ?? core;
  for (const [pattern, replacement] of replacements) translated = translated.replace(pattern, replacement);
  return `${leading}${translated}${trailing}`;
}

export function translateLegacyDom(root: ParentNode = document): void {
  if (getResolvedLanguage() !== "en-US") return;
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
  const texts: Text[] = [];
  while (walker.nextNode()) texts.push(walker.currentNode as Text);
  for (const node of texts) {
    if (node.parentElement?.closest("textarea, [contenteditable='true'], .cm-editor, code, pre")) continue;
    const translated = translateLegacyUiText(node.data);
    if (translated !== node.data) node.data = translated;
  }
  const elements = root instanceof Element ? [root, ...root.querySelectorAll<HTMLElement>("*")] : [...root.querySelectorAll<HTMLElement>("*")];
  for (const element of elements) for (const name of ATTRIBUTE_NAMES) {
    const value = element.getAttribute(name);
    if (value) element.setAttribute(name, translateLegacyUiText(value));
  }
}

export function startLegacyDomTranslation(): () => void {
  stopLegacyDomTranslation();
  translateLegacyDom();
  observer = new MutationObserver((mutations) => {
    observer?.disconnect();
    for (const mutation of mutations) {
      if (mutation.type === "characterData" && mutation.target.parentNode) translateLegacyDom(mutation.target.parentNode);
      for (const node of mutation.addedNodes) if (node instanceof Element) translateLegacyDom(node);
      if (mutation.type === "attributes" && mutation.target instanceof Element) translateLegacyDom(mutation.target);
    }
    observer?.observe(document.body, { childList: true, subtree: true, characterData: true, attributes: true, attributeFilter: [...ATTRIBUTE_NAMES] });
  });
  observer.observe(document.body, { childList: true, subtree: true, characterData: true, attributes: true, attributeFilter: [...ATTRIBUTE_NAMES] });
  return stopLegacyDomTranslation;
}

export function stopLegacyDomTranslation(): void {
  observer?.disconnect();
  observer = undefined;
}
