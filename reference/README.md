# TS 参照基线（reference）

本目录记录 Rust 重写所对拍的 **TypeScript 原实现** 的冻结版本，对应
[`05-migration-and-verification.md`](../Plan/05-migration-and-verification.md) §2「参照物管理」。

**不是逐行翻译，而是按功能边界重写**；TS 版只作为行为参照（oracle）。

## 冻结版本

| 项 | 值 |
|---|---|
| Git tag | `ts-baseline-2.8.4` |
| Commit | `77a7934e15d69da13879112ed7db695c9ee7a52a` |
| 上游版本 | `2.8.4` |
| 运行时 | Bun `>= 1.3.0` |

该 tag 指向 Rust 脚手架提交（`ca75ede8`, T0.1）的父提交，因此在本仓库历史中可达，
且不会被后续重写影响。

## 如何取得参照源码

任选其一：

```bash
# A. 用 worktree 检出到仓库外，避免污染当前工作树
git worktree add ../ccb-ts-reference ts-baseline-2.8.4

# B. 只读导出
git archive --format=tar ts-baseline-2.8.4 | tar -x -C /path/to/ccb-ts-reference

# C. 本地已有的副本（通过 .git/info/exclude 排除，不参与 Rust 工作树）
ls claude-code-nodejs/claude-code-rust
```

本地副本与 tag 同为 `77a7934e`，仅用于查阅，**不再修改**。

## 用途

1. **请求体对拍**：同一输入、同一 `base_url` + key，分别运行 TS 版与 Rust 版，比较
   system prompt、tools 定义、消息序列。
2. **流事件对拍**：比较 SSE 事件序列、最终输出、工具调用。
3. **测试移植**：把核心测试逐项翻译为 `cargo test`（见 T10.1）。
4. **golden transcript**：录制真实会话作为验收基线（见 T10.2）。

## 许可说明

参照源码是逆向工程产物，仓库中未附带许可证，仅作本地行为比对；Rust 重写后的
代码由本仓库以 MIT 许可发布（见根目录 `LICENSE`）。
