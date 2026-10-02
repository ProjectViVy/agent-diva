# DN-P release notes

No user-facing release cut in this iteration (plan-owned stage).

Deliverables landed:

- `scripts/ci/check_vivy_backend_boundary.py` — P0-A runtime boundary gate (dep graph + loader/resource manifest + package contents).
- `.github/workflows/ci.yml` — `boundary-gate` CI job.
- Acceptance evidence for the linux/amd64 installed product (this directory).

Known limitation carried forward: Windows native acceptance still open; shutdown deadline transient noted in verification.
