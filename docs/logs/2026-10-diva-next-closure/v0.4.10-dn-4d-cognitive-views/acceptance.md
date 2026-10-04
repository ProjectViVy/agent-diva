# v0.4.10 acceptance — DN-4D

## Claims supported by evidence on this VM

- Every captured C2-3 action envelope (20/20 fixture rows) parses
  verbatim through `api/cognitive.ts`; no legacy aliases introduced.
- Setup gating: uninitialized/incomplete persona visibly gates primary
  send (ChatView) and shows the init form; unavailable capability keeps
  settings navigable.
- CAS conflict retains the draft and refetches authority; unknown
  outcomes surface with an explicit reconcile path — zero automatic
  writes after timeout.
- Current persona revisions and session FrozenCore revisions are
  separate fields everywhere; frozen recovery_required is shown and
  cannot be cleared by a policy write.
- Memory scope/destination come only from the bound status — no
  hidden-scope fetch exists in the client.
- Scope is DN-C2 only: no general notebook/report/resource pages.

## Not proven here

- Live backend round-trip on a real host (needs packaged shell +
  Garden/Gardenless profile). Owner acceptance pending (DN-8C).
