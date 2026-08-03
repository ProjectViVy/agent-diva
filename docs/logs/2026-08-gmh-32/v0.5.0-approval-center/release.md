# Release and rollback

This story is source-only and is not deployed or pushed. It does not start the desktop application or access a user profile.

Revert the focused GMH-32 commit to remove the badge, drawer, unified inline cards, presentation additions, and GUI event state. The GMH-31 HTTP/SSE/Tauri contract and all legacy domain cards/endpoints remain available after that rollback; no governance database migration or user-data rollback is required.

GMH-33 may rely on the same typed reason codes and unified routes, but must not reuse GUI cache state or introduce a second approval authority.
