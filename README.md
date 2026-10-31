# Pure Build Launcher (Vue 3 + Tauri 2 + Rust)

跨平台 Minecraft 启动器（Tauri 2 + Vue 3 / TypeScript + Rust `pcl-base`），支持 Windows、macOS（Apple Silicon `arm64` 与 Intel `x86_64`）及 Linux。

### macOS 首次运行解除 Gatekeeper 隔离
由于开源 GitHub Actions 产物未包含付费的 Apple Developer 证书，在他人 Mac 上下载安装后，首次打开若遇系统安全提示，运行以下命令即可正常启动：
```bash
xattr -cr "/Applications/Pure Build Launcher.app"
# 或在当前解压目录下运行：
xattr -cr "Pure Build Launcher.app"
```

## 开发与构建

### 环境依赖
- Node.js 18+
- Rust stable 工具链 (`cargo`, `rustc`)

### 本地开发与测试
```bash
# 安装前端依赖
npm install

# 前端类型检查与构建
npm run build

# 运行后端跨平台规则与平台层单元测试
cargo test -p pcl-base

# 启动 Tauri 开发模式
npm run tauri dev
```

### macOS 打包说明（遵循项目规范 §3.6）
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

- **关于代码签名与公证（Gatekeeper）**：未配置 Apple Developer ID 代码签名与公证（Notarization）的构建产物在他人 Mac 上首次打开时会被 macOS Gatekeeper 拦截，需通过 `xattr -cr "/Applications/Pure Build Launcher.app"` 移除隔离属性或完成正式公证后分发。
