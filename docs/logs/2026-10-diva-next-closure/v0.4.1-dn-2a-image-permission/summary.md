# v0.4.1 — DN-2A image send + admitted permission controls

Chat now sends real images and confirmed permission presets through the
existing `vivy_call` lane — no parallel sender, no dropped drafts.

Delivered: `TurnAttachment`/`PermissionPreset` contract types;
`turnStart(sessionId, text, attachments?)` and
`setSessionPermission(sessionId, preset)` on `VivyClient`; the
`chat-images.ts` validation lane mirroring the VIVY internal/attachment
limits (png/jpeg/gif/webp content sniff, ≤4 images, ≤5 MiB decoded) plus
the ABI 4 MiB framed-request bound measured on the exact
`{method, params}` JSON; a serialized `sendChain` mutation lane on the
controller with validation-before-echo; preset arming via
`session/set_permission` whose admitted DTO is published to the session
projection, with `session/get` readback reconciliation on mismatch or
ambiguous outcome — send blocked, never under a stale policy; ChatView
image picker/chips with pick-time and send-time validation (draft is
preserved on every client-detectable failure); EN/ZH copy.

Pending (recorded): model image capability is server-authoritative —
an incapable configured model fails the send explicitly at turn/start;
developer-host smoke with a real image-capable model is owner acceptance;
preview/web mode remains unsent by design.
