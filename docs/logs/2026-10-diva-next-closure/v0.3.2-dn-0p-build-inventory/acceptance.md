# v0.3.2 — DN-0P acceptance

Owner acceptance pending. To verify:

1. Read `docs/plans/diva-next/fixtures/closure-build-inputs.json` — pins,
   replace map, module graph, native inventory, artifact hashes, pending
   Windows/amd64 command must all be present and truthful.
2. Check the C2-5 link in `docs/plans/diva-next/backend-separation-contracts.md`.
3. Optional re-run: from a fresh VIVY checkout at `1db8b55` with laputa
   `dc6066e` and INOFY `71e2c9b` as siblings, run
   `go run ./sdk pack --recipe recipes/diva.vivy.yml --target shared --output
   out` and compare `sha256sum out/vivy-shared.so` with
   `fe5b334f…` (byte-identical only if the toolchain is identical; hash drift
   on a different toolchain is expected and recorded, not a failure).
4. No Windows acceptance is claimed by this inventory.
