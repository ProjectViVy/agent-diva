# Mentle Prompt Rebuild Summary

The reported `L2 Palace Memory` regression was based on a superseded prompt contract. Since the governed-memory boundary was introduced, runtime flags no longer inject Mentle routing text; the selected `MemoryProvider` is the sole prompt-memory source.

Regression coverage now exercises both `register_default_tools` and `rebuild_tools_for_turn`. An active Mentle runtime retains its provider-backed `Memory Startup Status`, while an inactive runtime does not expose Mentle tools or legacy routing text.

No public API or production behavior changed.
