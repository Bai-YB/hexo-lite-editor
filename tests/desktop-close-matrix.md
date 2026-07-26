# 桌面关闭回归矩阵

关闭链路由两层自动化共同覆盖：

| 场景 | 自动化证据 | 预期 |
| --- | --- | --- |
| 原生窗口、无未保存内容 | `pnpm test:native-close` 向真实 Tauri 主窗口发送 `WM_CLOSE` | 执行退出清理并在 10 秒内以退出码 0 关闭 |
| 编辑器有未保存内容 | `tests/e2e/desktop.spec.ts` | 阻止关闭并显示“保存并退出 / 不保存退出 / 取消” |
| 等待自动保存期间关闭 | `tests/e2e/desktop.spec.ts` | 弹窗保持稳定，自动保存计时器不会越过关闭确认 |
| 用户取消关闭 | `tests/e2e/desktop.spec.ts` | 弹窗消失，窗口和未保存内容继续保留 |

发布前在 Windows Release 构建完成后运行：

```powershell
pnpm test:e2e
pnpm test:native-close
```

原生脚本使用独立临时 `APPDATA`，不会读取或覆盖用户的真实配置；旧标识符和新标识符目录都不会被测试污染。
