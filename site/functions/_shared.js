// Pure Build Launcher 官网服务端共享逻辑（不公开，仅函数内部使用）
// 职责：查最新 Release（边缘缓存 1 小时）、解析并清洗更新日志、提供兜底数据。
const API = "https://api.github.com/repos/demon820308/Pure-Build/releases/latest";
const CHANGELOG_URL = "https://raw.githubusercontent.com/demon820308/Pure-Build/main/CHANGELOG.md";

export const SLUGS = {
  "windows-setup": "x64-setup.exe",
  "windows-msi": ".msi",
  "macos-dmg": ".dmg",
  "linux-deb": ".deb",
  "linux-rpm": ".rpm",
  "linux-appimage": ".AppImage",
};

const DL = "https://github.com/demon820308/Pure-Build/releases/download/v0.1.6/";
export const FALLBACK = {
  version: "v0.1.6",
  date_iso: "2026-11-04",
  assets: {
    "windows-setup": { name: "Pure.Build.Launcher_0.1.6_x64-setup.exe", size_mb: 5.3, url: DL + "Pure.Build.Launcher_0.1.6_x64-setup.exe" },
    "windows-msi": { name: "Pure.Build.Launcher_0.1.6_x64_en-US.msi", size_mb: 7.2, url: DL + "Pure.Build.Launcher_0.1.6_x64_en-US.msi" },
    "macos-dmg": { name: "Pure.Build.Launcher_0.1.6_universal.dmg", size_mb: 17.2, url: DL + "Pure.Build.Launcher_0.1.6_universal.dmg" },
    "linux-deb": { name: "Pure.Build.Launcher_0.1.6_amd64.deb", size_mb: 7.6, url: DL + "Pure.Build.Launcher_0.1.6_amd64.deb" },
    "linux-rpm": { name: "Pure.Build.Launcher-0.1.6-1.x86_64.rpm", size_mb: 7.6, url: DL + "Pure.Build.Launcher-0.1.6-1.x86_64.rpm" },
    "linux-appimage": { name: "Pure.Build.Launcher_0.1.6_amd64.AppImage", size_mb: 82.0, url: DL + "Pure.Build.Launcher_0.1.6_amd64.AppImage" },
  },
};

async function cached(context, key, ttl, fetcher) {
  try {
    const cache = caches.default;
    const ck = new Request("https://pbl-cache.internal/" + key);
    const hit = await cache.match(ck);
    if (hit) return await hit.json();
    const data = await fetcher();
    const resp = new Response(JSON.stringify(data), {
      headers: { "Content-Type": "application/json", "Cache-Control": "max-age=" + ttl },
    });
    context.waitUntil(cache.put(ck, resp.clone()));
    return data;
  } catch (e) {
    return await fetcher();
  }
}

async function fetchLatest() {
  const r = await fetch(API, { headers: { "User-Agent": "pbl-website", "Accept": "application/vnd.github+json" } });
  if (!r.ok) throw new Error("release api " + r.status);
  const rel = await r.json();
  const assets = {};
  for (const [slug, suffix] of Object.entries(SLUGS)) {
    const a = (rel.assets || []).find((x) => x.name.endsWith(suffix));
    if (a) assets[slug] = { name: a.name, size_mb: Math.round((a.size / 1048576) * 10) / 10, url: a.browser_download_url };
  }
  if (!rel.tag_name || Object.keys(assets).length === 0) throw new Error("release payload incomplete");
  return { version: rel.tag_name, date_iso: (rel.published_at || "").slice(0, 10), assets };
}

export async function getRelease(context) {
  try {
    return await cached(context, "release-v1", 3600, fetchLatest);
  } catch (e) {
    return FALLBACK;
  }
}

// 合规清洗：站内不出现游戏商标词、厂商名与外部仓库字样/链接
export function sanitize(t) {
  return t
    .replace(/\[([^\]]+)\]\([^)]*\)/g, "$1")
    .replace(/https?:\/\/\S+/g, "")
    .replace(/Minecraft/g, "沙盒游戏")
    .replace(/Mojang\s*/g, "")
    .replace(/GitHub/g, "")
    .replace(/[ \t]{2,}/g, " ")
    .trim();
}

function mdInline(t) {
  t = sanitize(t);
  t = t.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
  t = t.replace(/\*\*([^*]+)\*\*/g, "<b>$1</b>").replace(/`([^`]+)`/g, "<code>$1</code>");
  return t;
}

function parseChangelog(md) {
  const sections = [];
  for (const part of md.split(/^## /m).slice(1)) {
    const head = part.match(/^\[?(v[0-9][^\]\s]*)\]?\s*-\s*(\d{4}-\d{2}-\d{2})/);
    if (!head) continue;
    const groups = [];
    let cur = null;
    for (const line of part.split("\n")) {
      const g = line.match(/^###\s+(.*)/);
      if (g) { cur = { title: mdInline(g[1]), items: [] }; groups.push(cur); continue; }
      const it = line.match(/^\s*-\s+(.*)/);
      if (it && cur) cur.items.push(mdInline(it[1]));
    }
    if (groups.length) sections.push({ version: head[1], date_iso: head[2], groups });
  }
  return sections;
}

async function fetchChangelog() {
  const r = await fetch(CHANGELOG_URL, { headers: { "User-Agent": "pbl-website" } });
  if (!r.ok) throw new Error("changelog " + r.status);
  return parseChangelog(await r.text());
}

export async function getChangelog(context) {
  try {
    return await cached(context, "changelog-v1", 3600, fetchChangelog);
  } catch (e) {
    return [];
  }
}
