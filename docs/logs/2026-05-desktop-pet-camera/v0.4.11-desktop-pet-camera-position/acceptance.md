# Acceptance

## User Steps

1. Open Diva desktop pet mode.
2. Confirm the default avatar view faces the character directly.
3. Confirm the character is not biased toward the left or top of the desktop pet window.
4. Confirm the horizontal center is close after applying the explicit `targetX=-0.12` baseline.
5. Hide/show the desktop pet or trigger render pause/resume, then confirm the same centered view is retained.
6. Use wheel zoom and drag rotation to confirm interactive controls still work.

## Expected Result

The desktop pet starts from and returns to a centered, front-facing camera view while preserving existing interaction behavior.
