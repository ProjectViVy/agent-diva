# Real protected Tool provenance

Added an actual App test that asks the real protected read_file Tool to read an owned synthetic file, then drives native capture, ACTMEM activity and reflection. The recorded second model request contains the actual file canary. The captured source omits Tool content, preserves original user and assistant roles, and its user projection excludes the assistant-only reply. ACTMEM Recap and the derived canonical note preserve only the admitted user fact.

Test infrastructure now decodes assistant Tool-call messages that omit content and records raw requests before decoding, preserving failed requests too. No production behavior or backend was replaced. Native evolution evidence retains the role-tagged conversation archive as untrusted data; absence of every assistant string from the raw archive is not the contract.
