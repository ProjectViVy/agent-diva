# Acceptance

1. Start with a fresh temporary workspace and open Laputa storage.
2. Confirm `.laputa/cognitive/MEMRULES.MD` exists.
3. Confirm `.laputa/cognitive/WORLD.MD` does not exist.
4. Confirm the four Frozen Core seed files (`identity.json`, `relationship.json`,
   `commitment.json`, `preferences.json`) do not exist.
5. Add custom WORLD and identity files, reopen storage, and confirm both contents
   are unchanged.
6. Confirm the Frozen Core snapshot is empty while those section files are absent.
7. Confirm subsequent S2/S3/S4 work is still separately gated and no onboarding or
   Persona state transition was introduced.
