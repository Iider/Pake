# Kimi Code 客户端构建 overlay

本目录保存 Kimi 品牌构建所需的配置与图标快照。Pake 的构建配置（`src-tauri/pake.json`、`tauri.conf.json`、`tauri.windows.conf.json`、`src-tauri/png/`、`src-tauri/icons/`）是 tracked 文件，正式提交前必须保持上游原状，因此 Kimi 构建态只存在于本目录，构建时临时覆盖、构建后恢复。

## 构建 Kimi.exe

```powershell
# 1. 覆盖配置与图标
Copy-Item kimi\pake.json src-tauri\pake.json -Force
Copy-Item kimi\tauri.conf.json src-tauri\tauri.conf.json -Force
Copy-Item kimi\tauri.windows.conf.json src-tauri\tauri.windows.conf.json -Force
Copy-Item kimi\icon_512.png src-tauri\png\icon_512.png -Force
Copy-Item kimi\icon_256.png src-tauri\png\icon_256.png -Force
Copy-Item kimi\icon_32.ico src-tauri\png\icon_32.ico -Force
Copy-Item kimi\icon_256.ico src-tauri\png\icon_256.ico -Force
Copy-Item kimi\icon.png src-tauri\icons\icon.png -Force
Copy-Item kimi\icon.ico src-tauri\icons\icon.ico -Force

# 2. 构建（Rust 默认 pinned 1.95.0 未安装，必须指定 stable）
Set-Location src-tauri
$env:RUSTUP_TOOLCHAIN = "stable"
$env:CARGO_PROFILE_RELEASE_LTO = "off"
$env:PAKE_KIMI_WEB = "1"
cargo build --release

# 3. 产出
Set-Location ..
Copy-Item src-tauri\target\release\pake.exe Kimi.exe -Force

# 4. 恢复构建 churn（tracked 文件保持上游原状）
git restore src-tauri/pake.json src-tauri/tauri.conf.json src-tauri/tauri.windows.conf.json src-tauri/png src-tauri/icons
```

## 文件说明

| 文件                       | 作用                                                                                                      |
| -------------------------- | --------------------------------------------------------------------------------------------------------- |
| `pake.json`                | 窗口配置：`http://127.0.0.1:58627`、1400x900、隐藏标题栏、`CmdOrCtrl+Shift+K` 唤起快捷键、`hide_on_close` |
| `tauri.conf.json`          | 产品名 Kimi、identifier `com.pake.kimi`、托盘图标 `png/icon_512.png`                                      |
| `tauri.windows.conf.json`  | Windows bundle 图标与 resources                                                                           |
| `kimi.png`                 | 原始品牌图（`C:\Users\11696\Pictures\kimi.png` 的副本）                                                   |
| `icon_*.png/ico`、`icon.*` | 由 kimi.png 生成的各尺寸图标                                                                              |

行为细节（`kimi web` 子进程拉起/Job Object 回收、顶栏随页面主题变色）见根目录 `KIMI_CLIENT_HANDOFF.md`。
