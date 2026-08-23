# Verification

- `just --list`：出现 `msrv-probe *ARGS`。
- `just msrv-probe --version`：通过，输出 `cargo 1.80.1`；配方先设置
  `$env:CARGO_TARGET_DIR = "target/msrv-1.80"`。
- 未执行 `cargo +1.80 check --workspace`（依赖 MSRV 冲突另条）。
