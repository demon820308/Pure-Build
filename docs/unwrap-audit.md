# unwrap 审计与分类留档 (unwrap Classification Ledger)

> **版本基准**：`v0.1.5`（Git HEAD `ca105dd` 及后续收尾）  
> **审计范围**：`crates/pcl-base/src/installer/` 与 `crates/pcl-base/src/launch/` 模块全量代码

---

## 1. 统计口径与分类定义

### 1.1 统计口径
- **仅计入生产代码**：严格排除各源码文件中 `#[cfg(test)]` 标记及其后随的测试模块与单元测试函数；
- **全词精确匹配**：以 `.unwrap()` 方法调用作为检索基准。

### 1.2 分类定义
- **一类（内部不变量，允许保留）**：
  - 互斥锁（`Mutex::lock().unwrap()`）：在锁未中毒（PoisonError）时的内部并发控制；若互斥锁中毒，表明持有锁的并发线程已发生非正常恐慌，此时内存状态已处于脏状态或不可信状态，快速失败（panic）属于安全且可接受策略；
  - 进程启动前已确立的不变量（常量、内部初始化的静态资源等）。
- **二类（外部输入路径，严禁使用，必须归零）**：
  - 包括但不限于：外部 JSON 反序列化与字段提取、用户配置与本地文件读取、HTTP 网络请求与响应解析、路径拼接与解压条目处理；
  - 任何此类路径均必须采用 `match`、`if let` 或 `?` 运算符向上传播结构化 `Result` 错误，严禁抛出 panic。

---

## 2. 生产代码 unwrap 存量明细表（共 8 处）

经复查，`installer/` 与 `launch/` 模块非测试代码中共有 **8 处** `.unwrap()` 调用，全部归属于**一类（内部不变量）**，二类 unwrap 存量为 **0**。

| 序号 | 所在文件 | 代码行号 | 源码片段 | 分类 | 保留理由 |
|:---:|---|:---:|---|:---:|---|
| 1 | `crates/pcl-base/src/installer/download.rs` | 431 | `fail_list.lock().unwrap().push(err_msg);` | 一类 | 收集多线程并行下载失败错误；`Mutex::lock()` 内部中毒判定，状态损坏时 panic 为安全边界。 |
| 2 | `crates/pcl-base/src/installer/download.rs` | 444 | `fail_list.lock().unwrap().push(...)` | 一类 | 收集 Natives 解压失败信息；同上，属于并发聚合锁。 |
| 3 | `crates/pcl-base/src/installer/download.rs` | 469 | `failures.lock().unwrap().push(...)` | 一类 | 收集普通并发任务异常；互斥锁安全获取。 |
| 4 | `crates/pcl-base/src/installer/download.rs` | 474 | `let errs = failures.lock().unwrap().clone();` | 一类 | 汇总并发下载任务的失败清单并转为顶层错误返回；互斥锁安全获取。 |
| 5 | `crates/pcl-base/src/installer/verify.rs` | 47 | `let mut map = map_mutex.lock().unwrap();` | 一类 | 全局文件锁状态映射表的并发访问互斥锁保护。 |
| 6 | `crates/pcl-base/src/launch/process.rs` | 141 | `let mut buf = buf_clone.lock().unwrap();` | 一类 | 游戏子进程 stdout 日志环形缓冲区的并发写入互斥锁保护。 |
| 7 | `crates/pcl-base/src/launch/process.rs` | 155 | `let mut buf = buf_clone.lock().unwrap();` | 一类 | 游戏子进程 stderr 日志环形缓冲区的并发写入互斥锁保护。 |
| 8 | `crates/pcl-base/src/launch/process.rs` | 169 | `let buf = log_buffer.lock().unwrap();` | 一类 | 游戏崩溃或退出时提取最近 100 行日志上下文的互斥锁读取。 |

---

## 3. 审计结论

- **一类存量**：8 处（全部为标准库 `Mutex::lock()`）；
- **二类存量**：**0 处**；
- 外部输入与解析路径（含 Fabric/Quilt/Forge 版本合并、JSON 参数解析、ZIP 校验）在前期重构中已全面替换为模式匹配与错误传播机制。
