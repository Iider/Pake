# Kimi Code 桌面客户端

Pake 可通过编译开关打包为 Kimi Code 本地 Web 服务的桌面客户端。品牌配置和图标保存在 `kimi/`，运行时代码位于 `src-tauri/src/app/kimi_web.rs`。

## 运行行为

- `PAKE_KIMI_WEB=1` 在编译时启用 Kimi 客户端逻辑，普通 Pake 构建不启用。
- 启动后检查 `127.0.0.1:58627`；端口未监听时运行 `kimi web --no-open --port 58627`。
- 服务就绪后读取 `~/.kimi-code/server.token`，将主窗口导航到带认证片段的本地 Web UI。
- 应用只回收自己创建的服务进程，不终止启动前已经存在的 Kimi 服务。
- Windows 使用 Job Object 保证应用崩溃或被强制终止时一并回收子进程。

## 窗口与主题

- 会话右键菜单提供“在新窗口中打开”。深链保留主窗口的 `#token` 认证片段，且只允许 Kimi 本地服务同源 URL。
- Windows 在线程中创建 WebView2 子窗口，避免在 IPC 回调中同步建窗卡住；页面加载完成后显示窗口，3 秒后仍未显示则执行兜底显示。
- macOS 隐藏标题栏时，侧栏头部会为系统交通灯预留拖动区域，并隐藏与原生窗口重复的页内品牌。该适配不会启用 Kimi 依赖 `window.kimiDesktop` 的完整桌面模式。
- 每个窗口独立同步原生顶栏主题，不会在多窗口亮暗状态不一致时互相覆盖。
- 顶栏优先使用页面的 `--color-sidebar-bg`：月之亮面为 `#f9fbfc`，月之暗面为 `#0d0d0d`；其他网页回退到 `meta[name="theme-color"]`。

## 构建

Windows 构建步骤见 `kimi/README.md`。该流程临时覆盖 Pake 的 tracked 配置和图标，构建结束后必须恢复这些文件，只提交 `kimi/` overlay 与源码改动。

macOS 产物必须在 macOS 上构建。Mac mini M4 使用 `aarch64-apple-darwin` 目标；需要复制或克隆完整仓库，不能只迁移 `kimi/`。

## 验证重点

- 启动时自动连接或拉起 Kimi Web 服务。
- 右键会话后能打开第二个独立窗口，直接加载目标会话，不显示认证页。
- 多窗口分别切换亮暗主题时，顶栏不闪烁且与各自侧栏同色。
- Windows 强制结束客户端后没有残留的 `kimi web` 子进程。
- macOS 需要单独验证服务进程回收、独立窗口、顶栏主题，以及交通灯不遮挡侧栏头部。
