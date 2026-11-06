#!/usr/bin/env python3
"""Pure Build Launcher 官网生成器：中英双语六页静态站，输出到自身所在目录（仓库 site/）。
Cloudflare Pages 构建时运行 python3 site/build.py 生成六页 HTML（生成物不入库）；本地预览也在本目录运行它。"""
import os

ROOT = os.path.dirname(os.path.abspath(__file__))
SITE = ROOT
VER = "v0.1.7"

NAV = {
    "zh": [("首页", "index.html"), ("下载", "download.html"), ("更新日志", "changelog.html")],
    "en": [("Home", "index.html"), ("Download", "download.html"), ("Changelog", "changelog.html")],
}

def page(lang, slug, title, desc, body):
    prefix = "" if lang == "zh" else "../"
    self_prefix = "" if lang == "zh" else ""
    nav_items = ""
    for label, href in NAV[lang]:
        active = ' class="active"' if href == slug else ""
        nav_items += f'<a href="{self_prefix}{href}"{active}>{label}</a>'
    other = "en/" if lang == "zh" else "../"
    other_label = "EN" if lang == "zh" else "中文"
    footer_note = ("Pure Build Launcher 是独立的第三方启动器，与任何游戏开发商无关。"
                   if lang == "zh" else
                   "Pure Build Launcher is an independent third-party launcher, not affiliated with any game developer.")
    foot_links = ("下载", "更新日志") if lang == "zh" else ("Download", "Changelog")
    rights = "© 2026 Pure Build Launcher" if lang == "zh" else "© 2026 Pure Build Launcher"
    return f"""<!DOCTYPE html>
<html lang="{'zh-CN' if lang == 'zh' else 'en'}">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>{title}</title>
<meta name="description" content="{desc}">
<link rel="icon" href="{prefix}assets/app-icon.png">
<link rel="stylesheet" href="{prefix}assets/style.css">
</head>
<body>
<header><div class="wrap nav">
<a class="brand" href="{self_prefix}index.html"><img src="{prefix}assets/app-icon.png" alt="PBL logo">Pure Build Launcher</a>
<nav class="links">{nav_items}<a class="lang" href="{other}{slug}">{other_label}</a></nav>
</div></header>
{body}
<footer><div class="wrap">
<span>{rights}</span>
<div class="right"><a href="{self_prefix}download.html">{foot_links[0]}</a><a href="{self_prefix}changelog.html">{foot_links[1]}</a></div>
</div><div class="wrap" style="margin-top:10px"><small>{footer_note}</small></div></footer>
<script src="{prefix}assets/release.js" defer></script>
</body></html>"""

FEATURES = {
"zh": [
 ("🧩", "五大加载器，一键安装", "原版、Fabric、Forge、NeoForge、Quilt 与 OptiFine 全部真实安装：自动探测、拉取依赖、合并版本元数据，不用再手动拼文件。"),
 ("🔑", "三种登录方式", "微软正版（OAuth 设备码授权）、外置 Yggdrasil 登录与离线模式并存，账号凭据本地保存，随开随用。"),
 ("☕", "Java 自动配齐", "按每个版本声明的 Java 大版本（8 / 17 / 21）自动校验与改选；本机没有时，从 Adoptium 官方源下载 Temurin 运行时并安全解压。"),
 ("⬇️", "高速下载与自愈", "32 路并发下载，SHA1 / SHA256 / SHA512 权威哈希校验，损坏文件自动识别重下；内置 BMCLAPI 等镜像源，断网恢复自动续传。"),
 ("📦", "整合包与 Modrinth", "Modrinth 整合包（.mrpack）一键导入、当前实例一键导出；站内浏览并安装 Mod、资源包与光影。"),
 ("🎮", "联机与崩溃诊断", "局域网房间自动发现与邀请码联机；游戏崩溃后自动分析日志，给出内存溢出、Mod 冲突等针对性修复建议。还有内置音乐播放器陪你等加载。"),
],
"en": [
 ("🧩", "Five loaders, one click", "Vanilla, Fabric, Forge, NeoForge, Quilt and OptiFine — all really installed: automatic detection, dependency resolution and version-metadata merging. No more hand-assembling files."),
 ("🔑", "Three ways to sign in", "Microsoft accounts (OAuth device-code flow), third-party Yggdrasil servers and offline mode. Credentials stay on your machine."),
 ("☕", "Java, handled for you", "Each game version's required Java (8 / 17 / 21) is checked and selected automatically; if it's missing, the Temurin runtime is downloaded from Adoptium and extracted safely."),
 ("⬇️", "Fast downloads that heal themselves", "32-way concurrent downloads verified against authoritative SHA1 / SHA256 / SHA512 hashes, corrupt files re-fetched automatically, mirror sources built in, interrupted downloads resumed."),
 ("📦", "Modpacks & Modrinth", "Import Modrinth modpacks (.mrpack) in one click, export your instance just as easily, and browse and install mods, resource packs and shaders in-app."),
 ("🎮", "Multiplayer & crash diagnosis", "LAN room discovery and invite-code multiplayer; when the game crashes, logs are analyzed automatically with targeted fixes for OOM, mod conflicts and more. A built-in music player keeps you company while you wait."),
]}

def home(lang):
    zh = lang == "zh"
    feats = "".join(f'<div class="card"><h3><span class="ic">{i}</span>{t}</h3><p>{d}</p></div>' for i, t, d in FEATURES[lang])
    if zh:
        tagline = "快、干净、跨平台的沙盒游戏启动器"
        sub = "为 Windows、macOS 与 Linux 打造的一致启动体验：装版本、配 Java、下整合包、联机，全都收进一个清爽的界面里。"
        dl, more = "下载 <span data-rel=\"version\">v0.1.7</span>", "查看更新日志"
        ver = "最新版本 <span data-rel=\"version\">v0.1.7</span> · <span data-rel=\"date\">2026-11-06</span> · Windows / macOS / Linux"
        stats = [("3", "操作系统平台"), ("6", "种官方安装包"), ("0.7s", "热启动耗时（实测）"), ("54", "自动化测试守护")]
        f_title, f_lede = "该有的，一样不少", "从装第一个版本到和朋友联机，每一步都替你想好了。"
        s_title, s_lede = "界面预览", ""
        shots = [("shot-classic-launch.jpg", "经典布局 · 启动"), ("shot-classic-download.jpg", "经典布局 · 版本下载")]
    else:
        tagline = "A fast, clean, cross-platform sandbox game launcher"
        sub = "One consistent launching experience on Windows, macOS and Linux: installing versions, sorting out Java, fetching modpacks and playing together — all in one tidy interface."
        dl, more = "Download <span data-rel=\"version\">v0.1.7</span>", "See the changelog"
        ver = "Latest release <span data-rel=\"version\">v0.1.7</span> · <span data-rel=\"date\">Nov 6, 2026</span> · Windows / macOS / Linux"
        stats = [("3", "operating systems"), ("6", "official installers"), ("0.7s", "warm start (measured)"), ("54", "automated tests")]
        f_title, f_lede = "Everything you need, nothing you don't", "From your first install to a multiplayer night with friends, every step is taken care of."
        s_title, s_lede = "A look inside", ""
        shots = [("shot-classic-launch.jpg", "Classic layout · Launcher"), ("shot-classic-download.jpg", "Classic layout · Downloads")]
    stat_html = "".join(f'<div class="stat"><b>{n}</b><span>{t}</span></div>' for n, t in stats)
    shot_html = "".join(f'<figure class="shot"><img src="{'' if zh else '../'}assets/shots/{f}" alt="{c}" loading="lazy"><figcaption>{c}</figcaption></figure>' for f, c in shots)
    body = f"""
<div class="hero"><div class="wrap">
<img class="logo" src="{'' if zh else '../'}assets/app-icon.png" alt="Pure Build Launcher">
<h1>Pure Build Launcher</h1>
<p class="tagline">{tagline}</p>
<p class="sub">{sub}</p>
<div class="cta"><a class="btn" href="download.html">{dl}</a><a class="btn ghost" href="changelog.html">{more}</a></div>
<p class="ver">{ver}</p>
<div class="stats">{stat_html}</div>
</div></div>
<section><div class="wrap"><h2>{f_title}</h2><p class="lede">{f_lede}</p><div class="grid">{feats}</div></div></section>
<section class="soft"><div class="wrap"><h2>{s_title}</h2><p class="lede">{s_lede}</p><div class="shots">{shot_html}</div></div></section>"""
    title = "Pure Build Launcher — 跨平台沙盒游戏启动器" if zh else "Pure Build Launcher — Cross-platform Sandbox Game Launcher"
    desc = ("Pure Build Launcher：快、干净、跨平台的沙盒游戏启动器，支持 Windows、macOS 与 Linux，五大加载器与 Java 自动配齐。"
            if zh else "Pure Build Launcher: a fast, clean, cross-platform sandbox game launcher for Windows, macOS and Linux, with five loaders and automatic Java management.")
    return page(lang, "index.html", title, desc, body)

DL_GROUPS = {
"zh": [
 ("Windows", "推荐使用安装程序（.exe），会自动补齐 WebView2 与运行库；.msi 适合批量部署。",
  [("/dl/windows-setup", "Pure.Build.Launcher_0.1.7_x64-setup.exe", "安装程序 · x64 · 5.3 MB", True),
   ("/dl/windows-msi", "Pure.Build.Launcher_0.1.7_x64_en-US.msi", "MSI 安装包 · x64 · 7.2 MB", False)]),
 ("macOS", "通用包（Universal），Apple Silicon 与 Intel 均可使用；首次打开请看下方注意事项。",
  [("/dl/macos-dmg", "Pure.Build.Launcher_0.1.7_universal.dmg", "DMG 磁盘映像 · Universal · 17.2 MB", True)]),
 ("Linux", "Debian / Ubuntu 用 .deb，Fedora / RHEL 用 .rpm，不想安装可用 AppImage 直接运行。",
  [("/dl/linux-deb", "Pure.Build.Launcher_0.1.7_amd64.deb", "DEB 包 · amd64 · 7.6 MB", False),
   ("/dl/linux-rpm", "Pure.Build.Launcher-0.1.7-1.x86_64.rpm", "RPM 包 · x86_64 · 7.6 MB", False),
   ("/dl/linux-appimage", "Pure.Build.Launcher_0.1.7_amd64.AppImage", "AppImage · amd64 · 82.0 MB", False)]),
],
"en": [
 ("Windows", "The setup program (.exe) is recommended — it installs WebView2 and the runtime libraries for you. The .msi suits managed deployments.",
  [("/dl/windows-setup", "Pure.Build.Launcher_0.1.7_x64-setup.exe", "Setup · x64 · 5.3 MB", True),
   ("/dl/windows-msi", "Pure.Build.Launcher_0.1.7_x64_en-US.msi", "MSI package · x64 · 7.2 MB", False)]),
 ("macOS", "A universal build for both Apple Silicon and Intel Macs. Please read the first-launch note below.",
  [("/dl/macos-dmg", "Pure.Build.Launcher_0.1.7_universal.dmg", "DMG image · Universal · 17.2 MB", True)]),
 ("Linux", ".deb for Debian / Ubuntu, .rpm for Fedora / RHEL, or AppImage to run without installing.",
  [("/dl/linux-deb", "Pure.Build.Launcher_0.1.7_amd64.deb", "DEB package · amd64 · 7.6 MB", False),
   ("/dl/linux-rpm", "Pure.Build.Launcher-0.1.7-1.x86_64.rpm", "RPM package · x86_64 · 7.6 MB", False),
   ("/dl/linux-appimage", "Pure.Build.Launcher_0.1.7_amd64.AppImage", "AppImage · amd64 · 82.0 MB", False)]),
]}

def download(lang):
    zh = lang == "zh"
    groups = ""
    for name, hint, files in DL_GROUPS[lang]:
        items = ""
        for slug, fname, meta, rec in files:
            badge = " ⭐" if rec and zh else (" ⭐ Recommended" if rec else "")
            btn = "下载" if zh else "Download"
            fdesc, fsize = meta.rsplit(" · ", 1)
            items += (f'<div class="dl-item" data-asset="{slug[4:]}"><div><div class="fname"><span data-fname>{fname}</span>{badge}</div>'
                      f'<div class="fmeta"><span>{fdesc}</span> · <span data-fsize>{fsize}</span></div></div>'
                      f'<a class="btn" href="{slug}">{btn}</a></div>')
        groups += f'<div class="dl-group"><h3>{name}</h3><p class="hint">{hint}</p>{items}</div>'
    if zh:
        head_t, head_p = "下载 Pure Build Launcher", "最新版本 <span data-rel=\"version\">v0.1.7</span> · 发布于 <span data-rel=\"date\">2026-11-06</span>"
        note_t, note_p = "⚠️ macOS 首次打开提示「已损坏」或被拦截？", "本软件未经 Apple 付费证书签名，首次打开可能被 Gatekeeper 拦截。把应用拖入「应用程序」后，在终端执行下面一行命令即可正常打开："
        alt = "也可以在当前解压目录下对 .app 执行同一命令。历史版本说明见本站更新日志页。"
        src = ""
    else:
        head_t, head_p = "Download Pure Build Launcher", "Latest release <span data-rel=\"version\">v0.1.7</span> · <span data-rel=\"date\">Nov 6, 2026</span>"
        note_t, note_p = "⚠️ macOS says the app is damaged or blocked on first launch?", "The app is not signed with a paid Apple certificate, so Gatekeeper may block the first launch. After dragging the app into Applications, run this one line in Terminal:"
        alt = "You can also run the same command against the .app in the extraction folder. Older releases are listed on this site's changelog page."
        src = ""
    body = f"""
<div class="page-head"><div class="wrap"><h1>{head_t}</h1><p>{head_p}</p></div></div>
<section style="padding-top:30px"><div class="wrap">
{groups}
<div class="note"><h4>{note_t}</h4><p>{note_p}</p>
<pre><code>xattr -cr "/Applications/Pure Build Launcher.app"</code></pre>
<p style="margin-top:10px">{alt}</p></div>
<p style="color:var(--mut);font-size:14.5px">{src}</p>
</div></section>"""
    title = "下载 — Pure Build Launcher" if zh else "Download — Pure Build Launcher"
    desc = "下载 Pure Build Launcher v0.1.7：Windows / macOS / Linux 官方安装包。" if zh else "Download Pure Build Launcher v0.1.7 for Windows, macOS and Linux."
    return page(lang, "download.html", title, desc, body)

# 更新日志数据：(版本, 日期, [(小节, [条目...]) ...])；zh 与仓库 CHANGELOG.md 一致，en 为译文
CHANGELOG = {
"zh": [
 ("v0.1.7", "2026-11-06", [
   ("界面交互与质感升级", ["<b>全站矢量图标标准化</b>：引入轻量化纯矢量图标组件 <code>PclIcon</code>，全局替换原散落在各页面的 emoji 表情符号，统一经典与现代两套布局下的视觉设计语言。",
                          "<b>音乐播放弹窗布局优化</b>：重构迷你播放器控制行，将调音预设下移至工具栏，完整保留循环模式文字按钮的同时将音量滑块大幅拓宽，解决音量条被挤压的问题。",
                          "<b>标题栏品牌标识精炼</b>：优化经典布局顶部状态栏排版，去除多余的重复字母字标，保持界面干净清爽。",
                          "<b>官网基础设施沉淀</b>：新增基于 Cloudflare Pages 与边缘函数的轻量化官网（<code>site/</code>），支持按架构自动分发多平台安装包。"]),
 ]),
 ("v0.1.6", "2026-11-04", [
   ("架构与组件化重构", ["<b>LaunchView 架构模块化</b>：将主启动视图从 824 行巨石单文件重构为清晰内聚的子组件与 composable（<code>LaunchView.vue</code> 收敛至 68 行，下辖 <code>VersionPicker</code>、<code>JavaSelectPanel</code>、<code>LaunchClassicLayout</code>、<code>LaunchModernLayout</code> 与 <code>useLaunch</code>），各独立模块代码行数严格控制在 350 行以内。",
                      "<b>状态单一事实源与双布局解耦</b>：彻底分家 PCL 经典布局与现代网格布局，消除两套视图间的模板冗余与状态污染，确保原版 PCL 像素级一致性。",
                      "<b>启动生命周期与异常强杀治理</b>：通过 <code>useLaunch</code> composable 集中收敛后端启动进度监听、正常/异常退出处理与进程超时强制终止逻辑。"]),
 ]),
 ("v0.1.5", "2026-11-04", [
   ("界面与体验修复", ["<b>音乐播放弹窗修复</b>：修复音量滑杆在经典与现代两套布局下溢出弹窗边框的问题；移除 EQ 预设图标，预设项目与音效参数保持不变。"]),
   ("架构与内部重构", ["<b>后端模块化解耦</b>：启动参数构建、版本继承与文件哈希校验逻辑独立拆分为子模块。",
                      "<b>前端状态与组件治理</b>：Pinia 全局状态按配置、版本、音乐、会话四大领域解耦拆分；抽离独立音乐播放条与登录认证弹窗组件；引入基于 ts-rs 的前后端强类型契约，类型漂移即测试报错中断。"]),
   ("测试体系建设", ["<b>测试覆盖大幅补强</b>：后端测试由 29 个扩充至 46 个（新增真实版本 JSON 夹具的启动参数跨平台快照、认证解析与生命周期用例）；前端引入 vitest 并新增 8 个核心逻辑用例。"]),
 ]),
 ("v0.1.4", "2026-11-03", [
   ("新增与优化", ["<b>资源并发下载与完整性优化</b>：静态 CDN 资源放行免节流，恢复 32 并发拉取；流式下载边收数据边在内存累加 SHA-1 并识别 ZIP 魔数，消除海量碎片小文件的二次读盘开销。",
                   "<b>Java 版本智能校验与自动配齐</b>：读取版本 JSON（含 inheritsFrom 继承链）声明的 javaVersion（8 / 17 / 21），启动前自动比对本机清单，缺失时从 Adoptium 下载 Temurin JRE 并安全解压（路径规范化防 zip-slip）。",
                   "<b>启动性能打点与秒开优化</b>：前后端全链路打点，实测冷启动中位数约 1278ms、热启动中位数约 732ms。",
                   "<b>分发与打包架构规范</b>：弃用免安装绿色包，全面收敛为标准化安装器（NSIS / MSI / DMG），规避 WebView2 与 VC++ 运行时依赖隐患；补齐签名与公证指南。"]),
 ]),
 ("v0.1.3", "2026-11-01", [
   ("重构与平台支持", ["<b>平台层解耦与反写死治理</b>：确立 platform.rs 唯一平台适配层，全仓清除硬编码平台字面量，系统与架构判定均由运行时推导；补齐官方启动规则与分类器匹配，支持 macOS（arm64 与 x64）及 Linux。",
                      "<b>下载源架构解耦</b>：下载源与镜像常量集中至 endpoints.rs，统一 BMCLAPI 与 MCIMirror 镜像路由策略。"]),
 ]),
 ("v0.1.2", "2026-11-01", [
   ("新增功能", ["<b>整合包与生态资源管理</b>：支持 Modrinth .mrpack 整合包一键导入，及导出当前实例为标准 .mrpack 包。",
                 "<b>崩溃智能分析系统</b>：游戏异常退出时自动捕获日志上下文，识别内存溢出、Mod 兼容冲突、驱动崩溃与字节码版本不匹配并给出修复建议。",
                 "<b>资源与存档管理</b>：扫描并呈现存档、材质包、光影包与背景音乐，支持直接操作与快速定位。"]),
 ]),
 ("v0.1.1", "2026-11-01", [
   ("新增与修复", ["<b>版本安装器重构</b>：支持原版、Fabric、Forge、NeoForge 与 OptiFine 的自动探测、依赖拉取与版本元数据深度合并；并发下载与 SHA1 / SHA256 / SHA512 完整性校验，断点重试与损坏自愈。",
                  "<b>局域网联机协作</b>：基于 UDP 组播的局域网房间广播与自动发现，支持生成并解析联机邀请码。"]),
 ]),
 ("v0.1.0", "2026-11-01", [
   ("初始发布", ["<b>核心启动框架搭建</b>：Tauri 2 + Vue 3 + TypeScript 构建现代化 UI；Rust 核心支持离线登入与微软 OAuth2 设备码正版登录；本地版本扫描、版本列表展示与自定义 JVM 参数配置。"]),
 ]),
],
"en": [
 ("v0.1.7", "Nov 6, 2026", [
   ("UI & Aesthetic Polish", ["<b>Unified vector icons</b>: standardized across the whole app using the lightweight <code>PclIcon</code> component, replacing raw emoji characters for crisp rendering at any DPI.",
                              "<b>Music player popover redesign</b>: reworked player controls by moving EQ tuning presets to the bottom toolbar and expanding the volume slider width to over 100px while keeping full loop-mode controls.",
                              "<b>Streamlined title bar</b>: cleaned up redundant branding in the Classic layout header to match the clean design aesthetic.",
                              "<b>Official website infrastructure</b>: added the Cloudflare Pages & Workers static site and download routing infrastructure (<code>site/</code>)."]),
 ]),
 ("v0.1.6", "Nov 4, 2026", [
   ("Architecture & Component Refactor", ["<b>LaunchView modularization</b>: the main launch view was refactored from an 824-line monolith into cohesive subcomponents and a composable (<code>LaunchView.vue</code> down to 68 lines, with <code>VersionPicker</code>, <code>JavaSelectPanel</code>, <code>LaunchClassicLayout</code>, <code>LaunchModernLayout</code> and <code>useLaunch</code>), each module kept under 350 lines.",
                      "<b>Single source of truth, decoupled layouts</b>: the classic PCL layout and the modern grid layout were fully separated, eliminating template duplication and state cross-talk between the two views while keeping the classic layout pixel-faithful to the original.",
                      "<b>Launch lifecycle & force-kill governance</b>: launch-progress listening, normal/abnormal exit handling and process-timeout force termination were centralized in the <code>useLaunch</code> composable."]),
 ]),
 ("v0.1.5", "Nov 4, 2026", [
   ("Fixes & polish", ["<b>Music player popover fix</b>: the volume slider no longer overflows the popover frame in either the Classic or Modern layout; EQ preset icons were removed, presets and sound unchanged."]),
   ("Architecture", ["<b>Backend modularization</b>: launch-argument building, version inheritance and file-hash verification split into dedicated submodules.",
                     "<b>Frontend state & components</b>: the Pinia store split into config, versions, music and session domains; music player bar and login dialog extracted as components; a ts-rs type contract between Rust and TypeScript — drift fails the test suite."]),
   ("Testing", ["<b>Test coverage push</b>: backend tests grew from 29 to 46 (launch-argument snapshots from real version JSON fixtures, auth parsing and lifecycle cases); the frontend gained vitest with 8 core-logic cases."]),
 ]),
 ("v0.1.4", "Nov 3, 2026", [
   ("New & improved", ["<b>Concurrent downloads & integrity</b>: static CDN assets unthrottled with 32-way parallel fetching; streaming downloads accumulate SHA-1 in memory and detect ZIP magic, eliminating re-reads of thousands of small asset files.",
                       "<b>Smart Java matching</b>: reads the javaVersion declared in version JSON (including inheritsFrom chains; Java 8 / 17 / 21), matches it against installed runtimes before launch, and downloads a Temurin JRE from Adoptium when missing — extracted with zip-slip protection.",
                       "<b>Startup instrumentation</b>: end-to-end timing across the Rust backend and the webview; measured medians of ~1278ms cold and ~732ms warm starts.",
                       "<b>Distribution</b>: portable builds retired in favor of standard installers (NSIS / MSI / DMG) to avoid WebView2 and VC++ runtime pitfalls; signing and notarization guides added."]),
 ]),
 ("v0.1.3", "Nov 1, 2026", [
   ("Refactoring & platforms", ["<b>Platform layer & de-hardcoding</b>: a single platform.rs adaptation layer; every OS/architecture decision derived at runtime; official launch rules and classifier matching completed for macOS (arm64 & x64) and Linux.",
                                "<b>Download sources</b>: endpoints and mirror constants centralized in endpoints.rs with a unified BMCLAPI / MCIMirror routing policy."]),
 ]),
 ("v0.1.2", "Nov 1, 2026", [
   ("New features", ["<b>Modpacks & resources</b>: one-click import of Modrinth .mrpack modpacks, and export of the current instance as a standard .mrpack.",
                     "<b>Crash analysis</b>: on abnormal exit, log context is captured automatically to identify OOM, mod conflicts, driver crashes and bytecode mismatches, with targeted fix suggestions.",
                     "<b>Saves & assets</b>: browse and manage worlds, resource packs, shader packs and music, with quick folder access."]),
 ]),
 ("v0.1.1", "Nov 1, 2026", [
   ("New & fixed", ["<b>Installer rework</b>: automatic detection, dependency resolution and deep metadata merging for Vanilla, Fabric, Forge, NeoForge and OptiFine; concurrent downloads with SHA1 / SHA256 / SHA512 verification, resume and self-healing.",
                    "<b>LAN multiplayer</b>: room broadcast and discovery over UDP multicast, with invite-code generation and parsing."]),
 ]),
 ("v0.1.0", "Nov 1, 2026", [
   ("Initial release", ["<b>Core launcher</b>: a modern UI built with Tauri 2 + Vue 3 + TypeScript; a Rust core with offline sign-in and Microsoft OAuth2 device-code login; local version scanning, a version list and custom JVM arguments."]),
 ]),
]}

def changelog(lang):
    zh = lang == "zh"
    blocks = ""
    for ver, date, secs in CHANGELOG[lang]:
        blocks += f'<div class="rel" data-ver="{ver}"><h3>{ver}</h3><p class="date">{date}</p>'
        for sec_title, items in secs:
            lis = "".join(f"<li>{it}</li>" for it in items)
            blocks += f"<h4>{sec_title}</h4><ul>{lis}</ul>"
        blocks += "</div>"
    head_t = "更新日志" if zh else "Changelog"
    head_p = ("Pure Build Launcher 的全部版本记录。"
              if zh else "Every release of Pure Build Launcher.")
    body = f"""
<div class="page-head"><div class="wrap"><h1>{head_t}</h1><p>{head_p}</p></div></div>
<section style="padding-top:36px"><div class="wrap" data-changelog>{blocks}</div></section>"""
    title = "更新日志 — Pure Build Launcher" if zh else "Changelog — Pure Build Launcher"
    desc = "Pure Build Launcher 更新日志：v0.1.5 至 v0.1.0 全部版本记录。" if zh else "Pure Build Launcher changelog: all releases from v0.1.5 to v0.1.0."
    return page(lang, "changelog.html", title, desc, body)

def main():
    pages = {
        "index.html": home("zh"), "download.html": download("zh"), "changelog.html": changelog("zh"),
        "en/index.html": home("en"), "en/download.html": download("en"), "en/changelog.html": changelog("en"),
    }
    for rel, html in pages.items():
        path = os.path.join(SITE, rel)
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, "w", encoding="utf-8") as f:
            f.write(html)
        print("wrote", rel, len(html), "bytes")

if __name__ == "__main__":
    main()
