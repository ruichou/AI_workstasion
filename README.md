# GlassWorkspace — 开发进度提示

## 当前进度（2026-08-05）

已完成 1600×900 双行面板大改版，对照效果图：

- 时间卡：大时钟 + 秒数上标 + 公历/农历日期 + 天气（温度/多云/湿度/体感/风速）
- 系统状态卡：SVG 环形进度（CPU+温度 / 内存 / 硬盘 C:）+ "● 运行良好" 徽章
- 日历卡：前后月淡显、今天蓝色高亮、月份导航
- 待办卡：localStorage 持久化、勾选/删除/添加、进度条
- 快捷启动卡：应用程序/文件夹标签页 + 2×6 图标网格（11 个预置应用 + 添加）
- 实用工具卡：8 个系统工具（文件管理器/记事本/CMD/截图/计算器/回收站/控制面板/任务管理器）
- 系统设置：透明度 / 音量 / 护眼模式（gamma ramp 实现）/ 护眼强度
- 缩服模式：折叠成迷你时钟条（360×110）
- 页脚：● 已连接 | 版本: v1.0.0

## 待办（回家继续）

- [x] **重新构建验证**：`npm install && npm run tauri build` 已完成
     构建产物：`src-tauri\target\release\bundle\nsis\GlassWorkspace_0.1.0_x64-setup.exe`
     以及 `src-tauri\target\release\bundle\msi\GlassWorkspace_0.1.0_x64_en-US.msi`
     冒烟测试：exe 启动正常无崩溃
- [x] **快捷启动名称截断问题**：已确认 `.app-name` 的省略号样式已移除
- [x] **launch_app 参数解析**：支持双引号包裹的带空格路径（`split_args`）
- [x] **磁盘统计口径**：只统计 C: 盘，与 UI「硬盘 C:」标签一致

1. **配置真实应用路径**：
   - 默认配置了 11 个应用占位（name/emoji），路径为空
   - 运行应用 → 点顶部 ⚙ → 编辑 `%APPDATA%\com.glassworkspace.app\config.json`
   - 格式：`{ "name": "ChatGPT", "path": "C:\\...\\ChatGPT.exe", "emoji": "🤖", "args": null }`
   - 改完重启应用生效

4. **可选优化**：
   - 效果图里的真实应用图标（当前用 emoji 近似）
   - 护眼模式下 todo 第一条时间红色等细节微调
   - 文件夹标签页只有 6 个文件夹，可补充

## 环境备忘

- 开发目录：`C:\Users\34632\Documents\Default Project\floating-workspace`（与 D 盘仓库同步）
- 构建命令（PowerShell，需先加 PATH）：
  ```powershell
  $env:PATH = "$env:LOCALAPPDATA\node-dist\node-v22.22.2-win-x64;" + $env:USERPROFILE + "\.cargo\bin;" + $env:PATH
  ```
- 运行中的应用是 exe 直跑（`src-tauri\target\release\floating-workspace.exe`），改代码后需 Stop-Process 再重新构建
- 截图验证流程：截窗口 → 用 mimo 描述（`opencode run -m opencode-go/mimo-v2.5 "描述指令" -f "图片路径"`，指令在前、用英文）
