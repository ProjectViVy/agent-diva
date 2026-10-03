# v0.4.1 — DN-2A acceptance

Owner acceptance pending. To verify:

1. Wire parity with the DN-0C fixture: `turn/start` attachments are
   `{name?, mime_type, data}` base64 only; `session/set_permission`
   returns the session DTO the controller reads back.
2. Boundaries enforced client-side identical to internal/attachment:
   4 images, 5 MiB decoded each, sniffed MIME, and the complete
   serialized request ≤ 4 MiB — rejection mutates nothing.
3. Permission semantics: preset armed before the turn; admitted DTO
   mismatch or ambiguous write reconciles through `session/get`; a
   stale policy never carries a send.
4. Mutation lane: duplicate sends / a session switch cannot interleave
   (`sendChain`); timeout after a submitted turn never resends.
5. `just gui-test` + `just gui-build` on a dev machine; native smoke:
   pick a PNG, choose Cautious, send — image lands in history with
   `data_url`, sandbox_mode flips to read_only.
