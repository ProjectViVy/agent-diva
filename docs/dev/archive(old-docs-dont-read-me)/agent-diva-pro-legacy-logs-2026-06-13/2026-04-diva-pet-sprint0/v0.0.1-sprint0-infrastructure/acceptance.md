# Acceptance — Sprint 0

## From User/Product Perspective

### Given / When / Then

1. **Sidebar entry visibility**
   - Given: pet.enabled is true
   - When: User opens Diva GUI
   - Then: "Diva Pet" sidebar button is visible with cat icon

2. **Sidebar entry hidden**
   - Given: pet.enabled is false
   - When: User opens Diva GUI
   - Then: "Diva Pet" sidebar button is hidden

3. **Message synchronization (DivaPet → ChatView)**
   - Given: User is on DivaPet view
   - When: User types "Hello" and sends
   - Then: Message appears in DivaPet message list; switching to Chat tab shows the same message

4. **Message synchronization (ChatView → DivaPet)**
   - Given: User is on Chat view
   - When: User types a message and gets agent reply
   - Then: Switching to DivaPet tab shows both messages

5. **Typing indicator**
   - Given: Agent is generating a response
   - When: User switches to DivaPet view
   - Then: "Thinking..." spinner is visible until response is complete

6. **No impact on existing features**
   - Given: Diva Pet module exists
   - When: User uses Chat, Settings, or Console tabs
   - Then: All existing functionality works identically

## Edge Cases Verified

- [x] Rapid tab switching between Chat and Pet — no message loss
- [x] Empty message list — graceful empty state
- [x] Disabled input while agent is typing
- [x] localStorage corruption — fallback to DEFAULT_PET_CONFIG
