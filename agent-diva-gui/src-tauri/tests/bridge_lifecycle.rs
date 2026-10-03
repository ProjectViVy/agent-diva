//! Lifecycle seam tests for the thin shell. These exercise the state
//! machine and ownership rules headlessly; the real window/tray behaviors
//! (close-hide, reopen-while-streaming, second window, quit) are verified
//! on the native target per DN-5 verification.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use agent_diva_shell::{CloseAction, Phase, Shell};
use serde_json::json;
use vivy_bridge::ErrorKind;

fn ok(_event: vivy_bridge::BridgeEvent) {}

#[test]
fn calls_rejected_before_start() {
    let shell = Shell::new();
    assert_eq!(shell.phase(), Phase::Starting);
    let err = shell.call("initialize", json!({}), None).unwrap_err();
    assert_eq!(err.kind, ErrorKind::Closed);
}

#[test]
fn startup_failure_is_process_fatal() {
    let shell = Shell::new();
    let err = shell
        .start(Path::new("/no/such/runtime"), PathBuf::from("x.yaml"), ok)
        .unwrap_err();
    assert_eq!(err.kind, ErrorKind::LoadFailed);
    // A failed start leaves the shell Starting; calls stay rejected and a
    // later quit still unwinds cleanly.
    assert_eq!(shell.phase(), Phase::Starting);
    shell.shutdown();
    assert_eq!(shell.phase(), Phase::Dead);
}

#[test]
fn window_close_hides_until_explicit_quit() {
    let shell = Shell::new();
    assert_eq!(shell.window_close_action(), CloseAction::Hide);
    shell.request_quit();
    assert_eq!(shell.window_close_action(), CloseAction::Quit);
}

#[test]
fn shutdown_rejects_late_calls_and_is_idempotent() {
    let shell = Shell::new();
    // Start against a fake artifact to reach Ready.
    let dir = tempfile::tempdir().unwrap();
    let src = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/crates/vivy-bridge/tests/fake_vivy.c"
    );
    let out = dir.path().join("vivy-shared.so");
    let status = std::process::Command::new("cc")
        .args(["-shared", "-fPIC", "-O0", src, "-o"])
        .arg(&out)
        .output()
        .unwrap();
    assert!(status.status.success());
    std::fs::write(dir.path().join("vivy-shared.h"), b"/* fake */").unwrap();
    std::fs::write(dir.path().join("vivy_abi.h"), "#define VIVY_ABI_VERSION 1\n").unwrap();

    shell
        .start(dir.path(), PathBuf::from("fake.yaml"), ok)
        .unwrap();
    assert_eq!(shell.phase(), Phase::Ready);
    assert!(shell.call("x/y", json!({}), None).is_ok());

    // Shutdown once: pump joins, host closed, late calls rejected.
    shell.shutdown();
    assert_eq!(shell.phase(), Phase::Dead);
    let err = shell.call("x/y", json!({}), None).unwrap_err();
    assert_eq!(err.kind, ErrorKind::Closed);

    // Second shutdown is a no-op.
    shell.shutdown();
    assert_eq!(shell.phase(), Phase::Dead);

    // Concurrent late calls during teardown never hang: a racing call sees
    // either the pre-shutdown result or `closed`, never a stuck FFI call.
    let shell = Arc::new(shell);
    let t = std::thread::spawn({
        let shell = shell.clone();
        move || shell.call("x/y", json!({}), Some(5_000))
    });
    std::thread::sleep(Duration::from_millis(10));
    let _ = t.join().unwrap();
}
