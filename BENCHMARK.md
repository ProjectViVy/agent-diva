# Agent Diva Performance Benchmarks

Criterion-based microbenchmarks for `agent-diva-core` hot paths. These measure
CPU-bound operations and simulated concurrency workloads, not end-to-end latency
over real networks.

**Run all benchmarks:**

```bash
cargo bench -p agent-diva-core
```

**Run a single benchmark group:**

```bash
cargo bench -p agent-diva-core -- api_latency
cargo bench -p agent-diva-core -- cli_latency
cargo bench -p agent-diva-core -- channels_100
```

Baseline results are machine-dependent and should be treated as relative
comparison points within the same hardware, not absolute performance guarantees.

---

## 1. API Latency

**Benchmark group:** `bench_api_latency`
**Source:** [`agent-diva-core/benches/performance.rs`](agent-diva-core/benches/performance.rs#L54)

### `api_latency_parse_and_respond`

Simulates a JSON API request/response cycle: parse a chat-completion request
body (with messages, model, temperature, max_tokens fields), build a
structured JSON response with usage stats, and serialize it back to string.

- **What it measures:** serde_json deserialization + JSON construction +
  serialization cost for a typical API payload.
- **Input size:** ~200 bytes JSON request body.
- **Operations per iteration:** 1x `serde_json::from_str`, 1x `serde_json::json!`
  macro invocation, 1x `serde_json::to_string`.

### `api_latency_simulated_roundtrip`

Simulates a minimal HTTP request/response flow: parse request headers from raw
bytes, extract method and path, build an HTTP response status line with JSON body.

- **What it measures:** String parsing and formatting overhead for a basic HTTP
  roundtrip (DNS/TLS/network I/O are excluded, this is CPU-bound parsing only).
- **Input size:** ~60 bytes raw HTTP request bytes.

---

## 2. CLI Latency

**Benchmark group:** `bench_cli_latency`
**Source:** [`agent-diva-core/benches/performance.rs`](agent-diva-core/benches/performance.rs#L111)

### `cli_latency_arg_parsing`

Simulates CLI argument parsing without clap: walks a 9-element string slice
(`--config`, `agent`, `--message`, `--model`, `--verbose`) and extracts
structured values (config_path, message, model, verbose flag).

- **What it measures:** Manual string-matching arg parsing overhead.
- **Args count:** 9 elements (1 binary name + 8 arguments).

### `cli_latency_output_formatting`

Simulates CLI response display formatting: constructs a formatted output string
containing a multi-line response body, model name, and token counts.

- **What it measures:** `format!` macro overhead for typical CLI output
  (~170 bytes input, produces formatted string with separators).
- **Output size:** ~250 bytes formatted string.

---

## 3. GUI Load

**Benchmark group:** `bench_gui_load`
**Source:** [`agent-diva-core/benches/performance.rs`](agent-diva-core/benches/performance.rs#L182)

### `gui_load_widget_tree_build`

Simulates recursive widget tree construction: builds a 4-level-deep tree where
each node spawns 4 children. This represents a chat UI panel hierarchy
(message list, input area, sidebar, toolbar, etc.).

- **What it measures:** Recursive struct allocation and vector construction cost.
- **Tree depth:** 4 levels.
- **Branching factor:** 4 children per node. Total nodes: (5^4 - 1)/4 = 341 widgets.

### `gui_load_layout_calculation`

Simulates flex-like layout bounding-box calculation: iterates 100 rectangle
items, computes each item's right/bottom edges, and tracks the maximum extent.

- **What it measures:** Loop iteration + integer arithmetic over 100 layout items.
- **Item count:** 100 rectangles with varying positions and sizes.

---

## 4. 100 Concurrent Channels

**Benchmark group:** `bench_channels_100`
**Source:** [`agent-diva-core/benches/performance.rs`](agent-diva-core/benches/performance.rs#L253)

### `channels_100_concurrent`

Spawns 100 async tasks via `tokio::spawn`, each simulating a channel connection
lifecycle: generate a channel ID, format a timestamped message, and produce an
ACK response. All 100 tasks run concurrently and are joined before the iteration
completes.

- **What it measures:** Tokio task spawn overhead + concurrent string formatting
  at scale (100 simultaneous "channels").
- **Concurrency:** 100 tokio tasks, all awaited before completion.
- **Per-task work:** String formatting (`chrono::Utc::now()`, `format!`), one
  boolean check, one conditional branch.

---

## 5. 50 Background Tasks

**Benchmark group:** `bench_background_tasks_50`
**Source:** [`agent-diva-core/benches/performance.rs`](agent-diva-core/benches/performance.rs#L288)

### `background_tasks_50_concurrent`

Spawns 50 async tasks via `tokio::spawn`, each performing lightweight CPU work:
string formatting, a checksum-like byte-level hash computation (iterate over
payload bytes, multiply by position, fold), and a status branch.

- **What it measures:** Combined async spawn + CPU-bound per-task processing
  overhead for a moderate background workload.
- **Concurrency:** 50 tokio tasks.
- **Per-task work:** ~30 byte payload formatting, ~30-iteration byte folding
  with `wrapping_mul`/`wrapping_add`, one modulo branch.

---

## 6. 10 Concurrent Workspace Operations

**Benchmark group:** `bench_workspace_ops_10`
**Source:** [`agent-diva-core/benches/performance.rs`](agent-diva-core/benches/performance.rs#L327)

### `workspace_ops_10_concurrent_file_rw`

Spawns 10 blocking tasks (via `tokio::task::spawn_blocking`) in a temp
directory, each performing: write a ~150-byte text file to disk, read it back,
parse the first line, and return metadata.

- **What it measures:** Filesystem I/O concurrency under blocking task pool
  (write + read + parse for 10 concurrent files).
- **Concurrency:** 10 `spawn_blocking` tasks.
- **Per-task work:** 1x file write (~150 bytes), 1x file read (~150 bytes),
  string line splitting.

> Note: This benchmark exercises the tokio blocking thread pool and real disk
> I/O. Results vary significantly across filesystem types (SSD vs HDD, NTFS vs
> ext4 vs APFS) and operating systems.

---

## Additional: Security Benchmarks

Two pre-existing security benchmarks are also included in the same suite:

| Benchmark | Group | Description |
|-----------|-------|-------------|
| `pii_redact_safe_small` | `bench_pii_redaction` | PII redaction on clean input (no PII present) |
| `pii_redact_large_with_pii` | `bench_pii_redaction` | PII redaction on 50KB text with embedded email + API key patterns |
| `injection_detect_clean` | `bench_injection_detection` | Injection detection on benign user message |
| `injection_detect_attack` | `bench_injection_detection` | Injection detection on DAN-style prompt injection attack |

---

## Running with Baseline Comparison

Criterion saves results to `target/criterion/`. To compare against a saved
baseline:

```bash
# Save current results as baseline
cargo bench -p agent-diva-core -- --save-baseline master

# After changes, compare against the saved baseline
cargo bench -p agent-diva-core -- --baseline master
```

## Requirements

- Rust 1.80.0+ (workspace MSRV)
- `criterion` 0.5 with `async_tokio` feature (declared in `agent-diva-core` dev-dependencies)
- Benchmarks using `spawn_blocking` (Group 6) need a writable temp directory
