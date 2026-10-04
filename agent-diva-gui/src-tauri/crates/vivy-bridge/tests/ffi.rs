//! Bridge acceptance tests. `fake_*` cases run against a deterministic C
//! stand-in (`fake_vivy.c`) that injects failure modes the sealed library
//! cannot produce on demand; `real_*` cases run the genuine packed artifact
//! when `DIVA_VIVY_RUNTIME` + `DIVA_VIVY_CONFIG` point at one.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::AtomicBool;
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

use serde_json::json;
use vivy_bridge::{
    spawn_event_pump, BridgeEvent, ErrorKind, Host, InitConfig, VivyLibrary,
};

/// Serializes access to the fake library's process-global state.
static FAKE_LOCK: Mutex<()> = Mutex::new(());

#[cfg(all(unix, not(target_os = "macos")))]
fn shared_name() -> &'static str {
    "vivy-shared.so"
}
#[cfg(target_os = "macos")]
fn shared_name() -> &'static str {
    "vivy-shared.dylib"
}
#[cfg(target_os = "windows")]
fn shared_name() -> &'static str {
    "vivy-shared.dll"
}

/// Returns false when no C compiler is available so callers skip with a
/// recorded reason instead of panicking (CI runners may lack `cl`/`cc`).
fn build_fake_artifact(dir: &Path) -> bool {
    let src = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fake_vivy.c");
    for header in ["vivy-shared.h", "vivy_abi.h"] {
        let body = if header == "vivy_abi.h" {
            "#define VIVY_ABI_VERSION 1\n"
        } else {
            "/* fake */\n"
        };
        std::fs::write(dir.join(header), body).unwrap();
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    let (cc, args, out) = (
        "cc",
        vec!["-shared", "-fPIC", "-O0"],
        dir.join("vivy-shared.so"),
    );
    #[cfg(target_os = "macos")]
    let (cc, args, out) = (
        "cc",
        vec!["-shared", "-fPIC", "-O0"],
        dir.join("vivy-shared.dylib"),
    );
    #[cfg(target_os = "windows")]
    let (cc, args, out) = {
        // MSVC: cl /LD produces vivy-shared.dll in dir.
        ("cl", vec!["/LD"], dir.join("vivy-shared.dll"))
    };
    let mut cmd = Command::new(cc);
    #[cfg(not(target_os = "windows"))]
    cmd.args(args).arg(src).arg("-o").arg(&out);
    #[cfg(target_os = "windows")]
    {
        cmd.args(args)
            .arg(format!("/Fe{}", out.display()))
            .arg(format!("/Fo{}\\", dir.display()))
            .arg(src)
            .current_dir(dir);
    }
    match cmd.output() {
        Ok(out) if out.status.success() => true,
        Ok(out) => panic!(
            "fake library build failed: {}",
            String::from_utf8_lossy(&out.stderr)
        ),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("fake artifact skipped: C compiler {cc:?} not on PATH");
            false
        }
        Err(e) => panic!("spawn C compiler for fake artifact: {e}"),
    }
}

fn fake_config() -> InitConfig {
    InitConfig { config_path: PathBuf::from("fake.yaml"), without_ears: Some(true) }
}

#[test]
fn load_missing_dir_is_load_failed() {
    let err = VivyLibrary::load(Path::new("/definitely/not/here")).unwrap_err();
    assert_eq!(err.kind, ErrorKind::LoadFailed);
}

#[test]
fn load_missing_abi_header_is_load_failed() {
    let dir = tempfile::tempdir().unwrap();
    // Library present, bundled vivy_abi.h absent.
    std::fs::write(dir.path().join(shared_name()), b"elf-ish").unwrap();
    std::fs::write(dir.path().join("vivy-shared.h"), b"").unwrap();
    let err = VivyLibrary::load(dir.path()).unwrap_err();
    assert_eq!(err.kind, ErrorKind::LoadFailed);
    assert!(err.message.contains("vivy_abi.h"), "{err}");
}

#[test]
fn fake_lifecycle_and_error_paths() {
    let _g = FAKE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile::tempdir().unwrap();
    if !build_fake_artifact(dir.path()) {
        return;
    }
    let lib = VivyLibrary::load(dir.path()).unwrap();

    // Happy path: init asserts the echoed ABI version against the header.
    let host = Host::start(lib.clone(), &fake_config()).unwrap();
    assert_eq!(host.raw_handle(), 42);

    // Echo call returns the request as value.
    let v = host
        .call("session/list", json!({"limit": 1}), None)
        .unwrap();
    assert_eq!(v["echo"]["method"], "session/list");
    assert_eq!(v["echo"]["params"]["limit"], 1);

    // Empty poll before the gap is armed.
    let batch = host.poll(10).unwrap();
    assert!(batch.events.is_empty() && !batch.gap);

    // Overflow flag surfaces through the batch.
    host.call("fake/arm-gap", json!(null), None).unwrap();
    let batch = host.poll(500).unwrap();
    assert!(batch.gap);
    assert_eq!(batch.events[0].method, "fake/event");

    // Null return -> transport_lost.
    let err = host.call("fake/null", json!(null), None).unwrap_err();
    assert_eq!(err.kind, ErrorKind::TransportLost);

    // Malformed JSON and non-UTF-8 -> internal, allocations still freed.
    let err = host.call("fake/malformed", json!(null), None).unwrap_err();
    assert_eq!(err.kind, ErrorKind::Internal);
    let err = host.call("fake/utf8", json!(null), None).unwrap_err();
    assert_eq!(err.kind, ErrorKind::Internal);
    let err = host.call("fake/noenvelope", json!(null), None).unwrap_err();
    assert_eq!(err.kind, ErrorKind::Internal);

    // Ordered shutdown is idempotent; late calls are rejected closed
    // without touching FFI again.
    host.shutdown().unwrap();
    host.shutdown().unwrap();
    let err = host.call("session/list", json!(null), None).unwrap_err();
    assert_eq!(err.kind, ErrorKind::Closed);

    // Allocation hygiene: every produced return was freed exactly once
    // (the counters call's own return is the one pending free).
    let host2 = Host::start(lib.clone(), &fake_config()).unwrap();
    let v = host2.call("fake/counters", json!(null), None).unwrap();
    let allocs = v["allocs"].as_u64().unwrap();
    let frees = v["frees"].as_u64().unwrap();
    // Snapshot excludes the counters call's own alloc and free: every
    // previously returned buffer must have been freed exactly once.
    assert_eq!(allocs, frees, "every produced return freed exactly once");
    drop(host2);
}

#[test]
fn fake_abi_mismatch_and_duplicate_init() {
    let _g = FAKE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile::tempdir().unwrap();
    if !build_fake_artifact(dir.path()) {
        return;
    }
    // Header demands ABI 2: the request carries 2 and the fake refuses.
    std::fs::write(dir.path().join("vivy_abi.h"), "#define VIVY_ABI_VERSION 2\n").unwrap();
    let lib = VivyLibrary::load(dir.path()).unwrap();
    let err = Host::start(lib, &fake_config()).unwrap_err();
    assert_eq!(err.kind, ErrorKind::IncompatibleAbi);
}

#[test]
fn event_pump_forwards_events_and_gap() {
    let _g = FAKE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile::tempdir().unwrap();
    if !build_fake_artifact(dir.path()) {
        return;
    }
    let lib = VivyLibrary::load(dir.path()).unwrap();
    let host = Arc::new(Host::start(lib, &fake_config()).unwrap());
    host.call("fake/arm-gap", json!(null), None).unwrap();

    let (tx, rx) = mpsc::channel();
    let stop = Arc::new(AtomicBool::new(false));
    let pump = spawn_event_pump(host.clone(), stop.clone(), move |e| {
        tx.send(e).unwrap();
    });

    let mut saw_event = false;
    let mut saw_gap = false;
    for _ in 0..200 {
        match rx.recv_timeout(Duration::from_millis(50)) {
            Ok(BridgeEvent::Vivy(n)) => saw_event |= n.method == "fake/event",
            Ok(BridgeEvent::Gap) => saw_gap = true,
            Ok(BridgeEvent::TransportLost(m)) => panic!("unexpected transport loss: {m}"),
            Err(_) => {
                if saw_event && saw_gap {
                    break;
                }
            }
        }
        if saw_event && saw_gap {
            break;
        }
    }
    assert!(saw_event && saw_gap, "event+gap must reach the single pump");
    stop.store(true, std::sync::atomic::Ordering::Release);
    pump.join().unwrap();
    host.shutdown().unwrap();
}

/// Real packed artifact: exercised when DIVA_VIVY_RUNTIME (dir containing
/// vivy-shared.so + headers) and DIVA_VIVY_CONFIG are provided. Skipped
/// otherwise — this is live evidence, not a mock.
#[test]
fn real_artifact_init_call_shutdown() {
    let _g = FAKE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let (Ok(dir), Ok(cfg)) = (
        std::env::var("DIVA_VIVY_RUNTIME"),
        std::env::var("DIVA_VIVY_CONFIG"),
    ) else {
        eprintln!("DIVA_VIVY_RUNTIME/DIVA_VIVY_CONFIG unset; skipping real-artifact test");
        return;
    };
    let lib = VivyLibrary::load(Path::new(&dir)).unwrap();
    let host = Host::start(
        lib,
        &InitConfig { config_path: PathBuf::from(cfg), without_ears: Some(true) },
    )
    .unwrap();
    let v = host.call("initialize", json!({}), Some(30_000)).unwrap();
    assert_eq!(v["protocol_version"], "vivy.rpc.v1", "initialize returned {v}");
    host.shutdown().unwrap();
}

/// DN-1 real native smoke through the sealed DLL: initialize → session
/// create/read → run subscribe/log on a missing run (RPC error envelope
/// preserved) → approval/question list → session delete. Same env gate as
/// the lifecycle test; skipped when the artifact is not staged.
#[test]
fn real_artifact_dn1_rpc_smoke() {
    let _g = FAKE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let (Ok(dir), Ok(cfg)) = (
        std::env::var("DIVA_VIVY_RUNTIME"),
        std::env::var("DIVA_VIVY_CONFIG"),
    ) else {
        eprintln!("DIVA_VIVY_RUNTIME/DIVA_VIVY_CONFIG unset; skipping real-artifact test");
        return;
    };
    let lib = VivyLibrary::load(Path::new(&dir)).unwrap();
    let host = Host::start(
        lib,
        &InitConfig { config_path: PathBuf::from(cfg), without_ears: Some(true) },
    )
    .unwrap();

    let init = host.call("initialize", json!({}), Some(30_000)).unwrap();
    assert_eq!(init["protocol_version"], "vivy.rpc.v1");
    assert!(init["capabilities"].as_array().map(|c| !c.is_empty()).unwrap_or(false));

    let sess = host
        .call("session/create", json!({"title": "dn1-smoke"}), Some(30_000))
        .unwrap();
    let sid = sess["id"].as_str().unwrap().to_string();

    let list = host.call("session/list", json!({}), Some(30_000)).unwrap();
    assert!(list["sessions"].as_array().unwrap().iter().any(|s| s["id"] == sid));

    let got = host.call("session/get", json!({"session_id": sid}), Some(30_000)).unwrap();
    assert_eq!(got["session"]["id"], sid);
    assert!(got["messages"].is_array());

    // Subscribe attaches even to a missing run id (no existence check).
    let sub = host
        .call("run/subscribe", json!({"run_id": "run_missing"}), Some(30_000))
        .unwrap();
    assert_eq!(sub["run_id"], "run_missing");
    assert!(sub["subscription_id"].as_str().unwrap().starts_with("sub_"));

    // Missing/inactive run: the backend's RPC error must reach the caller
    // intact (fixture observed -32004 for run/cancel on an inactive run).
    let err = host
        .call("run/cancel", json!({"run_id": "run_missing"}), Some(30_000))
        .unwrap_err();
    assert_eq!(err.code, -32004, "expected rpc error, got {err}");

    let approvals = host.call("approval/list", json!({}), Some(30_000)).unwrap();
    assert!(approvals["approvals"].is_array());
    let questions = host.call("question/list", json!({}), Some(30_000)).unwrap();
    assert!(questions["questions"].is_array());

    let del = host
        .call("session/delete", json!({"session_id": sid}), Some(30_000))
        .unwrap();
    assert_eq!(del["deleted"], true);

    host.shutdown().unwrap();
}
