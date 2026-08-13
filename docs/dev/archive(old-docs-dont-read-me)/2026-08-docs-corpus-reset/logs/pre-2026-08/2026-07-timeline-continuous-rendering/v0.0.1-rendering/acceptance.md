# Acceptance

## Acceptance Steps
1. Recompile and start the project backend (`just run`).
2. Start the `agent-diva-gui` frontend.
3. Open the Token Statistics panel.
4. Verify that the "1h" option is completely removed from the selector, and "1d" is restored as the default.
5. Verify that the trend chart renders a continuous set of bars representing the past periods accurately scaled starting from the exact current time backwards. (Empty periods should be clearly visible as empty spaces/zero bars).
