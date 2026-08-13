# Summary

## Change

Fixed duplicate thinking bubbles in Diva Pet whispers. The whispers panel now relies on the streaming agent message placeholder for the loading indicator and no longer renders a second global `isTyping` bubble.

## Impact

- Affects `agent-diva-gui` Diva Pet whispers chat panel.
- Does not change message sending, streaming event handling, ASR recognition, or TTS playback flow.

