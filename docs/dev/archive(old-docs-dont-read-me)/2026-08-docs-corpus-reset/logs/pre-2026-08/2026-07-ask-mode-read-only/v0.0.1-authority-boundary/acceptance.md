# Ask Mode Read-only Boundary Acceptance

- [x] Ask is classified independently from Agent and Plan.
- [x] Unknown explicit modes fail closed to Ask.
- [x] Ask cannot inherit or start an approved execution.
- [x] Only read file, list directory, read attachment, web search, and web
  fetch definitions are visible to the model.
- [x] Forged mutating tool calls are denied before executor entry.
- [ ] Real desktop command-execution attempt is denied.
- [ ] Real desktop disposable-file deletion attempt is denied and the file
  remains present.
- [ ] Switching back to Agent mode reaches the normal command approval path.
