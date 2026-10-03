# v0.4.6 acceptance

Pending owner acceptance. Evidence for review:

1. FrozenCore reaches the real model input: the 7 frozen sections sit
   between persona and code-mode/frame/mask in the actual request
   messages — `TestPrimaryFrozenCoreActualModelInput`; payload records
   `Frozen{Source:"garden/frozen-core", Revision:"v2", Digest}`; v1 /
   corrupt / oversize rejected explicitly in `diva-cognitive.Prepare`.
2. Admission fence: persisted input pins verified against the bound
   composition on every path (new / dedupe / recovery) — scope triple,
   destination, strategy digest, mission revision —
   `TestMissionAdmissionFence`.
3. Capture discipline: supervisor-session primaries excluded with an
   accepted-skip receipt; committed runs still keyed by
   `run_id:journal_seq` — `TestSupervisorCaptureExcluded`.
4. Safe lifecycle: durable `Blocked` reasons; `unknown_outcome` never
   retries; manual trigger is the only override —
   `TestUnknownOutcomeNoNewAttempt`; retry cap still 3 for ordinary
   failures (`TestCognitiveFailedRunRetriesWithNewKey` unchanged).
5. Embedded start unified: `StartEmbeddedServices` starts the loop once,
   `Close` stops it before drains —
   `TestEmbeddedCognitionStartsOnce`.

Suites: runtime/observerhost/embedded/app/diva-cognitive all green;
-race green on the new tests; gofmt clean.
