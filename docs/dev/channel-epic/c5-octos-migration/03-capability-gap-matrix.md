# C5 target capability matrix

`T` is a C5 commitment and requires wire-level offline evidence. `F` means the adapter must not
advertise the capability; applicable commands return `UnsupportedCapability` without transport
side effects. A later implementation may not flip `F` to `T` without updating the ADR, provenance,
fixtures, and evidence manifest in the same focused commit.

| Capability | Telegram | Discord | Feishu | DingTalk | Email | QQ |
| --- | :---: | :---: | :---: | :---: | :---: | :---: |
| `IngressText` | T | T | T | T | T | T |
| `IngressMarkdown` | F | T | T | T | F | F |
| `IngressThread` | T | T | T | F | T | F |
| `IngressGroup` | T | T | T | T | F | T |
| `IngressDirect` | T | T | T | T | T | T |
| `IngressTypedAttachments` | T | T | T | T | T | T |
| `IngressDedupId` | T | T | T | T | T | T |
| `EgressText` | T | T | T | T | T | T |
| `EgressMarkdown` | T | T | T | T | F | F |
| `EgressChunking` | T | T | F | F | F | T |
| `EgressReply` | T | T | T | F | T | T |
| `EgressImage` | T | T | T | T | T | T |
| `EgressAudio` | T | T | F | T | T | T |
| `EgressVideo` | T | T | F | T | T | T |
| `EgressFile` | T | T | T | T | T | T |
| `EgressCard` | F | T | T | F | F | F |
| `InteractionTyping` | T | T | F | F | F | F |
| `InteractionListening` | T | F | F | F | F | F |
| `InteractionEdit` | T | T | T | F | F | F |
| `InteractionDelete` | T | T | T | F | F | F |
| `InteractionReaction` | F | T | F | F | F | F |
| `InteractionStreamFinalize` | T | T | T | F | F | F |
| `ReliabilityHealth` | T | T | T | T | T | T |
| `ReliabilityHeartbeat` | F | T | T | T | F | T |
| `ReliabilityResume` | F | T | F | F | F | T |
| `ReliabilityTokenRefresh` | F | F | T | T | F | T |
| `ReliabilityPacing` | T | T | T | T | T | T |
| `ReliabilitySupervisedRestart` | T | T | T | T | T | T |

## Evidence meaning

- Ingress capabilities require a real platform event fixture admitted as the expected typed
  `ChannelEnvelopeV1`; a parser-only unit test is insufficient.
- Egress/interaction capabilities require a captured mock request, platform response parsing, and
  a truthful `DeliveryReceipt` or typed error.
- Reliability capabilities require state-transition evidence, not a constant declaration.
- `ReliabilityPacing` and `ReliabilitySupervisedRestart` are proven with each real adapter mounted
  in the C2 runtime, not by reimplementing those mechanisms in platform code.
- `IngressTypedAttachments` requires storage through the content-addressed attachment seam and
  validation of URI, MIME, size, digest, and optional file name.

## Frozen limits to determine from platform constants

Each adapter declares `max_text_chars`, `max_attachment_bytes`, supported MIME types, and an
optional rate-limit hint. These are code/runtime-probe values, never user capability toggles. The
platform spec names the source; tests assert boundary-1, boundary, and boundary+1 behavior.
