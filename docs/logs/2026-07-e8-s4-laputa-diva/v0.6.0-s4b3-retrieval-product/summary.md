# S4b-3 Summary

> Repatriated evidence from `refactor/deep-governance` on 2026-07-30.
> This implementation is a port source, not proof that current
> `agent-diva-pro` already uses Embedded Laputa retrieval.

Embedded Laputa now provides profile-local FTS5/BM25 retrieval with metadata visibility filtering, application-layer importance/recency/persona reranking, stable section diversity, and a bounded top-eight result.

Manager `/v1` exposes typed status, search, proposal, and section reads. The product GUI exports bound status/search/section functions only through `@/api`. Runtime status reports Garden as `not_configured` and sync as `local_only`; schema/init failures are not converted into empty success.

Garden client, synchronization, embeddings, Memory Pack, legacy migration, and automatic Evolution apply remain out of scope.
