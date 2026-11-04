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

### 打包与分发说明

#### 1. 分发策略：全面转向 Setup 安装包（弃用 Portable 绿色免安装包）
本项目对外分发**全面采用 Setup 安装包（Windows NSIS / MSI、macOS DMG）**，正式废止 Portable 绿色便携包：
- **原因**：绿色便携包（单文件或解压缩文件夹）在运行于未预装或精简版系统的设备上时，无法可靠拉起 Microsoft Edge WebView2 引导安装程序（Bootstrapper），且容易因 VC++ 运行时动态库缺失导致进程静默闪退；
- **优势**：NSIS / MSI 安装器能够以系统级权限自动校验并静默补齐 WebView2 Runtime 与运行库依赖，macOS 下通过 DMG 挂载并拖拽至 `/Applications` 能够建立标准的应用沙箱路径与权限环境。

#### 2. macOS 跨平台打包构建
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

#### 3. 代码签名与公证（Code Signing & Notarization）
- **Windows 代码签名**：
  生产发布包推荐使用有效 OV / EV 代码签名证书签署可执行文件与 NSIS 安装包，以解除 Windows Defender SmartScreen 的未知发布者拦截：
  ```cmd
  signtool sign /f "<cert_path>.pfx" /p "<password>" /tr http://timestamp.digicert.com /td sha256 /fd sha256 target\release\bundle\nsis\*.exe
  ```
- **macOS 代码签名与官方公证**：
  生产分发需配置 Apple Developer ID Application 证书并使用 `xcrun notarytool` 完成公证（Notarization）与装订（Staple）：
  ```bash
  # 签名应用程序
  codesign --deep --force --options runtime --sign "Developer ID Application: Your Name (TeamID)" "Pure Build Launcher.app"

  # 提交苹果官方公证
  xcrun notarytool submit "Pure Build Launcher.dmg" --keychain-profile "AC_NOTARY" --wait

  # 装订公证票据
  xcrun stapler staple "Pure Build Launcher.dmg"
  ```
- **开发者/测试人员临时运行方案（未签名情况）**：
  未签署商业证书的开发构建产物在他人 Mac 上下载后，会被 Gatekeeper 拦截提示「应用已损坏」或「无法验证开发者」。将应用移入 `/Applications` 后，在终端中执行以下命令清除隔离属性即可正常运行：
  ```bash
  xattr -cr "/Applications/Pure Build Launcher.app"
  ```

## 开源协议 (License)

本项目基于 [GNU General Public License v3.0 (GPL-3.0)](./LICENSE) 开源发布。
