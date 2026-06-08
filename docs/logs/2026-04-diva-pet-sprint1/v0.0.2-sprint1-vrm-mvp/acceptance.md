# Acceptance — Sprint 1

## From User/Product Perspective

### Given / When / Then

1. **VRM character rendering**
   - Given: User navigates to Diva Pet tab
   - When: VRM model finishes loading
   - Then: 3D character appears in the upper area, rotates with mouse drag, zooms with scroll

2. **Happy expression**
   - Given: VRM character is loaded
   - When: Agent reply contains "哈哈" or "happy"
   - Then: Character's face shows happy expression; mood badge shows "😊 happy"

3. **Sad expression**
   - Given: VRM character is loaded
   - When: Agent reply contains "难过" or "sorry"
   - Then: Character's face shows sad expression; mood badge shows "😢 sad"

4. **Angry expression**
   - Given: VRM character is loaded
   - When: Agent reply contains "生气" or "damn"
   - Then: Character's face shows angry expression; mood badge shows "😠 angry"

5. **Surprised expression**
   - Given: VRM character is loaded
   - When: Agent reply contains "哇" or "wow"
   - Then: Character's face shows surprised expression; mood badge shows "😲 surprised"

6. **Neutral expression (default)**
   - Given: VRM character is loaded
   - When: Agent reply contains no mood keywords
   - Then: Character returns to neutral expression; no mood badge shown

7. **Model load error**
   - Given: VRM model path is invalid
   - When: Component attempts to load
   - Then: Error overlay appears with retry button

8. **Message sync preserved**
   - Given: Messages exist in session
   - When: User switches between Chat and Pet tabs
   - Then: Messages are consistent in both views

9. **No impact on existing features**
   - Given: Diva Pet VRM module exists
   - When: User uses Chat, Settings, or Console tabs
   - Then: All existing functionality works identically

## Edge Cases Verified

- [x] Model loading while switching tabs → cleanup on unmount
- [x] Empty messages → VRM renders with neutral expression
- [x] Multiple rapid mood changes → latest agent reply takes priority
- [x] WebGL context loss → handled by Three.js defaults
