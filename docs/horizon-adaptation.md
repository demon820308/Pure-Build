# Pure-Build × Horizon 适配改造单

> 执行方：Gemini。下单人：Muse（验收方）。日期：2026-11-16。  
> 基线：Pure-Build v0.1.7（仓库 demon820308/Pure-Build，main / e733e93）。  
> 目标：让 Pure Build Launcher（下称 PBL）成为 Horizon 的第三方启动器——在 PBL 内完成 Horizon 构建的下载验签、资源准备、账号交接与启动，全程走 Horizon 官方 Launch API 契约，不做任何非官方 hack。

---

## 一、依据（双方事实，均已核实，不许凭印象改）

### Horizon 侧（上游 horizon-rust/horizon，契约文档在 docs/api/）

1. **Launch API 0.1.0（experimental）**：`docs/api/launch.md` + `launch-guide.md` + `launch-reference.md` 是官方给第三方启动器的契约，点名欢迎第三方接入。契约要点：
   - 启动前先协商：`horizon-client --launch-api` 输出 JSON（`launch_api`、`horizon`、`minecraft`、 `options[]`、`content`、`exit_codes`、`crash_report_line`）。**契约之外的选项会被以退出码 2 拒绝**，所以选项只能来自协商结果，不许猜。
   - 账号交接：会话 JSON `{"name","uuid","access_token","user_type":"msa","xuid"?}`，经 `--session-stdin`（首选）或 `--session-file` 交给游戏；**绝不许出现在命令行参数里**。
   - 所有权门禁：只许为「拥有 Minecraft: Java Edition 的微软账号」启动；Horizon 本体不含任何 Mojang 文件，也不下载它们。
   - 每实例一文件夹：`horizon-client --resources PREPARED --game-dir INSTANCE --home INSTANCE [--width N] [--height N] [--fullscreen]`。
   - 退出码：0 正常；2 选项被拒（stderr 点名被拒选项）；1 / 101 崩溃（stderr 末行或协商给出的 `crash_report_line` 前缀指出崩溃报告路径）。
2. **资源准备**：`horizon-prepare` 把玩家自己的正版文件转成 `--resources` 读的资源，自带完整契约：
   - `horizon-prepare --launch-api` 输出它钉住的 `minecraft` 版本、`java_major`、`output_version`、`schema` 与选项表；
   - 准备：`horizon-prepare --prepare --output DIR --complete --progress json`（`--progress json` 每行一个 JSON 对象：`message` / `fraction` / `error`，是 Launch API 的一部分）；
   - 校验：`horizon-prepare --check --output DIR`，0 = 完整且为当前版，1 = 有缺口（stdout 逐条列出）；
   - 文件指认：布局与官方启动器不同时，用 `--version-json` / `--client-jar` / `--libraries-dir` / `--assets-dir` 逐个指认（每个文件仍按 `research-lock.json` 的钉子校验）；
   - `--server-jar`：仅首次准备需要，Horizon 永不下载它，由启动器为已登录的正版账号下载（版本 JSON 的 `downloads.server`）；
   - `--java-home`：启动器自管的 Java（版本须为协商给出的 `java_major`）。
3. **构建分发**：GitHub releases（horizon-rust/horizon），每平台一个包：Windows x64 `horizon-windows-x64.zip`、Windows ARM64 `horizon-windows-arm64.zip`、Linux x86_64 `horizon-linux-x86_64.tar.gz`、macOS ARM64 `horizon-macos-arm64.zip`、macOS Intel `horizon-macos-x64.zip`。当前只有 `nightly` 渠道（另有 `nightly-<日期>` 标签，**只保留 7 天**）。每版附 `SHA256SUMS` 与 Ed25519 签名 `SHA256SUMS.sig`（签名消息为 `horizon release <tag>\n` + SHA256SUMS 内容），公钥在上游仓 `crates/horizon-release-sign/release-keys.txt`。
4. **构建包是一个整体**：`horizon-client`、`horizon-prepare`、`research-lock.json`、`exporters/` 必须同目录共存，解压后整包不得拆散、不得改名移动单个文件。
5. **每个构建只钉一个 Minecraft 版本**（当前为 26.3，以协商结果为准，不许写死）。
6. macOS 构建为 ad hoc 签名：首次启动需清 quarantine 标记（指南给启动器的口径：由启动器清）。

### PBL 侧（基线代码已核对）

1. 微软登录链完整：`crates/pcl-base/src/auth.rs` 的 `MicrosoftAuth` 已走完设备码 → MS token → XBL → XSTS → MC 登录 → Profile，`UserProfile` 持有 `name` / `uuid` / `access_token`——正是会话 JSON 需要的字段（`xuid` 契约里可选，没有就不带）。
2. Java 管理现成：`crates/pcl-base/src/java.rs` 有 `JavaScanner::select_best_matching_java(major)` 与 `JavaDownloader::download_and_extract_temurin(major)`（Adoptium，落 `<game_dir>/runtime/java-<major>`），`src-tauri` 有 `ensure_java_version` 命令。备 Java 25 不需要新管线。
3. 服务端 jar 下载现成：`crates/pcl-base/src/installer/server.rs` 已按版本 JSON 的 `downloads.server` 下载。
4. 版本文件管理现成：PBL 已管理 `versions/` 下的版本 JSON、客户端 jar、`libraries/`、`assets/`——正是 prepare 要指认的四件。
5. 进程托管有参照：`crates/pcl-base/src/launch/process.rs` 是 Java 专用管线（参照其 reporter / 退出上报模式，**不许改它的行为**，Horizon 另起原生进程管线）。
6. 前端服务层在 `src/services/backend.ts`，DTO 走 ts-rs 导出（`crates/pcl-base/tests/type_export_tests.rs` 是既有机制）；窗口宽高在 `AppConfig.window_width / window_height`。

---

## 二、总原则（违反任一，整批退回）

1. **禁止补丁式修改**：在源头按完整逻辑单元实现；不叠特判、不留旧逻辑并行、不加「以防万一」的 fallback 分支。
2. **禁止硬编码（写死）**：凡随环境/版本变化的值一律运行时推导——游戏版本钉子、`java_major`、可用选项清单、崩溃行前缀，全部来自 `--launch-api` 协商结果；平台包名由现有 `platform.rs` 的平台判定推导。允许写死的只有契约本身固定的东西：会话 JSON 字段名、退出码语义、API 字段名。**验收会 grep `26.3` 等字面量：生产代码中出现即退回**（测试 fixture 除外）。
3. **一文件一职责**：Horizon 适配全部落在新模块 `crates/pcl-base/src/horizon/`，按第三节的文件划分实现；单文件触红线当场按缝拆，不许把五摊事塞进一个文件。
4. **令牌卫生**：access_token 只许经 stdin 进游戏进程；不进 argv、不落盘、不写日志、不进 reporter 事件。日志与进度事件里出现 token 即退回。
5. **保留清单（禁改）**：`PCL2-LINK-` / `PCL2-HOST-` 邀请码前缀、下载 UA 与 Referer；Java 版启动参数与行为；现有实例/版本的数据格式。
6. 每批独立提交、CI 全绿才算该批完成；每批附新增单测（见各批验收）。

---

## 三、数据布局与模型（先定死，执行方不另行发挥）

在现有 `game_dir` 下新增 `horizon/` 子树：

```text
<game_dir>/horizon/
  builds/<tag>/            # 一个构建的完整解压包（不可变，含 horizon-client、horizon-prepare、research-lock.json、exporters/）
  builds/<tag>/pbl-build.json   # PBL 自记元数据：tag、平台包名、SHA-256、协商得到的 horizon 版本 / minecraft 钉子 / launch_api 版本、安装时间
  builds/<tag>/server.jar  # 该构建首次准备用的服务端 jar（按其 minecraft 钉子下载，仅正版账号）
  resources/<tag>/         # 该构建准备好的资源（horizon-prepare --output 的目标），同构建的实例共享
  instances/<name>/        # 实例目录，同时作 --game-dir 与 --home（存档、日志、崩溃报告都在此）
```

- 构建身份以**安装时协商结果**为准（跑一次 `horizon-client --launch-api` 与 `horizon-prepare --launch-api` 写入 `pbl-build.json`），不许从 tag 字符串里解析版本号。
- 资源与构建一对一绑定：换构建必须用新构建的 `horizon-prepare` 重新准备到 `resources/<新tag>/`，不许跨构建复用资源目录（`--check` 的 output_version 就是干这个的）。
- 实例只记三件事：名字、绑定的构建 tag、创建时间。存一个 Horizon 实例注册表（放 `horizon/instances.json`，由新模块自己管，不侵入现有版本/实例结构）。

---

## 四、分批任务

### 批 1 · 后端核心（crates/pcl-base）

新建 `crates/pcl-base/src/horizon/`，`lib.rs` 注册模块：

| 文件 | 职责 |
|---|---|
| `mod.rs` | 门面：对外导出类型与入口，不放实现细节 |
| `api.rs` | 协商层：跑 `horizon-client --launch-api` / `horizon-prepare --launch-api`，解析为 DTO（未知字段忽略，循契约「readers ignore fields they don't know」）；Launch API 大版本不兼容时返回明确错误 |
| `build.rs` | 构建发现与安装：查 GitHub releases → 按平台选包 → 下载 → 验 SHA256SUMS → 验 Ed25519 签名 → 解压到 `builds/<tag>/` → 协商并写 `pbl-build.json`。发布公钥集中定义在本文件常量区，注明来源（上游 `crates/horizon-release-sign/release-keys.txt`）与对应上游提交；验签失败整包删除并报错，不许降级为「只验 SHA」 |
| `prepare.rs` | 准备管线：组装 prepare 参数（优先逐文件指认 PBL 已管的版本文件；该版本未安装时先走现有版本安装管线，仍缺则明确报缺哪件）、`--java-home` 指向自管 Java、`--progress json` 逐行解析为现有 `DownloadProgress` 事件、跑前/跑后 `--check` 门禁 |
| `session.rs` | 会话组装：从 `UserProfile` 生成会话 JSON（`user_type` 固定 `msa`；`xuid` 无则省略）；只返回内存中的字节，不提供任何落盘方法 |
| `launch.rs` | 原生进程托管：按协商选项表组装参数（`--resources` / `--game-dir` / `--home` / `--width` / `--height`，宽高取 `AppConfig`）、`--session-stdin` 写入会话后关闭 stdin、捕获 stderr 尾部、退出码映射（0 正常 / 2 契约错误，报「构建与启动器不兼容」类明确错误 / 1·101 崩溃，按协商的 `crash_report_line` 前缀从 stderr 末行取报告路径并走现有 `crash.rs` 通路）、PID 登记以便复用 `kill_game_process` 的管理口径 |

配套改动：
- `endpoints.rs`：按现有模式加 GitHub releases 相关 URL 构造（horizon-rust/horizon 的 releases API 与下载地址），不许把 URL 散落在 horizon 模块里拼。
- 验签需要 Ed25519 验签能力：优先复用 Cargo.lock 里已有的传递依赖；没有则加 `ed25519-dalek`（只用 verify 功能），在提交说明里写明理由。

**批 1 验收**：新增单测覆盖——协商 JSON 解析（fixture 取自真实 `--launch-api` 输出结构）、会话 JSON 组装（字段齐全、无多余字段）、验签（正确向量通过、篡改 SHA256SUMS 必失败）、退出码映射、prepare 参数组装（指认四件套路径正确）。CI 全绿。

### 批 2 · Tauri 命令与前端类型

`src-tauri/src/lib.rs` 按现有命令模式新增（命名循现有 snake_case 风格）：

- `fetch_horizon_builds`：远端可装构建列表（tag、日期、平台包大小）与本地已装状态合并返回
- `install_horizon_build(tag)`：下载验签解压协商，进度走现有 reporter 通道
- `delete_horizon_build(tag)`：删构建与对应 resources（有实例绑定的构建拒绝删除并说明）
- `prepare_horizon_resources(tag)`：准备/重备资源，进度事件同批 1
- `check_horizon_resources(tag)`：返回 `--check` 结果（完整 / 缺口清单）
- `list_horizon_instances` / `create_horizon_instance(name, tag)` / `delete_horizon_instance(name)`
- `launch_horizon(instance_name)`：账号门禁（见批 3 门禁规则，后端再查一遍，不只靠前端）→ 资源 `--check` 不过先自动转准备 → 启动并托管

DTO 全部经 ts-rs 导出到前端（循 `type_export_tests` 机制），`src/services/backend.ts` 补对应类型与 invoke 封装。

**批 2 验收**：`type_export_tests` 更新且通过；命令层无业务逻辑复制（只调 horizon 模块）；CI 全绿。

### 批 3 · 前端 UI

- **下载中心（DownloadView）**：新增「Horizon」区：构建列表（tag、日期、已装标记）、安装/删除、资源状态（未准备 / 准备中 / 已就绪 / 有缺口）与「准备资源」按钮，进度复用现有下载悬浮卡与进度组件。
- **启动页（LaunchView，经典与现代两布局都要）**：实例列表合并显示 Horizon 实例（带类型徽标，与 Java 实例视觉可辨）；选中 Horizon 实例时启动按钮走 `launch_horizon`；窗口宽高沿用现有设置。
- **实例管理（InstanceView）**：Horizon 实例只显示「存档」页签（存档格式与 Java 版同为 Anvil，可复用现有存档列表）；模组 / 光影 / 资源包页签对 Horizon 实例不显示（那是 Java 语义，Horizon 的内容由游戏在 game-dir 内自管，本期不做，见非范围）。
- **账号门禁**：当前账号非微软正版时，Horizon 的安装准备可以看，但「启动」禁用并明示原因（契约的所有权门禁）；离线 / 外置登录账号同理。门禁文案说清「Horizon 需要正版微软账号」即可，不许写成系统错误。
- **崩溃**：Horizon 崩溃走现有 CrashDialog，展示报告路径与打开入口（复用现有崩溃 UI 的数据通路）。

**批 3 验收**：双布局各走通一遍冒烟（装构建 → 准备 → 启动 → 正常退出；杀进程；崩溃路径如能构造则验）；UI 文案无内部术语；CI 全绿。

### 批 4 · 更新与健壮性

- **更新检查**：启动时（或进下载中心时）对比已装 tag 与远端 `nightly`：有新构建时在 Horizon 区提示「有新构建」，用户点装才装；**不自动替换**。提示里注明上游构建只保留 7 天、已装构建本地保留不受影响。
- **启动前门禁**：每次启动前跑 `--check`；不过 → 自动重备（进度可见）→ 再不过 → 明确报错停住，不许带病启动。
- **协商失败降级**：`--launch-api` 失败或大版本不兼容 → 该构建标记为不可启动并给出原因（构建损坏 / API 不兼容），不许回退到「按默认选项硬启」。
- **macOS**：安装完成后对构建目录清 quarantine（`xattr -dr com.apple.quarantine`），失败只警告不拦安装。
- **musl Linux**：不处理（上游指南明说其官方启动器也不装 musl 包，PBL 只认 glibc 包）。

**批 4 验收**：断网、验签失败、资源损坏（手动删 resources 里一个文件）、协商失败四种故障各有明确报错且不崩溃；CI 全绿。

---

## 五、非范围（本期不做，做了算超范围）

- Horizon 模组（.hmod / 原生模组）、光影包、资源包在 PBL 内的管理 UI（游戏内自管）；
- Horizon 专用服务器（horizon-server）的托管；
- 联机 / PCL2-LINK 与 Horizon 的任何打通；
- Java 版管线的任何行为变更；官网（site/）文案。

---

## 六、验收方（Muse）复验清单

1. `grep` 生产代码：`26.3`、`--render-distance` 等版本/选项字面量为零（fixture 除外）；token 不出现在日志/事件/argv 的任何路径上；
2. horizon 模块文件划分与第三节一致，无单文件多摊事、无转发壳；
3. 批 1–4 的新增单测真实存在且断言的是行为（协商解析、验签、退出码、参数组装），不是凑数；
4. 保留清单逐项未动（邀请码前缀、UA/Referer、Java 启动参数快照测试不变）；
5. CI 全绿；每批提交说明写清改了什么、为什么。

> 上游契约出处（执行前必读原文，不要只看本单转述）：horizon 仓 `docs/api/launch.md`、`docs/api/launch-guide.md`、`docs/api/launch-reference.md`、`docs/launcher-preparation.md`、`docs/microsoft-account.md`。契约与本单冲突时以契约为准，并回报下单人。
