import type { TranslationTree } from "../types";

export const zhCN: TranslationTree = {
  common: { confirm: "确认", cancel: "取消", save: "保存", delete: "删除", retry: "重试", close: "关闭" },
  navigation: { label: "主导航", editor: "编辑器", imageBed: "图床", settings: "设置", about: "关于" },
  window: { minimize: "最小化", maximize: "最大化", restore: "还原", close: "关闭", unsaved: "有未保存更改" },
  loading: { workspace: "正在初始化桌面工作区", page: "正在加载页面" },
  settings: {
    languageTitle: "界面语言",
    languageDescription: "跟随系统，或为应用固定选择一种语言。",
    languageSystem: "跟随系统",
    languageChinese: "简体中文",
    languageEnglish: "English"
  },
  update: { availableNotice: "发现新版本 {version}，可在“关于”中查看。" },
  pages: {
    aboutTitle: "关于", aboutDescription: "一个安静、可靠的 Hexo 桌面写作工具。",
    aboutProductDescription: "专注于 Markdown 写作、图片管理与可靠发布，不内置博客环境，也不接管你的文章文件。",
    projectHomepage: "项目主页", projectHomepageDescription: "源代码、问题反馈与版本记录", licenseDescription: "查看开源许可证",
    version: "版本", update: "更新", notChecked: "尚未检查", checkUpdate: "检查更新", checking: "检查中",
    downloadUpdate: "下载更新", restartInstall: "重启并安装"
  },
  errors: {
    project_not_found: "找不到项目，请重新选择 Hexo 项目。",
    image_list_failed: "无法读取图片列表，请重试。",
    update_check_failed: "检查更新失败，请检查网络后重试。",
    plugin_permission_denied: "插件没有执行此操作所需的权限。"
  }
};
