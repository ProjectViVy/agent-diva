# Streaming UTF-8 Fix Summary

## Change

- Preserved incomplete UTF-8 code points across HTTP response chunks in the OpenAI-compatible, Anthropic, and Ollama streaming providers.
- Replaced lossy per-chunk decoding with a shared incremental decoder that reports malformed or incomplete final UTF-8 instead of inserting replacement characters.

## Impact

Chinese and other multi-byte output can no longer become `�` merely because a network chunk boundary splits a character. The corrected text flows unchanged to the GUI, session store, and response log.
