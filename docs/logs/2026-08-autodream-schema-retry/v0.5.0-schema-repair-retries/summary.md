# Summary

AutoDream reflection now requests one exact RFC 8259 JSON object with a minimal provider-facing candidate schema. The prompt explicitly forbids internal identifiers, evidence payloads, workspace scope, transcript text, Markdown, and surrounding prose.

When parsing or schema validation returns `InvalidSchema`, the Manager makes a fresh reflection request with an explicit format-repair instruction. It performs at most three total attempts and then fails closed without publishing a proposal or writing Memory.

Provider authentication, availability, and timeout errors are not disguised as schema errors and are not retried by this format-repair loop.
