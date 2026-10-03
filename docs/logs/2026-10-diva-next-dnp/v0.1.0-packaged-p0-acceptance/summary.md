# DN-P — Packaged P0-A acceptance + runtime boundary gate

## Scope executed

DN-P tasks 1–5 against the sealed artifact set plus the freshly built installer on linux/amd64.

1. **Artifact verification vs DN-L manifest** — packaged `vivy-runtime/` contents compared bit-for-bit against the staged DN-L inputs.
2. **Deterministic lifecycle chain on the sealed .so** — ctypes driver `/home/ubuntu/diva-dnp-accept/lifecycle.py` against `vivy-shared.so` in `agent-diva-gui/src-tauri/vivy-runtime/` (the exact bytes that ship in the deb).
3. **Restart recovery on the same data root** — `VivyShutdown` → `VivyInit` cycles reuse `/home/ubuntu/diva-dnp-accept/vivy.db`; recovered state inspected via backend calls, not UI.
4. **Live-model chain** — provider `sensenova` (`sensenova-6.8-flash-lite` via `SENSENOVA_API_KEY`), chat → stream → policy-gated tool approval → continue → second run cancelled.
5. **Boundary gate** — `scripts/ci/check_vivy_backend_boundary.py` over the Rust dependency graph, tauri.conf.json resource/externalBin/invoke manifest, and the real `.deb` package contents. Wired into `.github/workflows/ci.yml` as `boundary-gate`.

## GUI-level window lifecycle (installed product)

Run of `target/release/agent-diva-shell` with `DIVA_VIVY_RUNTIME`/`DIVA_VIVY_CONFIG` env:

- window opened and rendered the DIVA UI (webkit2gtk-4.1 host);
- `wmctrl -c` close → window hidden, process stayed alive (close = hide);
- second launch → single-instance refused a second process and focused the existing window (same XID, window restored);
- `SIGINT` → ordered teardown, process exited cleanly.

## Shipped-artifact evidence

```
sha256  83e8ee88fded…ecb5  vivy-shared.so   (deb == staged pin, identical)
sha256  f1467ade0d3b…ef   vivy_abi.h       (deb == staged pin)
sha256  3d8046bb4be8…191a  generation.json  (deb == staged pin)
generationId 1fd14fb2…fc5b   recipeDigest 723eb0bc…
bundles  DIVA_0.4.10_amd64.deb / DIVA-0.4.10-1.x86_64.rpm / AppImage
```

The library was re-sealed once during this stage: `agent-vivy@2947c472`
fixed the embedded control peer's missing Caller/Identity (module actions
were unreachable over the in-process pipe). Artifact hashes above are the
post-fix seal; the acceptance chain was re-run against those bytes.

Package contents (deb): `usr/bin/agent-diva-shell` + `usr/lib/DIVA/vivy-runtime/{vivy-shared.so,vivy_abi.h,generation.json}` — no legacy business crates, no Manager executable, no domain database, boundary gate clean.

## Outcome

P0-A evidence gates pass on linux/amd64 installed product. One transient finding recorded (shutdown deadline under in-flight utility generation) — see TODOLIST `VIVYSHUTDOWN-DEADLINE-UTIL`.
