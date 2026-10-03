# v0.4.7 acceptance

Pending owner acceptance. Evidence for review:

1. Selected-turn regeneration replays the ORIGINATING turn: text via
   atomic session/edit, images via validated inclusive rewind + resend
   of the original bytes — `regenerateSelectedTextAtomicEdit`,
   `regenerateImageInclusiveCutoff`.
2. Rewind-succeeded/send-failed keeps a retryable draft and the rewound
   view — `rewindSucceededSendFailedDraft`.
3. Cancel after restart reads authoritative run state before claiming
   settled — `cancelAfterRestartReconciles`.
4. start_goal goes through plan/decide on the existing work controller
   with objective + max_rounds + expected_version —
   `goalUsesWorkController`; UI: rounds input + "批准并启动目标循环" on
   the approval card.
5. `onConversationInvalidate(reason)` hook fires before every session
   mutation — the DN-6C voice lane subscribes to it.

Suites: named vitest files 69/69, `just gui-test` 518/518,
`just gui-build` clean, vue-tsc clean.
