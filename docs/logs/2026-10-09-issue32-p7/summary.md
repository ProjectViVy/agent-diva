# Issue #32 P7 DIVA consumer integration

## Scope

Repin DIVA to the final VIVY/Laputa source closure, run source-bound consumer
and frontend gates, and record the remaining native candidate prerequisites.
This implements the DIVA portion of VIVY's
`docs/superpowers/plans/issue32-remediation/P7-final-integration.md`.

## Selected source closure

- VIVY: `54462d1553428430412600c1070d7f06161e2488`, tree SHA-256
  `396d34a2de3a50881133f6dfe0683f3918ade2bd054a21390b07bc65ca410596`.
- Laputa: `30fa208e3cded4af43f8cb226d80b225b1910854`, tree SHA-256
  `99e52ac972be48b867fd55554bc6f5195cd8af4ae75a209fa2309f48342988b4`.
- INOFY remains `71e2c9bbe47d5d095b27d756e38eaff565d93d36`.
- Locked tools: Go `go1.26.4`, Wails `v3.0.0-beta.27`, Node `24`, pnpm
  `10.33.2`; Linux target is `linux/amd64`, CGO enabled, `gtk3` build tag.
- Canonical source-lock SHA-256:
  `b7162395324aa85e0feac62fa9daa296224798888bbfb716250198a153b1f64a`.

## Outcome

The DIVA source pin, Linux lock derivation, strict candidate/consumer contracts,
sealed consumer Go race tests, frontend typecheck/build/tests, Wails binding
drift check, Rust bridge tests, and desktop transition boundaries pass. The
normal GUI test command exposed a Wails runtime polling timer racing with
parallel happy-dom worker teardown. Vitest now uses one worker; the standard
test command passes all 578 tests without unhandled errors.

The Linux sealed native package remains blocked because GTK3/WebKit4.1
development packages are absent. Windows-native build and installed-product
acceptance also remain open. This is an engineering-source integration record,
not candidate, owner, or release acceptance.

The first cold pack attempt exhausted the workspace filesystem while compiling
the full SDK closure. A retry used isolated `/tmp` caches and reached the
native build step, which then failed on the missing system packages listed in
`verification.md`. All task-created compile caches and stage directories were
removed afterward.
