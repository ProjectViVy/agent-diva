# Discord C5 wire fixtures

These redacted fixtures describe the official Gateway and REST wire shapes
used by the native adapter tests. The Gateway transcript covers HELLO,
IDENTIFY, READY, MESSAGE_CREATE, heartbeat/ACK, RESUME/RESUMED, reconnect and
invalid-session opcodes. REST data covers message receipts, standard
`message_reference`, rate limits, permission errors, health discovery and the
multipart `payload_json`/`files[n]` shape.

The executable WebSocket/HTTP fixtures remain inline in `src/adapters/discord.rs`
so they can assert each request and response. These files are also parsed by a
fixture-shape test and the shared C5 fixture sanity check. No credential,
signed URL, or private Discord identifier is stored here.
