import { getRelease, SLUGS } from "../_shared.js";

// 下载短链：点击时按最新 Release 实时解析对应安装包并 302 跳转；查询失败走兜底版本。
export async function onRequest(context) {
  const slug = context.params.slug;
  if (!(slug in SLUGS)) return new Response("Not found", { status: 404 });
  const rel = await getRelease(context);
  const a = rel.assets[slug];
  if (!a) return new Response("Not found", { status: 404 });
  return Response.redirect(a.url, 302);
}
