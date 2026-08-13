# Verification — COGNITIVE-R1 v0.0.1

## 命令与检查

| 检查 | 结果 |
| --- | --- |
| GA `git rev-parse HEAD` | `ee5a474e5a1b6d203438d7d1dfa21da912268bb8` |
| GA `skills/` 存在 | False |
| GA `mykey.py` 存在 | False |
| GA `*_sop.md` 计数 | 22 |
| 研究包 10 文件落盘 | Pass（目录 listing） |
| 源码交叉：`do_start_long_term_update` / L0 公理 / scheduler L4 | Pass（主会话复读） |
| 子代理 7/7 完成 | Pass |
| `just fmt-check/check/test` | **未跑**（纯 docs，无 Rust 变更） |
| 活体 LLM 实验 | **阻断**（无密钥） |

## 证据分级遵守

各文档标注 源码事实 / 提交事实 / 实验观察 / 推断 / 建议。

## 已知限制

1. 本地 `git fetch` 曾 SSL 失败；远程 tip 经 GitHub API 记录，非本机 fetch 合并。  
2. P1/P2 未测量模型遵守率。  
3. 全量 R0 未完成；仅 Evolution 切片。  
