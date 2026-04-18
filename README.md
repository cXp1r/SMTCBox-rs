# SMTCBox

一个基于 Tauri 2 + TypeScript + Rust 的 Windows 桌面工具，用来读取并展示系统当前的 SMTC（System Media Transport Controls）媒体会话信息。

它会轮询系统媒体会话列表，并在界面中展示：

- 当前活跃播放器
- 媒体元数据，如标题、艺术家、专辑、曲目序号、流派
- 播放状态，如 Playing / Paused / Stopped
- 可用控制能力，如上一首、播放、暂停、下一首、快进、快退
- 时间轴信息，如当前进度、总时长、可跳转范围

## 项目背景

Windows 上很多播放器都会接入 SMTC，例如音乐播放器、浏览器中的流媒体页面或视频应用。这个项目通过 Rust 调用 Windows 媒体控制相关 API，把这些会话读取出来，再用前端界面集中展示，方便调试、观察和后续扩展。

## 技术栈

- Tauri 2
- Vanilla TypeScript
- Vite
- Rust
- Windows Media Control API

## 运行要求

本项目当前主要面向 Windows。

- 操作系统：Windows
- Node.js：用于前端依赖安装与开发
- Rust：用于编译 Tauri 后端
- Tauri 开发环境：需提前安装系统依赖

说明：

- 在非 Windows 平台上，后端当前会返回空会话列表。
- 如果你是第一次配置 Tauri 环境，建议先完成 Tauri 官方要求的系统依赖安装。

## 开发启动

安装依赖：

```bash
npm install
```

启动开发模式：

```bash
npm run tauri dev
```

这个命令会同时启动：

- Vite 前端开发服务器
- Tauri 桌面应用

## 构建

构建前端资源：

```bash
npm run build
```

构建桌面应用安装包或可执行产物：

```bash
npm run tauri build
```

## 项目结构

```text
.
├─ src/                 # 前端界面与轮询逻辑
├─ src-tauri/           # Rust 后端与 Tauri 配置
│  ├─ src/
│  │  ├─ main.rs        # 桌面程序入口
│  │  └─ lib.rs         # SMTC 会话读取与 Tauri command
│  └─ tauri.conf.json   # Tauri 窗口与打包配置
├─ package.json         # 前端脚本
└─ README.md
```

## 核心实现

前端通过 Tauri `invoke` 调用 Rust 暴露的命令：

- `get_smtc_sessions`

Rust 端使用 Windows 的 `GlobalSystemMediaTransportControlsSessionManager` 读取系统会话，并将数据整理为以下几类结构返回给前端：

- `SmtcMediaProperties`
- `SmtcPlaybackInfo`
- `SmtcTimelineProperties`
- `SmtcSession`

前端默认每秒轮询一次，用于持续刷新当前媒体状态。

## 当前特性

- 展示所有可读取到的 SMTC 会话
- 标记当前活跃会话
- 展示播放状态与控制能力
- 展示时间轴与进度条
- 空状态提示与手动刷新按钮

## 已知限制

- 当前以“监视和展示”为主，还没有直接控制播放器的逻辑
- 数据获取目前采用轮询，不是事件驱动更新
- 项目目前主要适配 Windows

## 后续可扩展方向

- 增加播放控制能力，如播放、暂停、切歌
- 增加封面读取与展示
- 增加按应用过滤、搜索、排序
- 改成事件驱动刷新，降低轮询开销
- 导出会话信息用于调试或日志分析

## 许可证

仓库中暂未声明许可证。如需开源分发，建议补充 `LICENSE` 文件。
