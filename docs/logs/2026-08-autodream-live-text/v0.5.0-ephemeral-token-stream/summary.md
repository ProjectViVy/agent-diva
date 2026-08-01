# Ephemeral AutoDream token stream

AutoDream reflection now consumes the provider streaming interface. Text deltas are retained only in the Manager process memory for the active run and exposed to the Evolution monitor dialog as live model output.

The display does not include hidden reasoning deltas, tool-call deltas, API credentials, or input evidence. It is not persisted to typed Memory, AutoDream event files, or run history.
