# GUI Stream Final Reconciliation

The agent loop now flushes the safe suffix retained for internal-protocol detection when a provider stream completes. The GUI now treats the final SSE payload as the authoritative complete response, repairing any missing streamed deltas.

Impact: GUI chat responses no longer remain truncated by the 32-character protocol-guard buffer or by an interrupted delta stream.
