# PCL-Vue3 macOS 运行规范 ＋ 反写死规范（项目强制铁律）

> **强制执行指令**：每次在本项目（`Vue 3 TCL`）中新增、修改或重构任何前端（`src/`）或后端（`crates/pcl-base`、`src-tauri/`）代码时，**必须**首先对照并严格遵守以下规范。任何违反本规范的平台字面量、路径写死或散落配置均严禁提交。

---

## 0. 铁律（优先级高于一切排版、进度与「先跑起来再说」）

**没有运行时推导，不许写平台/机器相关的字面量。**

以下说法一律视为借口，不构成跳过理由：「Windows 上已经能跑」「先写死、以后再改」「这个值一般不会变」「用户自己改配置就行」「只是个默认值」。

- 凡是会随**操作系统、CPU 架构、用户、机器、安装位置**变化的值，必须在运行时从系统/框架推导，或由用户在设置中配置。推导方式见第 2 节对照表。
- 凡是与平台相关的**行为差异**（打开方式、杀进程、可执行文件名、脚本格式、目录位置），只许收敛到第 1 节规定的唯一平台层，其他模块不许出现平台判断。
- 产品级常量（如默认内存 4096MB）不算写死，但必须以**命名常量集中定义一处**并注明来源，禁止散落在业务代码中以魔法数字/字符串形式重复出现。
- 本规范与执行方自身习惯冲突时，以本规范为准；规范未覆盖的平台问题，按「运行时推导」原则自行处理并在交付报告中说明。

---

## 1. 唯一平台层（架构要求，先做这一步）

新建 `crates/pcl-base/src/platform.rs`，作为全仓**唯一**允许出现 `#[cfg(target_os = ...)]`、`cfg!(windows)`、平台命令名与平台路径的模块，对外只暴露语义化函数，至少包括：

| 函数 | 语义 |
|---|---|
| `mc_os_name() -> &'static str` | Minecraft 版本 JSON 的 os.name 取值：Windows→`windows`，macOS→`osx`，Linux→`linux`（注意是 `osx` 不是 `macos`，以 Mojang 规范为准） |
| `mc_os_arch() -> &'static str` | 规则匹配用架构：`x86` / `x64` / `arm64`，由 `std::env::consts::ARCH` 映射（aarch64→arm64、x86_64→x64、x86→x86） |
| `default_game_dir() -> PathBuf` | 按平台推导的默认 .minecraft 目录（见 §3.2） |
| `java_search_dirs() -> Vec<PathBuf>` | 本平台所有 Java 候选目录（见 §3.3） |
| `java_executable_name() -> &'static str` | Windows→`javaw.exe`，其余→`java` |
| `open_folder(path)` / `open_url(url)` | 按平台调用 explorer / `open` / `xdg-open` |
| `kill_process(pid)` | Windows→`taskkill /F`，其余→`kill -9` |
| `server_script(dir, version_id, jar_name)` | 生成本平台开服脚本并设置权限（见 §3.5） |
| `classpath_separator() -> char` | 由 `std::path` 语义推导，禁止手写 `;`/`:` 字面量二选一散落在外 |

可测试性要求：规则判定、classifier 选择等**纯逻辑函数必须把 os/arch 作为参数传入**，不许在函数内部直接读全局 cfg/env，以便在任意系统上用测试向量验证三平台行为（见第 8 节）。

改造完成后：`launch.rs`、`java.rs`、`resource.rs`、`installer.rs`、`config.rs`、`src-tauri/src/lib.rs` 中不许再残留任何平台字面量与 cfg 分支（第 5 节有自查命令）。

---

## 2. 反写死对照表（值 → 唯一合法来源）

| 禁止写死的值 | 合法来源 |
|---|---|
| OS 名称（`"windows"` 等） | `std::env::consts::OS` 经 platform.rs 映射 |
| 架构（x64/arm64） | `std::env::consts::ARCH` 经 platform.rs 映射；Java 架构另以 `java -version`/`os.arch` 输出实测为准 |
| 游戏目录默认值（现状：`D:\Program Files (x86)\.minecraft` 写死于 config.rs、src/stores/app.ts、src-tauri/pcl_config.json 三处） | platform.rs 按平台推导（§3.2）；前端与配置只保存用户实际值，不内置平台默认 |
| 配置文件位置（现状：相对路径 `pcl_config.json`，随工作目录漂移） | Tauri `app.path().app_config_dir()`；取不到时的回退顺序也要写在 platform/config 层内 |
| Java 路径与候选目录 | 环境变量（PATH、JAVA_HOME）＋ platform.rs 的本平台目录清单（§3.3） |
| 路径拼接 | 一律 `PathBuf::join` / `Path::join`；禁止字符串拼 `\` 或 `/`，禁止把路径存成预拼接字符串再拆 |
| 下载源/镜像地址（现状：`bmclapi2.bangbang93.com` 在 download.rs 与 installer.rs 中散布 14 处） | 集中到单独的 endpoints 常量模块：官方源为默认、镜像为可切换列表，顺序与开关可在设置中配置；业务代码只引用常量名 |
| 外部命令名（explorer/open/xdg-open/taskkill/kill） | platform.rs |
| 开服脚本格式（.bat/.sh）与内容中的内存值 | platform.rs 生成；内存取用户配置 `max_memory_mb`，禁止脚本内再写死 `-Xmx4096M` |
| 访问令牌的存储位置 | 禁止存入前端 localStorage（现状 stores/app.ts 把含 access_token 的 Profile 明文存入 localStorage，须改）；改由 Rust 侧保存至 app_config_dir 下的凭据文件，发布级可再升级系统钥匙串 |
| 前端任何平台默认值（现状 app.ts 内置 D 盘路径） | 前端启动时向后端请求默认值（新增/复用 Tauri 命令返回 platform 默认值），前端代码零平台字面量 |

例外登记：确实只能写死的字面量（如微软登录的公开 Client ID `00000000402b5328`、API 域名）必须集中定义为命名常量，并在交付报告的「例外清单」中逐条列明理由。未登记者一律按违规处理。

---

## 3. macOS 专项改造（按文件点名）

### 3.1 启动规则与 natives —— 最关键，不做完不算支持 macOS
现状（已核实）：`launch.rs` 的 `check_rules` 把 os.name 与 `"windows"` 比较，非 Windows 库一律排除；`LaunchRule` 只有 os.name/arch 两个字段，缺 os.version 与 features；`LaunchLibraryDownloads` **只有 artifact、根本没有 classifiers 字段**，natives 罐从未被下载与解压。

要求：
1. 按 Mojang 官方 rules 语义重写判定：无 rules → 允许；有 rules → 从上到下逐条匹配，**最后一条匹配的规则决定** allow/disallow；匹配条件含 os.name、os.arch、os.version（正则）、features（现状用不到的 features 可先支持 is_demo_user/has_custom_resolution 两项并留扩展位）。
2. 判定输入为 (mc_os_name, mc_os_arch) 参数（§1 的可测试性要求）；删除任何与 `"windows"` 字面量的直接比较。
3. 给 `LaunchLibraryDownloads` 补 `classifiers` 字段；安装时按版本 JSON 给出的 classifier 名**查表取用**（如 `natives-macos`、`natives-macos-arm64`），禁止自己拼 classifier 字符串；启动前把选中的 natives 罐解压到该版本的 natives 目录（剔除 META-INF 与签名文件），并设置 `-Djava.library.path` 指向它。
4. JVM/game 参数中的 `${arch}`、`${natives_directory}`、`${classpath}` 等占位符按当前平台实值替换；旧版 `minecraftArguments` 字符串路径同样处理。参数里出现的 `-Dos.name=...` 等由版本 JSON 规则选出的条目不得被覆盖为 Windows 值。

### 3.2 默认目录（config.rs ＋ platform.rs）
- macOS 默认游戏目录：`~/Library/Application Support/minecraft`（经 home_dir 推导，禁止拼字符串写死 `/Users/...`）。
- Windows 改为经 `%APPDATA%` 推导的 `.minecraft`，Linux 为 `~/.minecraft`。**删除一切盘符路径默认值。**
- 仓库内的 `src-tauri/pcl_config.json`（含作者本机 D 盘路径）从仓库移除并加入 .gitignore；改提供 `pcl_config.example.json`，字段用平台中立的占位说明。

### 3.3 Java 发现（java.rs）
在现有 PATH、JAVA_HOME 基础上，macOS 分支补齐：
- `/usr/libexec/java_home -V` 的输出解析（系统注册的全部 JDK）；
- `/Library/Java/JavaVirtualMachines/*/Contents/Home` 与 `~/Library/Java/JavaVirtualMachines/*`（注意 .jdk 包内是 `Contents/Home/bin/java` 结构）；
- Homebrew：`/opt/homebrew/opt/`（Apple Silicon）与 `/usr/local/opt/`（Intel）下的 openjdk*；
- 官方启动器 runtime：`<游戏目录>/runtime` 下各组件目录。
- 64 位/架构判定扩展：`java -version` 输出含 `aarch64`/`arm64` 时记为 arm64，禁止只认 `x86_64`/`amd64`（现状会把 M 芯片 Java 误判为 32 位）。
目录清单只许出现在 platform.rs 的 `java_search_dirs()`；java.rs 只负责遍历与解析。

### 3.4 打开文件夹 / 打开链接（resource.rs、src-tauri/src/lib.rs）
现状：`open_folder` 只有 Windows 分支，非 Windows 静默无操作；`open_url` 非 Windows 误用 Linux 的 `xdg-open`，macOS 上不存在该命令。
要求：两者改调 platform.rs 实现（macOS 用 `open`），失败必须向前端返回错误，禁止静默吞掉。

### 3.5 服务端脚本（installer.rs）
现状：只生成 `启动服务端.bat`。
要求：改调 platform.rs `server_script()`：Windows 生成 .bat；macOS/Linux 生成同名语义的 `.sh`（`#!/bin/sh`、java 命令行与现有参数一致、内存取用户配置）并 `chmod 0o755`。前端 DownloadView 中提示文案里的「启动服务端.bat」改为调用后端返回的脚本文件名，禁止前端写死扩展名。

### 3.6 窗口与打包（src-tauri/tauri.conf.json 及窗口创建代码）
- macOS 窗口保留原生红绿灯：用 Tauri 的 `titleBarStyle: Overlay`＋透明标题栏方案，或在 mac 上恢复 `decorations: true`；现状 `decorations: false`＋右上角自绘关闭/最小化按钮是 Windows 习惯，mac 上不许照搬（自绘按钮在 mac 分支隐藏或移位）。平台差异若静态 JSON 配置无法表达，则在 Rust 窗口创建代码中按 cfg 处理——这是 platform 层之外的**唯一**允许出现 cfg 的位置，且须在报告中登记。
- 打包目标：分别产出 `aarch64-apple-darwin` 与 `x86_64-apple-darwin`，或 universal 包；在 README 写明构建机必须是 macOS。
- 签名/公证：代码签名与 notarization 属发布流程，不要求在本轮代码内实现，但交付报告必须明说「未签名构建在他人 Mac 上会被 Gatekeeper 拦截」，不许宣称可直接分发。

### 3.7 前端（src/）
- 删除 stores/app.ts 等一切前端文件中的平台字面量与本机路径；默认值一律来自后端。
- 「在文件夹中打开」「启动服务端.bat」等文案改为语义文案或后端返回值。
- vue-router 已安装未启用、HelloWorld.vue 等模板残留与本规范无关，本轮**不要顺手重构**（见第 7 节范围）。

---

## 4. 下载与安装的平台中立性

- installer 下载普通库时同样必须过 rules 判定（现状按 artifact 无条件下载）：不适用于当前平台的库不下载、不进 classpath。
- 文件名大小写：macOS 文件系统默认大小写不敏感但可敏感，路径比较与去重禁止假设大小写规则与 Windows 相同；classpath 去重逻辑保持现有 package-key 方案即可，但 key 生成不得依赖平台分隔符。
- 解压 zip（mods/resources/natives 通用）：条目路径一律以 `Path` 组件方式落盘，防止 zip 内 `../` 或绝对路径逃逸（zip-slip），三平台同等生效。

## 5. 验收标准（全部满足才许报告「支持 macOS」）

1. **自查清零**：在仓库根目录运行并把输出贴进交付报告——
   - `grep -rn '"windows"' crates src-tauri/src src --include='*.rs' --include='*.ts' --include='*.vue'` 除 platform.rs 与测试外零命中；
   - `grep -rn 'D:\\\\|javaw\|xdg-open\|explorer\|taskkill' crates src-tauri/src src` 除 platform.rs 外零命中；
   - `grep -rn 'bmclapi2' crates` 只许出现在 endpoints 常量模块。
2. **规则单元测试**：新增 Rust 单元测试，同一份版本 JSON 样本分别以 (windows,x64)、(osx,arm64)、(osx,x64)、(linux,x64) 为参数跑 rules 判定与 classifier 选择，断言每组选出的库集合与 Mojang 官方启动器行为一致；测试在任意操作系统上可运行（靠 §1 的参数注入）。
3. **Windows 回归**：原有 Windows 主链路行为不变，上述测试的 windows 向量即回归证据；`cargo check` 与前端 `npm run build` 在 Windows 与 macOS 各过一次（在哪台机器上验的要写明）。
4. **mac 真机/构建机冒烟**（需在 macOS 上执行，无 Mac 时此项标「未验」，不许用推测顶替）：扫描到本机 Java（含一台 Apple Silicon 机上的 arm64 JDK）、下载并安装一个原版版本＋一个 Fabric 版本、成功启动到主菜单、打开文件夹与链接可用、服务端 .sh 可执行。
5. 交付报告按第 6 节格式提交，含例外清单与未验项。

## 6. 交付报告格式（执行方必须照此汇报）

- 改动文件清单：逐文件一句话说明改了什么、对应本规范哪一条；
- 第 5 节第 1 项三条 grep 的实际输出；
- 新增/通过的测试名单与运行环境（操作系统、架构、命令）；
- 例外清单：仍保留的字面量及理由（无则写「无」）；
- 未验项：没实际验证过的条目逐条列出（尤其是无 Mac 时的冒烟项），**禁止把代码审读或推测表述为已验证**。

## 7. 范围外（本轮不要做）

- 不补 Forge/NeoForge/OptiFine 的安装逻辑（另行立项）；
- 不做联机页、不启用 vue-router、不做 UI 改版；
- 不迁移凭据到系统钥匙串（本轮只要求移出 localStorage）；
- 不改微软登录流程本身（其 Client ID 按 §2 例外登记处理）。

---

## 8. 注释与代码整洁规范（长期规矩）

1. **只写为什么与约束，不写是什么**：函数名、变量名或模板标签已经说清的，禁止在上方写中文复述型注释；
2. **禁止横幅与叙事**：禁止新增 `====` 分隔横幅、设计出处叙事（如「对齐原版 PCL」等统一只在 README 总述一句）及外部文档条款号引用（如 `§x.x`）；
3. **公共 API 契约化**：新增公共 API 的 `///` 文档注释只写契约（参数语义、返回值、失败模式、平台差异与踩坑说明），不写实现过程；
4. **保留关键资产**：涉及安全边界（防 zip-slip）、权限约束（如凭据 0600）、平台特殊兼容（如 `-XstartOnFirstThread`）的注释必须永久保留。

---

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
