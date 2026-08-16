# S6 机器证明门

- 日期：2026-08-16
- 切片：COGNITIVE-I1-S6（扫描门；真机 §7 / §8 未做）
- 分支：`agent-diva-pro`（未 push）
- 版本目录：`docs/logs/2026-08-cognitive-s6-proof/v0.0.1-scan-gate/`

## 目标

把 D4 §3.1「生产路径零命中」落成可重复的 `just cognitive-clean-break-check`，并挂进 `just ci`。不替代 `just laputa-clean-break-check`。

## 提交

| Commit | 内容 |
| --- | --- |
| `2e58a022` | `scripts/ci/check_cognitive_clean_break.py` + just 配方/CI；巩固注释去掉已删符号名 |

## 未做

- D4 §7 真机八条（本环境无原生 WebView 控制器）
- D4 §8 保护分支恢复演练
- `put_governed` schema 清理
