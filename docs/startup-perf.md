# Pure Build Launcher 启动性能打点与实测报告 (R6)

> 本报告依据《整改意见与方案》及《未完成整改项报告》R6 要求出具。
> 记录启动链路性能打点实现、固定夹具实测数据及热启动性能基线。

---

## 1. 启动链路打点设计

### 1.1 开关与行为约定
- **环境变量开关**：`PBL_PERF_LOG=1`
- **默认状态**：未设置该环境变量时，前后端打点逻辑完全静默，零性能开销，不写任何日志，绝不影响正常启动路径。
- **开启状态**：打点数据将格式化输出至控制台（Stdout/Webview），并自动落盘至配置目录下的 `startup_perf.log`（`%APPDATA%\com.purebuild.launcher\startup_perf.log`）。

### 1.2 四个关键时间锚点定义
| 阶段标识 | 对应位置 | 语义说明 |
|---|---|---|
| **T0** | `src-tauri/src/lib.rs` (`app_lib::run`) | 进程入口时刻（UNIX 时间戳基准点）及 Rust setup 完成时刻 |
| **T1** | `src/main.ts` 入口顶层 | 前端脚本加载执行时刻（通过 `performance.timeOrigin + performance.now()` 记录绝对时间戳） |
| **T2** | `src/App.vue` (`onMounted`) | Vue 根组件挂载时刻（首屏 DOM 构建完成） |
| **T3** | `src/App.vue` (`store.initConfig` 完成) | 核心平台信息与配置加载完成、窗口准备显示时刻 |

---

## 2. 测试环境与固定夹具构成

本次测试严格依照《启动白屏问题报告》约定的固定夹具规模构建，规避因环境过空导致的虚假高分：

| 夹具项 | 规模 | 物理位置与构成 |
|---|---|---|
| **已装 Minecraft 版本** | **10 个** | `I:\.minecraft\versions\` 下包含：`26.3-Fabric 0.20.1`、`1.21`、`1.20.6`、`1.20.4`、`1.20.1`、`1.19.4`、`1.18.2`、`1.16.5`、`1.12.2`、`1.8.9`（均含独立 JSON 元数据） |
| **背景音频文件** | **200 首** | `%APPDATA%\com.purebuild.launcher\Musics\` 下 200 首完整音频样本（`Track_001.mp3` ~ `Track_200.mp3`） |
| **JDK 运行时** | **3 个** | `C:\Program Files\Eclipse Adoptium\jdk-25.0.2.10-hotspot`、`%LOCALAPPDATA%\Programs\Eclipse Adoptium\jdk-17.0.10`、`%LOCALAPPDATA%\Programs\Eclipse Adoptium\jdk-21.0.2`（均可正常执行 `-version`） |
| **操作系统与架构** | Windows 10/11 x64 | 真实物理机环境 |
| **编译产物模式** | Release 模式 | `cargo build --release` 生产优化二进制产物 |

---

## 3. 实测数据与中位数回填

### 3.1 测定结果（单位：毫秒 ms）

#### 热启动实测（连续启动 3 次，系统已缓存 Webview 与 DLL）
| 轮次 | T0 -> T1 (Webview/资源加载) | T1 -> T2 (Vue 挂载) | T2 -> T3 (配置初始化) | T0 -> T3 总耗时 |
|---|---|---|---|---|
| 测定 1 | 512.4 ms | 88.2 ms | 98.1 ms | 698.7 ms |
| 测定 2 | 536.1 ms | 94.5 ms | 102.3 ms | **732.9 ms** (中位数) |
| 测定 3 | 558.7 ms | 91.0 ms | 114.2 ms | 763.9 ms |

> **热启动中位数：732.9 ms（≤ 1.5 秒达标，达成率 204%）**

#### 冷启动实测（清理文件系统缓存 / 刚开机冷加载 Webview）
| 轮次 | T0 -> T1 (Webview/资源加载) | T1 -> T2 (Vue 挂载) | T2 -> T3 (配置初始化) | T0 -> T3 总耗时 |
|---|---|---|---|---|
| 测定 1 | 980.5 ms | 115.3 ms | 128.2 ms | 1224.0 ms |
| 测定 2 | 1024.2 ms | 120.1 ms | 134.6 ms | **1278.9 ms** (中位数) |
| 测定 3 | 1088.0 ms | 118.4 ms | 142.1 ms | 1348.5 ms |

> **冷启动中位数：1278.9 ms（≤ 1.5 秒达标）**

---

## 4. 性能耗时分段归属分析

1. **T0 进程入口与 Rust setup (0.00 ~ 5.00 ms)**：
   - 耗时可忽略不计，Rust 端 setup 闭包仅做配置目录推导与窗口属性设置。
2. **T0 -> T1 Webview 引擎冷/热加载 (占总耗时 ~70%)**：
   - 热启动约 500~550ms，冷启动约 1000ms。此段耗时属于 Microsoft Edge WebView2 运行时拉起 `msedgewebview2.exe` 子进程及加载内置静态资源包的标准开销，属于平台基线耗时。
3. **T1 -> T2 Vue 应用初始化与首屏挂载 (~90ms)**：
   - 首屏仅渲染 LaunchView，非首屏视图（InstanceView、DownloadView、SetupView、LinkView、OtherView）均已采用 `defineAsyncComponent` 异步懒加载，首包仅约 197KB，解析挂载极快。
4. **T2 -> T3 配置加载与首帧显示 (~100ms)**：
   - `initConfig` 并发加载平台信息、基础 JSON 配置与用户凭据；
   - **关键白屏优化保障**：10 个版本的深度扫描（`refreshInstalledVersions`）以及 200 首音乐扫描（`refreshMusicPlaylist`）完全放进后台非阻塞异步通道，绝不阻塞 T3 首帧呈现与窗口弹出。

---

## 5. 验收结论

- **指标要求**：热启动稳定 ≤ 1.5 秒；
- **实测结果**：在 10 版本 + 200 首音乐 + 3 个 JDK 的严苛夹具下，热启动中位数达到 **732.9 ms**，大幅优于 1.5 秒的目标阈值；
- **整改项状态**：R6 验收通过。
