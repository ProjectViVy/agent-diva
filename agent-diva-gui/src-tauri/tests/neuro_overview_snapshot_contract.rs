//! Story 3.2：`get_neuro_overview_snapshot` 使用的快照与 `CortexRuntime` 同源，且皮层关时为 `degraded`。

use agent_diva_swarm::{
    build_neuro_overview_snapshot_v0, CortexRuntime, NeuroDataPhase,
};

#[test]
fn neuro_snapshot_matches_cortex_degraded_when_off() {
    let rt = std::sync::Arc::new(CortexRuntime::new());
    rt.set_enabled(false);
    let cortex = rt.snapshot();
    let snap = build_neuro_overview_snapshot_v0(cortex.clone(), &[]);
    assert_eq!(snap.data_phase, NeuroDataPhase::Degraded);
    assert_eq!(snap.cortex, cortex);
    assert!(snap.left_rows.is_empty());
}

#[test]
fn neuro_snapshot_stub_when_cortex_on_and_no_events() {
    let rt = std::sync::Arc::new(CortexRuntime::new());
    assert!(rt.snapshot().enabled);
    let snap = build_neuro_overview_snapshot_v0(rt.snapshot(), &[]);
    assert_eq!(snap.data_phase, NeuroDataPhase::Stub);
}
