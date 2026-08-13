# Release

## Method

本轮为开发分支内的本地代码交付，不涉及独立部署步骤。

## Preconditions

- `agent-diva-autodream` crate 测试与编译通过。
- 下游 Story 3.3 可直接消费 `collect_inputs` 写回的 run record `input_summary`。

## Rollback

- 回滚本次 commit 即可恢复到输入采集能力引入前的状态。
