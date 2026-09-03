# C6 production cutover acceptance

## Automated acceptance

1. Start from a config with all channels disabled; the gateway remains ready and runtime status is
   empty.
2. Enable one of the six active channels with valid credentials; the update returns `status: ok`,
   and runtime status reports it registered with lifecycle/health details.
3. Submit an enabled channel with incomplete credentials; the gateway remains available and runtime
   health reports the adapter failure. If candidate construction itself fails, the previous config
   and runtime remain active.
4. Confirm external ingress is admitted by Fabric before AgentLoop receives `StartChannelTurn`.
5. Confirm egress is converted to a typed `ChannelCommand`, admitted by the bounded pacing lane, and
   produces a truthful delivery receipt.
6. Confirm retired channel names and old chat/SSE endpoints are rejected or absent.

## Manual acceptance still required

1. Run the desktop against the cutover gateway and verify admission, streaming, final, cancellation,
   reconnect/resume, planning state, and Mate presentation.
2. On one controlled real platform, verify inbound processing and a final outbound receipt/message
   identifier.
3. Complete C6-D, run the Rust 1.80 probe and every release gate, then perform the atomic merge to
   `dev`.
