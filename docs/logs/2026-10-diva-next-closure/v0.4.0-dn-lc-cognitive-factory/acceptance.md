# v0.4.0 — DN-LC acceptance

Owner acceptance pending. To verify:

1. `internal/cognitivecontract/ports.go` matches ledger C2-6 —
   Factory/FactoryInput/Bundle/ControlPort/Dispatcher/Capture* shapes.
2. `core/cognitive-factory@v1` is closed and conditional
   (`internal/moduleport/ports.go`, VIVY-PORT-CATALOG); the generated
   seam mirrors `MaskFactory` (`CognitiveFactoryValue` + `Has` on
   `internal/generated/assembly/zz_default.go`, never hand-edited).
3. `internal/modules/diva-cognitive` declares exactly the 20 C2-3
   action IDs; invoke is fail-closed (`ErrUnarmed` /
   "unavailable until handlers land"), no success placeholder.
4. `internal/app` composition order: selected factory → bundle → owned
   capture subscription appended at ObserverHost construction →
   `AttachRuntime` after `NewService`; omitted recipe behaves as before
   (asserted by tests).
5. Plan gate output in verification.md; the three
   `sdk/internal/assembly` UI failures are the pre-existing
   `ui/node_modules` env gap, not this change.
6. garden `BindEvolutionSource` (laputa `0b17b9e`) resolves no backend;
   the bound Domain keeps `BindEvolution`'s eager writer check.
