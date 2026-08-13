# GUI chat layout fix

## Scope

- Stabilized the thought-card header so copy and expand controls remain an independent fixed action group, including for a single short thought.
- Made chat bubbles content-sized with a responsive minimum width and safe long-token wrapping.

## Impact

Short messages no longer collapse into an unnecessarily narrow bubble, while longer messages and unbroken content remain constrained by the existing maximum width.
