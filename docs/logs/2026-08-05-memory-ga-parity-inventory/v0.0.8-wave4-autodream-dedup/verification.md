# Verification — GA-MEM-PARITY Wave 4（AutoDream 去重 — G4）

## 方法

每切片独立：`cargo fmt --all -- --check`、`cargo clippy -p <crate> --all-targets -- -D warnings`、
`cargo test -p <crate>`；S2 后全 workspace `cargo test --workspace`。

## 结果

| 切片 | 提交 | 验证 |
|------|------|------|
| S1 | `f042bba4` | laputa 38 lib + 9 integration 全绿（含新增 wave4_tests × 3）；clippy 干净 |
| S2 | `a39638bb` | autodream 14 lib + 6 integration suite（candidates/curation/inputs/outputs/reports/service/worker）全绿（含新增 wave4_tests × 3）；clippy 干净 |
| S3 | 本次 | docs only，`git diff --check` |

全 workspace 回归：core / agent 386+15 / laputa 38+9 / autodream 14+6 suites /
tools 132 / manager / migration 全绿。CLI 6 个既有 wiremock 502 失败
（`CLI-WIREMOCK-502-PREEXISTING`，TODOLIST 已记录，与本迭代无关）。

## 验收测试落点（G4 双写边界）

- S1（`applied_authority_digests` 服务方法契约）：
  - `applied_authority_digests_returns_only_active_authority`：四重过滤
    （AppliedAuthority / 无 tombstone / 无 session scope / 非 supersedes 目标）
    全部成立；长度精确为 1。
  - `applied_authority_digests_excludes_superseded_targets`：写 tombstone
    前后 digest 集合差异——supersedes 目标被正确剔除。
  - `applied_authority_digests_graceful_missing_store`：空 workspace
    首次 run → 返回 `Ok([])`，不报错。
- S2（端到端去重契约）：
  - `candidate_duplicate_against_typed_authority_is_rejected`：typed
    authority 已存记录 → AutoDream gate 拒绝同内容候选，
    `CandidateRejectionCode::Duplicate`。
  - `candidate_fresh_against_typed_authority_is_accepted`：不同内容候选
    正常通过 gate。
  - `superseded_authority_record_no_longer_blocks_duplicate_candidate`：
    supersedes tombstone 落库后，原内容 digest 消失 → 同内容候选可重新被
    接受（避免"误删一次、永久失去"反模式）。

## 遗留

- Wave 4 延期项（G1/G2/G3/G5/G6/G7/G10/G11/G12）均为真机/产品级验收，
  归 G2D+ 桌面验收或后续独立 Wave。
- 同会话热注入（F4，Wave 3 已条目化）不在本 Wave 范围。
- CLI wiremock 502 排查（独立 TODO，`CLI-WIREMOCK-502-PREEXISTING`）。
- `agent-diva-files` clippy clean（`s3.rs:247 empty_line_after_doc_comments`
  预存在，独立 sev-P3 TODO）。
