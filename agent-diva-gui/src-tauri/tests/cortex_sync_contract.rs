//! 无 GUI：验证与 Tauri 托管相同的 `Arc<CortexRuntime>` 在跨线程写入后查询一致（Story 1.3 / FR14）。

use agent_diva_swarm::{CortexRuntime, CORTEX_STATE_SCHEMA_VERSION_V0};
use std::sync::Arc;

#[test]
fn shared_runtime_set_query_matches_after_cross_thread_write() {
    let rt = Arc::new(CortexRuntime::new());
    assert!(rt.snapshot().enabled);

    let peer = Arc::clone(&rt);
    std::thread::spawn(move || {
        peer.set_enabled(false);
    })
    .join()
    .expect("thread join");

    let snap = rt.snapshot();
    assert!(!snap.enabled);
    assert_eq!(snap.schema_version, CORTEX_STATE_SCHEMA_VERSION_V0);
}
