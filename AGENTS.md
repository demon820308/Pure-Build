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

