## Verification
- Unit Tests: Frontend unit tests (\
pm run test\ in \gent-diva-gui\) pass successfully.
- UI Testing: Verified that the \width: max-content\ allows the chat bubble to grow horizontally to properly fit short strings like '你好！😺 有什么可以帮你的？' onto a single line without wrapping prematurely. Also verified that removing \min-width: 10rem\ eliminates the wasted blank space for very short messages like '你好', allowing the user bubble to shrink tightly around the content while remaining aligned to the right side of the chat container.
