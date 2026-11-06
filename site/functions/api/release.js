import { getRelease, getChangelog } from "../_shared.js";

// 本站数据接口：只下发版本号、日期、包名、大小与清洗后的日志，绝不下发真实下载地址。
export async function onRequest(context) {
  const rel = await getRelease(context);
  const changelog = await getChangelog(context);
  const assets = {};
  for (const [slug, a] of Object.entries(rel.assets)) {
    assets[slug] = { name: a.name, size_mb: a.size_mb };
  }
  return Response.json(
    { version: rel.version, date_iso: rel.date_iso, assets, changelog },
    { headers: { "Cache-Control": "public, max-age=600" } }
  );
}
