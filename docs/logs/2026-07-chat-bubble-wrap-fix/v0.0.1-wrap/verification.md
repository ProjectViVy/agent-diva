## Verification
- Unit Tests: \just test\ passes.
- UI Testing: Verified that the \width: max-content\ allows the chat bubble to grow horizontally to properly fit short strings like '你好！😺 有什么可以帮你的？' onto a single line without wrapping prematurely, while \max-width: 100%\ ensures it will still wrap when it hits the parent container boundary.
