# v0.4.0 — DN-LC cognitive factory composition

Delivered the C2-6 compile-and-compose seam in agent-vivy
(`2f009eee` on `feat/dn-closure-wave1`) plus the garden input-ports
change in laputa (`0b17b9e`).

Delivered: `internal/cognitivecontract` (Factory/FactoryInput/Bundle/
ControlPort/Dispatcher + Capture DTOs), closed Port
`core/cognitive-factory@v1` through moduleport + defaults catalog +
generated Assembly (`GoBinding.CognitiveFactory` mirrors `MaskFactory`
verbatim; Assembly regenerated via `go generate`), module
`vivy/diva-cognitive` (Open() binds one Garden owner under
`cfg.DataDirectory()`, arms source/sink/mission via the new
backend-independent `agentapi.BindEvolutionSource`, declares all 20
`diva.cognitive.*` providers with closed schemas and fail-closed
invokes — no success placeholders until DN-4C), and App composition
(selected factory → armed bundle → owned capture subscription appended
to `observerhost.Config` at construction → `AttachRuntime` arms the
App-backed ControlPort after `NewService`; omitted recipe unchanged;
close order reverse-owned).

Pending (recorded, not patched): all 15 non-control action handlers
(DN-4C), the ActionHost facade wrapper (DN-4C), embedded-loop start
(DN-4B), `ui/node_modules` env gap makes three pre-existing
`sdk/internal/assembly` UI tests un-runnable here, garden ONNX native
inventory for Windows (DN-P-C/DN-8C).
