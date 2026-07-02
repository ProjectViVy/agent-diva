# agent-diva 黑盒测试清单

> 版本: 0.2.0 | 最后更新: 2026-02-14
>
> 本文档为人工黑盒测试清单，覆盖所有用户可感知的功能。
> 测试人员无需阅读源码，仅需按照步骤操作并验证结果。

---

## 目录

1. [环境准备与构建](#1-环境准备与构建)
2. [CLI 命令测试](#2-cli-命令测试)
3. [配置系统测试](#3-配置系统测试)
4. [聊天通道测试](#4-聊天通道测试)
5. [AI 供应商测试](#5-ai-供应商测试)
6. [工具系统测试](#6-工具系统测试)
7. [Agent 对话流程测试](#7-agent-对话流程测试)
8. [会话与记忆测试](#8-会话与记忆测试)
9. [定时任务 (Cron) 测试](#9-定时任务-cron-测试)
10. [TUI 交互测试](#10-tui-交互测试)
11. [安全性测试](#11-安全性测试)
12. [错误处理与恢复测试](#12-错误处理与恢复测试)
13. [性能与边界测试](#13-性能与边界测试)
14. [多通道联合测试](#14-多通道联合测试)

---

## 1. 环境准备与构建

### 1.1 前置条件

| 依赖 | 最低版本 | 安装命令 |
|------|---------|---------|
| Rust | 1.70+ | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| Just | 最新 | `cargo install just` |
| Node.js | 20+ | 仅 WhatsApp bridge 需要 |

### 1.2 构建

```bash
cd agent-diva

# Debug 构建
cargo build --all

# Release 构建
cargo build --all --release

# 安装到本地
cargo install --path agent-diva-cli
```

### 1.3 构建验证

- [ ] `cargo build --all` 无错误完成
- [ ] `cargo clippy --all -- -D warnings` 无警告
- [ ] `cargo fmt --all -- --check` 格式检查通过
- [ ] `cargo test --all` 所有单元测试通过
- [ ] `just ci` 一键 CI 检查全部通过
- [ ] Release 构建产物为单一二进制文件

### 1.4 快速验证

```bash
# 验证二进制可执行
agent-diva --help
agent-diva --version
```

- [ ] `--help` 输出命令列表
- [ ] `--version` 输出 `0.2.0`
