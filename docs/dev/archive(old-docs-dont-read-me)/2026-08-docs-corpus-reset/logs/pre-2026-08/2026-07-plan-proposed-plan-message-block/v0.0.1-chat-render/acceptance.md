# Acceptance

1. Load or produce a chat message containing:

   ```text
   前言…
   <proposed_plan>
   …
   </proposed_plan>
   ```

2. **Expected**
   - Preface renders as normal chat markdown
   - Plan body renders inside a blue **计划 / Plan** card (no visible tags)
   - Bare section names like `目标` appear as plan headings

3. **Reject if** raw `<proposed_plan>` tags remain visible in the bubble.
