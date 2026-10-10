# Confirmed remaining boundary

## S08 V19: no model-visible WORLD / ACTMEM read tool

- **Observed:** The actual model request contains neither synthetic WORLD/ACTMEM data nor the cognitive control action IDs. A direct public host action read returns the persisted values, but it does not create a model tool call.
- **Cause:** `vivy/diva-cognitive` exposes these actions through the human `controlaction` plane; `internal/modules/diva-cognitive/module.go` explicitly states that no action is a model tool.
- **Impact:** Automatic authority isolation is verified, but the Agent cannot explicitly retrieve WORLD or ACTMEM as V19 requires.
- **Next step:** Agree and document the architecture for a model-visible, host-bound read surface, then add an actual provider tool declaration/call/result test and authorization negatives. Do not count UI/control-action reads as Agent recall or inject the authorities into prompt context.
- **Status:** Open. No product code or authority semantics were changed here.
