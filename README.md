# Pure Build Launcher

基于 **Tauri 2 + Vue 3 + TypeScript + Rust (`pcl-base`)** 的跨平台 Minecraft 启动器（版本 `0.1.8`），支持 Windows、macOS（Apple Silicon `arm64` 与 Intel `x86_64`）及 Linux，同时支持 **原版 - Java** 与 **Horizon - Rust** 双游戏生态。

> 详细系统分层、模块职责与数据流请查阅 [架构文档](docs/architecture.md)。

---

## 核心特性

### 1. 原版 - Java 生态
- **版本安装与加载器**：支持 Mojang 原版安装与 Fabric、Forge、NeoForge、OptiFine 加载器配置，支持独立目录另存为、官方服务端与对应平台开服脚本一键下载，以及版本目录安全删除。
- **跨平台运行规则**：按 Mojang 官方 `rules` 与 `classifiers` 矩阵自动提取对应平台 natives 库；统一由 `platform.rs` 推导平台路径与运行参数。
- **社区资源与实例管理**：接入 Modrinth 社区资源（支持独立选择目标 Java 版本、自动识别「独立隔离目录 / 通用共享目录」并标记已安装状态），提供模组启停/删除、光影/材质/存档查看以及 `.mrpack` 整合包导入导出。
- **Java 环境管理**：自动扫描多平台本地 JDK/JRE（识别 `x64` / `arm64`），并支持按需自动下载 Adoptium Temurin 运行时。

### 2. Horizon - Rust 生态
- **构建分发与验签**：对接 `horizon-rust/horizon` Releases（含 API 限流时的 Atom / `expanded_assets` 回退），下载后强制校验 `SHA256SUMS` 与 Ed25519 发布签名。
- **Launch API 协商与资源准备**：通过 `--launch-api` 动态协商可用参数、Minecraft 版本钉子与所需 Java 版本；通过 `horizon-prepare` 自动补齐并校验准备资源，不硬编码游戏版本号。
- **独立实例与账号门禁**：支持为构建创建多个独立 Horizon 实例（`horizon/instances/<name>`）；启动严格校验微软正版账号门禁，会话信息仅在内存中组装并经 `--session-stdin` 传入进程，不落盘、不进命令行与日志。
- **官方扩展仓库**：在下载页接入 Horizon 官方索引（`horizon-mods`），支持 `.hshader` 原生光影、`.hmod` 内容模组（自动写入 `horizon-mods.txt`）与材质包的 SHA-256 校验安装及按实例检查已安装状态；需本地源码重编的 `.hnmod` 单独归类于「原生重编类」标签并予以隔离提示。

### 3. 启动器体验与安全设计
- **双主题与内置音频**：提供现代 / 经典两套启动页与下载导航布局；内置基于 Web Audio API 的本地音乐播放器（支持均衡器预设、动态压缩与交叉淡入淡出）。
- **凭据安全**：微软 OAuth 设备码与 Yggdrasil 外置登录凭据由 Rust 侧存至应用配置目录（不写入前端 `localStorage`）。
- **局域网联机**：支持局域网房间广播发现与 `PCL2-LINK-` / `PCL2-HOST-` 邀请码编解码。

---

## 项目结构

```text
Pure-Build/
├── src/                        # Vue 3 + Pinia + UnoCSS 前端界面与 IPC 服务层
│   ├── views/                  # 启动 / 下载 / 实例 / 联机 / 设置 / 其他页面
│   ├── components/             # 业务组件与基础控件
│   ├── stores/                 # Pinia 状态管理 (app / config / versions / session / music)
│   ├── services/               # Tauri invoke 封装、ts-rs 生成类型与 Web Audio 引擎
│   └── utils/                  # 前端纯业务逻辑 (coreLogic.ts)
├── src-tauri/                  # Tauri 2 桌面宿主配置与命令注册 (src/lib.rs)
├── crates/pcl-base/            # Rust 核心库（认证 / 平台层 / Java 管线 / Horizon 适配）
├── tests/                      # Vitest 前端单元测试
└── docs/                       # 架构文档 (architecture.md) 与适配/调优文档
```

---

## 开发与构建

### 环境依赖
- **Node.js** 18+
- **Rust** 1.90+（Edition 2024，`cargo` / `rustc`）

### 常用开发与验证命令

```bash
# 安装前端依赖
npm install

# 前端单元测试 (Vitest)
npm test

# 前端类型检查与生产构建 (vue-tsc + Vite)
npm run build

# 运行 Rust 核心库全部单测、启动快照测试与 TS 类型契约校验
cargo test -p pcl-base

# 检查 Rust 全工作空间编译
cargo check --workspace

# 启动 Tauri 桌面开发模式
npm run tauri dev
```

---

## 打包与分发说明

### 1. 分发策略：全面采用安装程序（Setup）
本项目对外分发采用 **Setup 安装包（Windows NSIS / MSI、macOS DMG）**，不采用 Portable 绿色免安装包：
- **原因**：绿色便携包在未预装或精简系统上难以稳定拉起 Microsoft Edge WebView2 Bootstrapper，且易受系统缺少运行库影响。
- **优势**：NSIS / MSI 安装器可自动校验并补齐 WebView2 Runtime 与依赖；macOS 通过 DMG 拖拽至 `/Applications` 可建立标准的应用目录与权限环境。

### 2. macOS 跨平台打包构建
> **注意**：macOS 应用包（`.app` / `.dmg`）的构建机**必须是 macOS**。

```bash
# Apple Silicon (M 系列芯片) 目标构建
rustup target add aarch64-apple-darwin
npm run tauri build -- --target aarch64-apple-darwin

# Intel Mac 目标构建
rustup target add x86_64-apple-darwin
npm run tauri build -- --target x86_64-apple-darwin

# Universal 双架构通用包构建
npm run tauri build -- --target universal-apple-darwin
```

### 3. 代码签名与公证（Code Signing & Notarization）
- **Windows 代码签名**：
  生产发布包建议使用有效 OV / EV 代码签名证书签署可执行文件与 NSIS 安装包，以解除 Windows Defender SmartScreen 的未知发布者拦截：
  ```cmd
  signtool sign /f "<cert_path>.pfx" /p "<password>" /tr http://timestamp.digicert.com /td sha256 /fd sha256 target\release\bundle\nsis\*.exe
  ```
- **macOS 代码签名与官方公证**：
  生产分发需配置 Apple Developer ID Application 证书并使用 `xcrun notarytool` 完成公证（Notarization）与装订（Staple）：
  ```bash
  codesign --deep --force --options runtime --sign "Developer ID Application: Your Name (TeamID)" "Pure Build Launcher.app"
  xcrun notarytool submit "Pure Build Launcher.dmg" --keychain-profile "AC_NOTARY" --wait
  xcrun stapler staple "Pure Build Launcher.dmg"
  ```
- **未签名构建在他人 Mac 上的临时运行方式**：
  未签名构建在他人 Mac 上会被 Gatekeeper 拦截提示「应用已损坏」或「无法验证开发者」。将应用移入 `/Applications` 后，在终端执行以下命令清除隔离属性即可运行：
  ```bash
  xattr -cr "/Applications/Pure Build Launcher.app"
  # 或在当前解压目录下运行：
  xattr -cr "Pure Build Launcher.app"
  ```

---

## 开源协议 (License)

本项目基于 [GNU General Public License v3.0 (GPL-3.0-or-later)](./LICENSE) 开源发布。
