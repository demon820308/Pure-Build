# Pure Build Launcher 官网（site/）

Cloudflare Pages 项目 `pure-build-launcher`（https://pure-build-launcher.pages.dev/）绑定本仓库自动部署：

- 构建设置：根目录 `/`，构建命令 `python3 site/build.py`，输出目录 `site`，生产分支 `main`。
- 建议在 Pages 面板开启 Build watch paths，只让 `site/**` 的变动触发部署。
- 六页 HTML 由 `build.py` 在构建时生成，已在仓库根 `.gitignore` 中忽略，不要手工提交生成物。
- 文案与数据改 `build.py`；样式改 `assets/style.css`；页面回填脚本 `assets/release.js`。

## 网站端自更新（运行时）

- `functions/api/release.js`（仓库根目录 functions/）→ `/api/release`：查最新 Release 并下发版本/包名/大小/清洗后的更新日志（不下发真实下载地址），边缘缓存 1 小时，失败走 `_shared.js` 的 FALLBACK。
- `functions/dl/[slug].js` → `/dl/<slug>`：点击时实时解析最新版对应安装包并 302 跳转。
- 页面脚本按 data 属性回填版本号/日期/文件名/大小，更新日志页自动补入新段落（按 data-ver 去重）。
- 静态 HTML 内容是无 JS/接口故障时的底稿，长期落后时改 build.py 数据刷新一次即可。

## 合规口径（铁律）

页面文案、图片与接口下发内容中不得出现某沙盒游戏的商标词、其开发商名与代码托管平台的字样/链接：仓库 CHANGELOG 原文导入前须按站内口径改写；根目录 `functions/_shared.js` 的 sanitize 会在下发前再清洗一遍。截图须先打码图内同类字样与 `.minecraft` 路径并放大复验后再入库。

注：`assets/` 下的图片（图标与截图）为二进制文件，经 GitHub 网页端上传；文本文件走常规提交。
