# ClearCodex

Windows-first transparent floating manager for Codex CLI.

面向 Windows 的透明悬浮式 Codex CLI 管理器。

## Language / 语言

- [English](#english)
- [中文](#chinese)

<a id="english"></a>
## English

### What It Is

ClearCodex is a desktop shell built with `Tauri + React + xterm.js` that runs `codex` inside a transparent, always-on-top overlay window.

It is designed for this workflow:

- keep Codex visible without fully blocking the desktop
- switch between multiple Codex tabs
- toggle click-through when you only want to watch output
- hide/show the overlay quickly with global hotkeys

### Current Features

- transparent frameless top-layer window
- always-on-top overlay
- click-through toggle
- top bar / right rail docking modes
- multi-tab Codex sessions
- tray icon with hide/show behavior
- global hotkeys
- local metadata persistence
- Windows ConPTY-backed terminal sessions via `portable-pty`

### Current Limitation

Session metadata is restored across app restarts, but live ConPTY attachment is only preserved while the app process remains alive. If the whole app process exits, ClearCodex restores the tab list and session state, not the previous live terminal buffer.

### Requirements

- Windows 10 or Windows 11
- `Node.js` 24+
- `pnpm`
- Rust toolchain
- `codex` available in `PATH`

Check that Codex CLI is reachable:

```powershell
where codex
```

### Install Dependencies

```powershell
pnpm install
```

### Run In Development

```powershell
pnpm tauri:dev
```

This starts the Vite frontend and the Tauri desktop app together.

### Build

```powershell
pnpm tauri:build
```

Debug build used during local verification:

```powershell
pnpm tauri build --debug
```

Generated installers are typically placed under:

```text
src-tauri/target/release/bundle/
src-tauri/target/debug/bundle/
```

### How To Use

1. Start the app.
2. Click `New tab` to spawn a new Codex session.
3. Type directly into the terminal area.
4. Use `Enable pass-through` when you want the overlay to stop intercepting mouse input.
5. Use `Peek opacity` if you want the overlay to stay visible but less intrusive.
6. Use `Right rail` if the top bar layout blocks too much content.
7. Click `Hide` or use the overlay hotkey to temporarily remove the window.

### Default Hotkeys

- `Ctrl+Alt+Space`: toggle overlay visibility
- `Ctrl+Alt+N`: create a new Codex tab
- `Ctrl+Alt+T`: toggle click-through
- `Ctrl+Alt+Left`: previous tab
- `Ctrl+Alt+Right`: next tab

If a hotkey fails to register because another app already owns it, ClearCodex will show a warning notice and still remain usable from the tray and UI buttons.

### Project Structure

- `src/`: React UI
- `src/components/TerminalSurface.tsx`: xterm.js terminal host
- `src-tauri/src/lib.rs`: Tauri app entry, tray, hotkeys, window control
- `src-tauri/src/session.rs`: Codex session and ConPTY management
- `src-tauri/src/models.rs`: shared Rust-side models
- `src-tauri/src/persistence.rs`: local snapshot persistence

### Quality Checks

```powershell
pnpm lint
pnpm build
cargo check --manifest-path .\src-tauri\Cargo.toml
```

### Packaging Notes

- App name: `ClearCodex`
- Bundle identifier: `com.clearcodex.overlay`
- Default window mode: transparent top bar
- Default focus opacity target: `0.88`
- Default peek opacity target: `0.68`

<a id="chinese"></a>
## 中文

### 这是什么

ClearCodex 是一个基于 `Tauri + React + xterm.js` 的桌面壳程序，它会把 `codex` 跑在一个透明、无边框、置顶的悬浮终端里。

它解决的主要问题是：

- 想一直看到 Codex，但不想大面积遮挡桌面
- 想在多个 Codex 会话之间快速切换
- 只看输出时，希望窗口可以鼠标穿透
- 想用全局快捷键快速呼出或隐藏

### 当前功能

- 透明无边框窗口
- 始终置顶
- 鼠标点击穿透切换
- 顶部横条 / 右侧窄栏两种停靠模式
- 多标签 Codex 会话
- 托盘图标与隐藏/显示
- 全局快捷键
- 本地状态持久化
- 基于 Windows ConPTY 的终端会话

### 当前限制

应用重启后会恢复标签元数据，但只有在应用进程仍然存活时，原始 ConPTY 会话才算真正“保活”。如果整个应用进程退出，再次启动后恢复的是标签和状态，不是之前那块还在继续滚动的终端缓冲区。

### 环境要求

- Windows 10 或 Windows 11
- `Node.js` 24+
- `pnpm`
- Rust 工具链
- `codex` 已安装并且在 `PATH` 中

先确认系统能找到 Codex CLI：

```powershell
where codex
```

### 安装依赖

```powershell
pnpm install
```

### 开发模式运行

```powershell
pnpm tauri:dev
```

这个命令会同时启动 Vite 前端和 Tauri 桌面程序。

### 打包

```powershell
pnpm tauri:build
```

本地调试打包：

```powershell
pnpm tauri build --debug
```

打包产物通常在：

```text
src-tauri/target/release/bundle/
src-tauri/target/debug/bundle/
```

### 如何使用

1. 启动应用。
2. 点击 `New tab` 新建一个 Codex 会话。
3. 直接在终端区域输入即可。
4. 如果你只想看输出、不想挡住桌面操作，就点 `Enable pass-through` 开启穿透。
5. 如果你想继续看到窗口但更不碍事，就点 `Peek opacity`。
6. 如果顶部横条挡视线，就切到 `Right rail` 右侧窄栏模式。
7. 临时不用时，点 `Hide`，或者使用全局快捷键隐藏。

### 默认快捷键

- `Ctrl+Alt+Space`：显示/隐藏悬浮层
- `Ctrl+Alt+N`：新建 Codex 标签
- `Ctrl+Alt+T`：切换点击穿透
- `Ctrl+Alt+Left`：切到上一个标签
- `Ctrl+Alt+Right`：切到下一个标签

如果某个快捷键被系统里别的应用占用，ClearCodex 会弹出警告，但程序仍然可以通过托盘和界面按钮使用。

### 项目结构

- `src/`：React 前端界面
- `src/components/TerminalSurface.tsx`：xterm.js 终端承载层
- `src-tauri/src/lib.rs`：Tauri 入口、托盘、快捷键、窗口控制
- `src-tauri/src/session.rs`：Codex 会话与 ConPTY 管理
- `src-tauri/src/models.rs`：Rust 侧共享模型
- `src-tauri/src/persistence.rs`：本地状态持久化

### 质量检查

```powershell
pnpm lint
pnpm build
cargo check --manifest-path .\src-tauri\Cargo.toml
```

### 打包信息

- 应用名：`ClearCodex`
- Bundle Identifier：`com.clearcodex.overlay`
- 默认窗口形态：顶部透明横条
- 默认聚焦透明度：`0.88`
- 默认查看透明度：`0.68`
