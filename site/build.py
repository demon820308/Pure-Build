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

ICONS = {
    "download": '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/></svg>',
    "arrow": '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><line x1="5" y1="12" x2="19" y2="12"/><polyline points="12 5 19 12 12 19"/></svg>',
    "layers": '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><polygon points="12 2 2 7 12 12 22 7 12 2"/><polyline points="2 17 12 22 22 17"/><polyline points="2 12 12 17 22 12"/></svg>',
    "shield": '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/><polyline points="9 12 11 14 15 10"/></svg>',
    "key": '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><circle cx="7.5" cy="15.5" r="5.5"/><path d="m21 2-9.6 9.6"/><path d="m15.5 7.5 3 3L22 7l-3-3"/></svg>',
    "cpu": '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><rect x="4" y="4" width="16" height="16" rx="2"/><rect x="9" y="9" width="6" height="6"/><path d="M9 1v3M15 1v3M9 20v3M15 20v3M20 9h3M20 14h3M1 9h3M1 14h3"/></svg>',
    "zap": '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2"/></svg>',
    "box": '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z"/><polyline points="3.27 6.96 12 12.01 20.73 6.96"/><line x1="12" y1="22.08" x2="12" y2="12"/></svg>',
    "terminal": '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><polyline points="4 17 10 11 4 5"/><line x1="12" y1="19" x2="20" y2="19"/></svg>',
    "alert": '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3Z"/><line x1="12" y1="9" x2="12" y2="13"/><line x1="12" y1="17" x2="12.01" y2="17"/></svg>',
    "win": '<svg viewBox="0 0 24 24" fill="currentColor"><path d="M3 4.5 11 3.4v7.9H3V4.5Zm9-1.2L21 2v9.3h-9V3.3ZM3 12.7h8v7.9l-8-1.1v-6.8Zm9 0h9V22l-9-1.3v-8Z"/></svg>',
    "mac": '<svg viewBox="0 0 24 24" fill="currentColor"><path d="M16.36 12.76c-.03-2.62 2.14-3.88 2.24-3.94-1.22-1.78-3.12-2.02-3.79-2.05-1.6-.16-3.14.95-3.95.95-.82 0-2.07-.93-3.42-.9-1.74.03-3.36 1.02-4.26 2.58-1.83 3.17-.47 7.85 1.3 10.42.88 1.26 1.91 2.67 3.26 2.62 1.31-.05 1.8-.84 3.39-.84 1.58 0 2.04.84 3.41.81 1.42-.02 2.31-1.27 3.17-2.54 1-1.45 1.41-2.87 1.43-2.94-.03-.01-2.75-1.05-2.78-4.17ZM13.78 5.08c.72-.88 1.21-2.09 1.08-3.31-1.04.04-2.32.7-3.06 1.56-.66.76-1.24 2-1.09 3.17 1.17.09 2.35-.55 3.07-1.42Z"/></svg>',
    "linux": '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="4" width="18" height="16" rx="3"/><polyline points="8 10 11 13 8 16"/><line x1="13" y1="16" x2="17" y2="16"/></svg>',
}


def page(lang, slug, title, desc, body):
    prefix = "" if lang == "zh" else "../"
    nav_items = ""
    for label, href in NAV[lang]:
        active = ' class="active"' if href == slug else ""
        nav_items += f'<a href="{href}"{active}>{label}</a>'
    other = "en/" if lang == "zh" else "../"
    other_label = "EN" if lang == "zh" else "中文"
    cta_label = "下载" if lang == "zh" else "Download"
    footer_note = (
        "Pure Build Launcher 是独立的第三方启动器，与任何游戏开发商无关。"
        if lang == "zh"
        else "Pure Build Launcher is an independent third-party launcher, not affiliated with any game developer."
    )
    foot_links = ("下载", "更新日志") if lang == "zh" else ("Download", "Changelog")
    rights = "© 2026 Pure Build Launcher"
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
<a class="brand" href="index.html"><img src="{prefix}assets/app-icon.png" alt="PBL logo">Pure Build Launcher</a>
<nav class="links">{nav_items}<a class="lang" href="{other}{slug}">{other_label}</a><a class="nav-cta" href="download.html">{cta_label}</a></nav>
</div></header>
{body}
<footer><div class="wrap">
<span>{rights}</span>
<div class="right"><a href="download.html">{foot_links[0]}</a><a href="changelog.html">{foot_links[1]}</a></div>
</div><div class="wrap" style="margin-top:10px"><small>{footer_note}</small></div></footer>
<script src="{prefix}assets/release.js" defer></script>
</body></html>"""


FEATURES = {
    "zh": [
        ("01", "Loaders", "blue", "layers", "五大加载器全覆盖",
         "原版、Fabric、Forge、NeoForge、Quilt 与 OptiFine 全部真实安装：自动探测、拉取依赖并合并版本元数据，不用再手动拼文件。"),
        ("02", "Horizon", "green", "shield", "Horizon 原生引擎接入",
         "遵循官方 Launch API 契约，完整支持构建下载、SHA-256 与 Ed25519 验签、资源准备及独立实例管理，令牌仅经标准输入流交付。"),
        ("03", "Auth", "", "key", "三重登录方式",
         "微软正版（OAuth2 设备码授权）、外置 Yggdrasil 登录与离线模式并存；正版门禁与本地凭据隔离保存，随开随用。"),
        ("04", "Runtime", "", "cpu", "Java 运行时自动配齐",
         "按版本或构建协商结果自动校验 Java 8 / 17 / 21 / 25；本机缺失时从 Adoptium 官方源下载 Temurin 并安全解压。"),
        ("05", "32-Way", "", "zap", "高速并发与哈希自愈",
         "32 路并发拉取，流式累加 SHA1 / SHA256 / SHA512 校验；损坏文件自动重下，内置镜像路由与断点续传。"),
        ("06", "Ecosystem", "", "box", "整合包、联机与诊断",
         "Modrinth .mrpack 整合包一键导入导出；UDP 局域网房间自动发现与邀请码联机；崩溃日志自动诊断，附带内置音乐播放器。"),
    ],
    "en": [
        ("01", "Loaders", "blue", "layers", "Five loaders, real installs",
         "Vanilla, Fabric, Forge, NeoForge, Quilt and OptiFine with automated dependency resolution and version-metadata merging."),
        ("02", "Horizon", "green", "shield", "Native Horizon integration",
         "Built on the official Launch API: Ed25519 + SHA-256 build verification, automated resource preparation, per-instance isolation and stdin-only session handoff."),
        ("03", "Auth", "", "key", "Three ways to sign in",
         "Microsoft OAuth2 device-code flow, third-party Yggdrasil servers and offline mode, with credentials kept cleanly on your machine."),
        ("04", "Runtime", "", "cpu", "Java, sorted automatically",
         "Checks Java 8 / 17 / 21 / 25 against version metadata or API negotiation, and fetches verified Temurin builds from Adoptium when missing."),
        ("05", "32-Way", "", "zap", "Self-healing parallel downloads",
         "32-way concurrent fetching with streaming SHA1 / SHA256 / SHA512 checks, automatic corrupt-file recovery and mirror routing."),
        ("06", "Ecosystem", "", "box", "Modpacks, LAN & diagnostics",
         "One-click .mrpack import and export, UDP LAN room discovery with invite codes, automatic crash-log analysis and a built-in music bar."),
    ],
}


def home(lang):
    zh = lang == "zh"
    prefix = "" if zh else "../"

    if zh:
        eyebrow = "Tauri 2 + Rust 核心 · 双引擎沙盒启动器"
        h1 = '纯粹构建，<span class="accent">双引擎一手掌控。</span>'
        sub = "为 Windows、macOS 与 Linux 打造的一致启动体验：经典 Java 加载器与 Horizon 原生 Rust 引擎同台管理，现代深色与经典浅色随心切换。"
        dl_btn = '下载 <span data-rel="version">v0.1.7</span>'
        more_btn = "查看更新日志"
        ver = '最新版本 <span data-rel="version">v0.1.7</span> · <span data-rel="date">2026-11-06</span> · 现代深色 &amp; 经典浅色双布局'
        tabs = [
            ("shot-modern-horizon.jpg", "现代深色 · Horizon 构建与资源中心", "Ed25519 验签 · 资源就绪 · 实例管理"),
            ("shot-classic-horizon.jpg", "经典浅色 · 双引擎启动台", "正版授权 · Horizon 实例直启"),
        ]
        bench_eyebrow = "Benchmarks"
        bench_h2 = '更轻的体积，<span class="accent">更快的启动。</span>'
        bench_lede = "基于 Rust 原生后端与 Tauri 2 轻量化架构，去除冗余运行时开销，将等待时间压缩到毫秒级。"
        stats = [
            ("0.7", "s", "热启动中位耗时", "冷启动 ~1.28s · 实测打点", 92),
            ("32", "×", "静态资源并发拉取", "流式 SHA 校验 · 断点续传", 86),
            ("5+1", "", "加载器与原生引擎", "原版加载器 + Horizon Rust", 100),
            ("3", "", "全平台原生分发", "Win / macOS Universal / Linux", 95),
        ]
        split_eyebrow = "Architecture"
        split_h2 = '一套启动台，<span class="accent">两种运行内核。</span>'
        split_lede = "既保留传统 Java 生态的完整 Mod 兼容性，也率先遵循官方 Launch API 契约原生接入 Horizon 引擎。"
        pillars = [
            ("layers", "双布局，一套数据源", "现代深色侧栏与经典浅色看板共享同一份 Pinia 与 Rust 状态，无需重复配置实例与账号。"),
            ("shield", "端到端构建验签", "Horizon 构建下载强制校验 SHA256SUMS 与 Ed25519 签名，验签失败即整包清理，绝不降级放行。"),
            ("terminal", "契约驱动与令牌隔离", "启动参数严格来自 --launch-api 协商结果；账号会话仅通过标准输入流交付，不落盘、不进命令行。"),
        ]
        spec_title = "Horizon Launch API · 启动管线"
        spec_tag = "contract v0.1"
        spec_steps = [
            ("01", "Ed25519 + SHA-256 验签", "核对 SHA256SUMS 签名后解压至独立构建目录", "verified"),
            ("02", "--launch-api 参数协商", "运行时读取目标版本、Java 主版本与允许选项", "negotiated"),
            ("03", "--prepare / --check", "自动匹配版本文件并校验资源完整性", "ready"),
            ("04", "--session-stdin 会话交接", "令牌仅走标准输入流，不进参数与日志", "isolated"),
        ]
        feat_eyebrow = "Capabilities"
        feat_h2 = "该有的，一样不少"
        feat_lede = "从第一个 Java 运行时到原生 Horizon 构建，每一步都替你收敛好细节。"
    else:
        eyebrow = "Tauri 2 + Rust Core · Dual-Engine Launcher"
        h1 = 'The launcher you expect, <span class="accent">built for speed.</span>'
        sub = "One consistent experience across Windows, macOS and Linux — classic Java mod loaders and the native Horizon Rust client unified in Modern Dark and Classic Light layouts."
        dl_btn = 'Download <span data-rel="version">v0.1.7</span>'
        more_btn = "See the changelog"
        ver = 'Latest release <span data-rel="version">v0.1.7</span> · <span data-rel="date">Nov 6, 2026</span> · Modern Dark &amp; Classic Light'
        tabs = [
            ("shot-modern-horizon.jpg", "Modern Dark · Horizon Center", "Ed25519 verified · Ready resources"),
            ("shot-classic-horizon.jpg", "Classic Light · Dual Launch", "MSA verified · Horizon instance"),
        ]
        bench_eyebrow = "Benchmarks"
        bench_h2 = 'Smaller footprint, <span class="accent">faster launch.</span>'
        bench_lede = "Built on a Rust backend and Tauri 2 shell to eliminate heavy runtime bloat and keep startup under a second."
        stats = [
            ("0.7", "s", "Median warm start", "~1.28s cold · instrumented", 92),
            ("32", "×", "Concurrent asset fetch", "Streaming SHA checks & resume", 86),
            ("5+1", "", "Loaders & native engine", "Java loaders + Horizon Rust", 100),
            ("3", "", "Desktop OS targets", "Win / macOS Universal / Linux", 95),
        ]
        split_eyebrow = "Architecture"
        split_h2 = 'One workspace, <span class="accent">two execution paths.</span>'
        split_lede = "Keep full mod-loader compatibility for Java builds while launching verified native Horizon builds over the official Launch API."
        pillars = [
            ("layers", "Dual layouts, single source of truth", "Switch seamlessly between Modern Dark and Classic Light without reconfiguring accounts, directories or instances."),
            ("shield", "Strict Ed25519 verification", "Every Horizon build verifies SHA256SUMS and its Ed25519 signature before extraction — zero silent fallback."),
            ("terminal", "Contract-negotiated launch", "CLI flags are negotiated dynamically via --launch-api, and session tokens pass strictly over stdin without hitting disk or argv."),
        ]
        spec_title = "Horizon Launch API · Pipeline"
        spec_tag = "contract v0.1"
        spec_steps = [
            ("01", "Ed25519 + SHA-256 verify", "Check signed checksums before extracting build", "verified"),
            ("02", "--launch-api negotiation", "Derive game target, Java major and allowed flags", "negotiated"),
            ("03", "--prepare / --check", "Stage local assets and validate resource tree", "ready"),
            ("04", "--session-stdin handoff", "Stream auth token over stdin — never on disk or argv", "isolated"),
        ]
        feat_eyebrow = "Capabilities"
        feat_h2 = "Everything you need, nothing in the way"
        feat_lede = "Six core subsystems engineered for reliability across every platform."

    tab_btns = "".join(
        f'<button type="button" class="tab-btn{" active" if i == 0 else ""}" '
        f'data-shot="{prefix}assets/shots/{img}" data-meta="{meta}">{label}</button>'
        for i, (img, label, meta) in enumerate(tabs)
    )
    first_img, first_label, first_meta = tabs[0]

    stat_html = "".join(
        f'<div class="stat"><b>{val}<span class="unit">{unit}</span></b>'
        f'<span>{label}</span><small>{hint}</small>'
        f'<div class="stat-bar"><i style="width:{pct}%"></i></div></div>'
        for val, unit, label, hint, pct in stats
    )

    pillar_html = "".join(
        f'<div class="pillar"><div class="pillar-ic">{ICONS[ic]}</div>'
        f'<div><h3>{t}</h3><p>{d}</p></div></div>'
        for ic, t, d in pillars
    )

    feat_html = "".join(
        f'<div class="card"><div class="card-top"><span class="card-num">{num}</span>'
        f'<span class="tag {tone}">{tag}</span></div>'
        f'<h3>{ICONS[ic]}{t}</h3><p>{d}</p></div>'
        for num, tag, tone, ic, t, d in FEATURES[lang]
    )

    spec_html = "".join(
        f'<div class="spec-step"><span class="spec-idx">{idx}</span>'
        f'<div><div class="spec-name">{name}</div><div class="spec-desc">{desc}</div></div>'
        f'<span class="spec-badge">{badge}</span></div>'
        for idx, name, desc, badge in spec_steps
    )

    body = f"""
<div class="hero">
  <div class="wrap">
    <div class="hero-inner">
      <div class="eyebrow"><span class="dot"></span>{eyebrow}</div>
      <h1>{h1}</h1>
      <p class="sub">{sub}</p>
      <div class="cta">
        <a class="btn" href="download.html">{ICONS["download"]}<span>{dl_btn}</span></a>
        <a class="btn ghost" href="changelog.html"><span>{more_btn}</span>{ICONS["arrow"]}</a>
      </div>
      <div class="plat-strip">
        <a href="download.html#windows">{ICONS["win"]}<span>Windows</span></a>
        <a href="download.html#macos">{ICONS["mac"]}<span>macOS</span></a>
        <a href="download.html#linux">{ICONS["linux"]}<span>Linux</span></a>
      </div>
      <p class="ver">{ver}</p>
    </div>

    <div class="showcase">
      <div class="showcase-bar">
        <div class="showcase-tabs" role="tablist">{tab_btns}</div>
        <span class="showcase-meta" id="showcase-meta">{first_meta}</span>
      </div>
      <div class="showcase-stage">
        <img id="showcase-img" src="{prefix}assets/shots/{first_img}" alt="{first_label}">
      </div>
    </div>
  </div>
</div>

<section>
  <div class="wrap">
    <div class="panel">
      <div class="sec-eyebrow">{bench_eyebrow}</div>
      <h2>{bench_h2}</h2>
      <p class="lede">{bench_lede}</p>
      <div class="stats">{stat_html}</div>
    </div>
  </div>
</section>

<section class="bordered">
  <div class="wrap split">
    <div>
      <div class="sec-eyebrow">{split_eyebrow}</div>
      <h2>{split_h2}</h2>
      <p class="lede">{split_lede}</p>
      <div class="pillars">{pillar_html}</div>
    </div>
    <div class="spec-card">
      <div class="spec-head"><span>{spec_title}</span><span>{spec_tag}</span></div>
      <div class="spec-steps">{spec_html}</div>
    </div>
  </div>
</section>

<section class="bordered">
  <div class="wrap">
    <div class="sec-head">
      <div class="sec-eyebrow">{feat_eyebrow}</div>
      <h2>{feat_h2}</h2>
      <p class="lede">{feat_lede}</p>
    </div>
    <div class="grid">{feat_html}</div>
  </div>
</section>

<script>
document.querySelectorAll('.tab-btn').forEach(function(btn){{
  btn.addEventListener('click',function(){{
    document.querySelectorAll('.tab-btn').forEach(function(b){{b.classList.remove('active');}});
    btn.classList.add('active');
    var img=document.getElementById('showcase-img');
    var meta=document.getElementById('showcase-meta');
    if(img){{img.src=btn.dataset.shot;img.alt=btn.textContent;}}
    if(meta){{meta.textContent=btn.dataset.meta;}}
  }});
}});
</script>"""
    title = "Pure Build Launcher — 跨平台双引擎沙盒启动器" if zh else "Pure Build Launcher — Cross-platform Dual-Engine Launcher"
    desc = (
        "Pure Build Launcher：基于 Tauri 2 与 Rust 的跨平台沙盒游戏启动器，支持五大加载器与 Horizon 原生引擎。"
        if zh
        else "Pure Build Launcher: a fast, cross-platform sandbox game launcher built with Tauri 2 and Rust, supporting five Java loaders and native Horizon builds."
    )
    return page(lang, "index.html", title, desc, body)


DL_GROUPS = {
    "zh": [
        ("windows", "win", "Windows",
         "推荐使用安装程序（.exe），自动补齐 WebView2 与运行库；.msi 适合批量部署。",
         [("/dl/windows-setup", "Pure.Build.Launcher_0.1.7_x64-setup.exe", "安装程序 · x64 · 5.3 MB", True),
          ("/dl/windows-msi", "Pure.Build.Launcher_0.1.7_x64_en-US.msi", "MSI 安装包 · x64 · 7.2 MB", False)]),
        ("macos", "mac", "macOS",
         "通用包（Universal），Apple Silicon 与 Intel 均可原生运行；首次打开请查看下方命令。",
         [("/dl/macos-dmg", "Pure.Build.Launcher_0.1.7_universal.dmg", "DMG 磁盘映像 · Universal · 17.2 MB", True)]),
        ("linux", "linux", "Linux",
         "Debian / Ubuntu 用 .deb，Fedora / RHEL 用 .rpm，或直接运行 AppImage。",
         [("/dl/linux-deb", "Pure.Build.Launcher_0.1.7_amd64.deb", "DEB 包 · amd64 · 7.6 MB", False),
          ("/dl/linux-rpm", "Pure.Build.Launcher-0.1.7-1.x86_64.rpm", "RPM 包 · x86_64 · 7.6 MB", False),
          ("/dl/linux-appimage", "Pure.Build.Launcher_0.1.7_amd64.AppImage", "AppImage · amd64 · 82.0 MB", False)]),
    ],
    "en": [
        ("windows", "win", "Windows",
         "The setup program (.exe) is recommended — it installs WebView2 and runtime libraries automatically. The .msi suits managed deployments.",
         [("/dl/windows-setup", "Pure.Build.Launcher_0.1.7_x64-setup.exe", "Setup · x64 · 5.3 MB", True),
          ("/dl/windows-msi", "Pure.Build.Launcher_0.1.7_x64_en-US.msi", "MSI package · x64 · 7.2 MB", False)]),
        ("macos", "mac", "macOS",
         "A universal build for both Apple Silicon and Intel Macs. Please read the first-launch note below.",
         [("/dl/macos-dmg", "Pure.Build.Launcher_0.1.7_universal.dmg", "DMG image · Universal · 17.2 MB", True)]),
        ("linux", "linux", "Linux",
         ".deb for Debian / Ubuntu, .rpm for Fedora / RHEL, or AppImage to run without installing.",
         [("/dl/linux-deb", "Pure.Build.Launcher_0.1.7_amd64.deb", "DEB package · amd64 · 7.6 MB", False),
          ("/dl/linux-rpm", "Pure.Build.Launcher-0.1.7-1.x86_64.rpm", "RPM package · x86_64 · 7.6 MB", False),
          ("/dl/linux-appimage", "Pure.Build.Launcher_0.1.7_amd64.AppImage", "AppImage · amd64 · 82.0 MB", False)]),
    ],
}


def download(lang):
    zh = lang == "zh"
    groups = ""
    for gid, ic, name, hint, files in DL_GROUPS[lang]:
        items = ""
        for slug, fname, meta, rec in files:
            rec_label = "推荐" if zh else "Recommended"
            badge = f'<span class="rec-badge">{rec_label}</span>' if rec else ""
            btn = "下载" if zh else "Download"
            fdesc, fsize = meta.rsplit(" · ", 1)
            items += (
                f'<div class="dl-item" data-asset="{slug[4:]}">'
                f'<div><div class="fname"><span data-fname>{fname}</span>{badge}</div>'
                f'<div class="fmeta mono"><span>{fdesc}</span> · <span data-fsize>{fsize}</span></div></div>'
                f'<a class="btn" href="{slug}">{ICONS["download"]}<span>{btn}</span></a></div>'
            )
        groups += (
            f'<div class="dl-group" id="{gid}">'
            f'<div class="dl-head">{ICONS[ic]}<h3>{name}</h3></div>'
            f'<p class="hint">{hint}</p>{items}</div>'
        )
    if zh:
        head_t = "下载 Pure Build Launcher"
        head_p = '最新版本 <span class="mono" data-rel="version">v0.1.7</span> · 发布于 <span class="mono" data-rel="date">2026-11-06</span>'
        note_t = "macOS 首次打开提示「已损坏」或被拦截？"
        note_p = "本软件未经 Apple 付费证书签名，首次打开可能被 Gatekeeper 拦截。把应用拖入「应用程序」后，在终端执行下面一行命令即可正常打开："
        alt = "也可以在当前解压目录下对 .app 执行同一命令。历史版本说明见本站更新日志页。"
    else:
        head_t = "Download Pure Build Launcher"
        head_p = 'Latest release <span class="mono" data-rel="version">v0.1.7</span> · <span class="mono" data-rel="date">Nov 6, 2026</span>'
        note_t = "macOS says the app is damaged or blocked on first launch?"
        note_p = "The app is not signed with a paid Apple certificate, so Gatekeeper may block the first launch. After dragging the app into Applications, run this one line in Terminal:"
        alt = "You can also run the same command against the .app in the extraction folder. Older releases are listed on the changelog page."
    body = f"""
<div class="page-head"><div class="wrap"><h1>{head_t}</h1><p>{head_p}</p></div></div>
<section style="padding-top:36px"><div class="wrap">
{groups}
<div class="note"><h4>{ICONS["alert"]}<span>{note_t}</span></h4><p>{note_p}</p>
<pre><code>xattr -cr "/Applications/Pure Build Launcher.app"</code></pre>
<p style="margin-top:12px">{alt}</p></div>
</div></section>"""
    title = "下载 — Pure Build Launcher" if zh else "Download — Pure Build Launcher"
    desc = (
        "下载 Pure Build Launcher：Windows / macOS / Linux 官方安装包。"
        if zh
        else "Download Pure Build Launcher for Windows, macOS and Linux."
    )
    return page(lang, "download.html", title, desc, body)


# 更新日志数据：(版本, 日期, [(小节, [条目...]) ...])；zh 与仓库 CHANGELOG.md 一致，en 为译文
CHANGELOG = {
    "zh": [
        ("v0.1.7", "2026-11-06", [
            ("界面交互与质感升级", [
                "<b>全站矢量图标标准化</b>：引入轻量化纯矢量图标组件 <code>PclIcon</code>，全局替换原散落在各页面的 emoji 表情符号，统一经典与现代两套布局下的视觉设计语言。",
                "<b>音乐播放弹窗布局优化</b>：重构迷你播放器控制行，将调音预设下移至工具栏，完整保留循环模式文字按钮的同时将音量滑块大幅拓宽，解决音量条被挤压的问题。",
                "<b>标题栏品牌标识精炼</b>：优化经典布局顶部状态栏排版，去除多余的重复字母字标，保持界面干净清爽。",
                "<b>官网基础设施沉淀</b>：新增基于 Cloudflare Pages 与边缘函数的轻量化官网（<code>site/</code>），支持按架构自动分发多平台安装包。",
            ]),
        ]),
        ("v0.1.6", "2026-11-04", [
            ("架构与组件化重构", [
                "<b>LaunchView 架构模块化</b>：将主启动视图从 824 行巨石单文件重构为清晰内聚的子组件与 composable（<code>LaunchView.vue</code> 收敛至 68 行，下辖 <code>VersionPicker</code>、<code>JavaSelectPanel</code>、<code>LaunchClassicLayout</code>、<code>LaunchModernLayout</code> 与 <code>useLaunch</code>），各独立模块代码行数严格控制在 350 行以内。",
                "<b>状态单一事实源与双布局解耦</b>：彻底分家 PCL 经典布局与现代网格布局，消除两套视图间的模板冗余与状态污染，确保原版 PCL 像素级一致性。",
                "<b>启动生命周期与异常强杀治理</b>：通过 <code>useLaunch</code> composable 集中收敛后端启动进度监听、正常/异常退出处理与进程超时强制终止逻辑。",
            ]),
        ]),
        ("v0.1.5", "2026-11-04", [
            ("界面与体验修复", [
                "<b>音乐播放弹窗修复</b>：修复音量滑杆在经典与现代两套布局下溢出弹窗边框的问题；移除 EQ 预设图标，预设项目与音效参数保持不变。",
            ]),
            ("架构与内部重构", [
                "<b>后端模块化解耦</b>：启动参数构建、版本继承与文件哈希校验逻辑独立拆分为子模块。",
                "<b>前端状态与组件治理</b>：Pinia 全局状态按配置、版本、音乐、会话四大领域解耦拆分；抽离独立音乐播放条与登录认证弹窗组件；引入基于 ts-rs 的前后端强类型契约，类型漂移即测试报错中断。",
            ]),
            ("测试体系建设", [
                "<b>测试覆盖大幅补强</b>：后端测试由 29 个扩充至 46 个（新增真实版本 JSON 夹具的启动参数跨平台快照、认证解析与生命周期用例）；前端引入 vitest 并新增 8 个核心逻辑用例。",
            ]),
        ]),
        ("v0.1.4", "2026-11-03", [
            ("新增与优化", [
                "<b>资源并发下载与完整性优化</b>：静态 CDN 资源放行免节流，恢复 32 并发拉取；流式下载边收数据边在内存累加 SHA-1 并识别 ZIP 魔数，消除海量碎片小文件的二次读盘开销。",
                "<b>Java 版本智能校验与自动配齐</b>：读取版本 JSON（含 inheritsFrom 继承链）声明的 javaVersion（8 / 17 / 21），启动前自动比对本机清单，缺失时从 Adoptium 下载 Temurin JRE 并安全解压（路径规范化防 zip-slip）。",
                "<b>启动性能打点与秒开优化</b>：前后端全链路打点，实测冷启动中位数约 1278ms、热启动中位数约 732ms。",
                "<b>分发与打包架构规范</b>：弃用免安装绿色包，全面收敛为标准化安装器（NSIS / MSI / DMG），规避 WebView2 与 VC++ 运行时依赖隐患；补齐签名与公证指南。",
            ]),
        ]),
        ("v0.1.3", "2026-11-01", [
            ("重构与平台支持", [
                "<b>平台层解耦与反写死治理</b>：确立 platform.rs 唯一平台适配层，全仓清除硬编码平台字面量，系统与架构判定均由运行时推导；补齐官方启动规则与分类器匹配，支持 macOS（arm64 与 x64）及 Linux。",
                "<b>下载源架构解耦</b>：下载源与镜像常量集中至 endpoints.rs，统一 BMCLAPI 与 MCIMirror 镜像路由策略。",
            ]),
        ]),
        ("v0.1.2", "2026-11-01", [
            ("新增功能", [
                "<b>整合包与生态资源管理</b>：支持 Modrinth .mrpack 整合包一键导入，及导出当前实例为标准 .mrpack 包。",
                "<b>崩溃智能分析系统</b>：游戏异常退出时自动捕获日志上下文，识别内存溢出、Mod 兼容冲突、驱动崩溃与字节码版本不匹配并给出修复建议。",
                "<b>资源与存档管理</b>：扫描并呈现存档、材质包、光影包与背景音乐，支持直接操作与快速定位。",
            ]),
        ]),
        ("v0.1.1", "2026-11-01", [
            ("新增与修复", [
                "<b>版本安装器重构</b>：支持原版、Fabric、Forge、NeoForge 与 OptiFine 的自动探测、依赖拉取与版本元数据深度合并；并发下载与 SHA1 / SHA256 / SHA512 完整性校验，断点重试与损坏自愈。",
                "<b>局域网联机协作</b>：基于 UDP 组播协议的局域网房间广播与自动发现，支持生成并解析联机邀请码。",
            ]),
        ]),
        ("v0.1.0", "2026-11-01", [
            ("初始发布", [
                "<b>核心启动框架搭建</b>：Tauri 2 + Vue 3 + TypeScript 构建现代化 UI；Rust 核心支持离线登入与微软 OAuth2 设备码正版登录；本地版本扫描、版本列表展示与自定义 JVM 参数配置。",
            ]),
        ]),
    ],
    "en": [
        ("v0.1.7", "Nov 6, 2026", [
            ("UI & Aesthetic Polish", [
                "<b>Unified vector icons</b>: standardized across the whole app using the lightweight <code>PclIcon</code> component, replacing raw emoji characters for crisp rendering at any DPI.",
                "<b>Music player popover redesign</b>: reworked player controls by moving EQ tuning presets to the bottom toolbar and expanding the volume slider width to over 100px while keeping full loop-mode controls.",
                "<b>Streamlined title bar</b>: cleaned up redundant branding in the Classic layout header to match the clean design aesthetic.",
                "<b>Official website infrastructure</b>: added the Cloudflare Pages & Workers static site and download routing infrastructure (<code>site/</code>).",
            ]),
        ]),
        ("v0.1.6", "Nov 4, 2026", [
            ("Architecture & Component Refactor", [
                "<b>LaunchView modularization</b>: the main launch view was refactored from an 824-line monolith into cohesive subcomponents and a composable (<code>LaunchView.vue</code> down to 68 lines, with <code>VersionPicker</code>, <code>JavaSelectPanel</code>, <code>LaunchClassicLayout</code>, <code>LaunchModernLayout</code> and <code>useLaunch</code>), each module kept under 350 lines.",
                "<b>Single source of truth, decoupled layouts</b>: the classic PCL layout and the modern grid layout were fully separated, eliminating template duplication and state cross-talk between the two views while keeping the classic layout pixel-faithful to the original.",
                "<b>Launch lifecycle & force-kill governance</b>: launch-progress listening, normal/abnormal exit handling and process-timeout force termination were centralized in the <code>useLaunch</code> composable.",
            ]),
        ]),
        ("v0.1.5", "Nov 4, 2026", [
            ("Fixes & polish", [
                "<b>Music player popover fix</b>: the volume slider no longer overflows the popover frame in either the Classic or Modern layout; EQ preset icons were removed, presets and sound unchanged.",
            ]),
            ("Architecture", [
                "<b>Backend modularization</b>: launch-argument building, version inheritance and file-hash verification split into dedicated submodules.",
                "<b>Frontend state & components</b>: the Pinia store split into config, versions, music and session domains; music player bar and login dialog extracted as components; a ts-rs type contract between Rust and TypeScript — drift fails the test suite.",
            ]),
            ("Testing", [
                "<b>Test coverage push</b>: backend tests grew from 29 to 46 (launch-argument snapshots from real version JSON fixtures, auth parsing and lifecycle cases); the frontend gained vitest with 8 core-logic cases.",
            ]),
        ]),
        ("v0.1.4", "Nov 3, 2026", [
            ("New & improved", [
                "<b>Concurrent downloads & integrity</b>: static CDN assets unthrottled with 32-way parallel fetching; streaming downloads accumulate SHA-1 in memory and detect ZIP magic, eliminating re-reads of thousands of small asset files.",
                "<b>Smart Java matching</b>: reads the javaVersion declared in version JSON (including inheritsFrom chains; Java 8 / 17 / 21), matches it against installed runtimes before launch, and downloads a Temurin JRE from Adoptium when missing — extracted with zip-slip protection.",
                "<b>Startup instrumentation</b>: end-to-end timing across the Rust backend and the webview; measured medians of ~1278ms cold and ~732ms warm starts.",
                "<b>Distribution</b>: portable builds retired in favor of standard installers (NSIS / MSI / DMG) to avoid WebView2 and VC++ runtime pitfalls; signing and notarization guides added.",
            ]),
        ]),
        ("v0.1.3", "Nov 1, 2026", [
            ("Refactoring & platforms", [
                "<b>Platform layer & de-hardcoding</b>: a single platform.rs adaptation layer; every OS/architecture decision derived at runtime; official launch rules and classifier matching completed for macOS (arm64 & x64) and Linux.",
                "<b>Download sources</b>: endpoints and mirror constants centralized in endpoints.rs with a unified BMCLAPI / MCIMirror routing policy.",
            ]),
        ]),
        ("v0.1.2", "Nov 1, 2026", [
            ("New features", [
                "<b>Modpacks & resources</b>: one-click import of Modrinth .mrpack modpacks, and export of the current instance as a standard .mrpack.",
                "<b>Crash analysis</b>: on abnormal exit, log context is captured automatically to identify OOM, mod conflicts, driver crashes and bytecode mismatches, with targeted fix suggestions.",
                "<b>Saves & assets</b>: browse and manage worlds, resource packs, shader packs and music, with quick folder access.",
            ]),
        ]),
        ("v0.1.1", "Nov 1, 2026", [
            ("New & fixed", [
                "<b>Installer rework</b>: automatic detection, dependency resolution and deep metadata merging for Vanilla, Fabric, Forge, NeoForge and OptiFine; concurrent downloads with SHA1 / SHA256 / SHA512 verification, resume and self-healing.",
                "<b>LAN multiplayer</b>: room broadcast and discovery over UDP multicast, with invite-code generation and parsing.",
            ]),
        ]),
        ("v0.1.0", "Nov 1, 2026", [
            ("Initial release", [
                "<b>Core launcher</b>: a modern UI built with Tauri 2 + Vue 3 + TypeScript; a Rust core with offline sign-in and Microsoft OAuth2 device-code login; local version scanning, a version list and custom JVM arguments.",
            ]),
        ]),
    ],
}


def changelog(lang):
    zh = lang == "zh"
    blocks = ""
    for ver, date, secs in CHANGELOG[lang]:
        blocks += f'<div class="rel" data-ver="{ver}"><div class="rel-head"><h3>{ver}</h3><p class="date">{date}</p></div>'
        for sec_title, items in secs:
            lis = "".join(f"<li>{it}</li>" for it in items)
            blocks += f"<h4>{sec_title}</h4><ul>{lis}</ul>"
        blocks += "</div>"
    head_t = "更新日志" if zh else "Changelog"
    head_p = "Pure Build Launcher 的全部版本记录。" if zh else "Every release of Pure Build Launcher."
    body = f"""
<div class="page-head"><div class="wrap"><h1>{head_t}</h1><p>{head_p}</p></div></div>
<section style="padding-top:36px"><div class="wrap" data-changelog>{blocks}</div></section>"""
    title = "更新日志 — Pure Build Launcher" if zh else "Changelog — Pure Build Launcher"
    desc = (
        "Pure Build Launcher 更新日志：v0.1.7 至 v0.1.0 全部版本记录。"
        if zh
        else "Pure Build Launcher changelog: all releases from v0.1.7 to v0.1.0."
    )
    return page(lang, "changelog.html", title, desc, body)


def main():
    pages = {
        "index.html": home("zh"),
        "download.html": download("zh"),
        "changelog.html": changelog("zh"),
        "en/index.html": home("en"),
        "en/download.html": download("en"),
        "en/changelog.html": changelog("en"),
    }
    for rel, html in pages.items():
        path = os.path.join(SITE, rel)
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, "w", encoding="utf-8") as f:
            f.write(html)
        print("wrote", rel, len(html), "bytes")


if __name__ == "__main__":
    main()
