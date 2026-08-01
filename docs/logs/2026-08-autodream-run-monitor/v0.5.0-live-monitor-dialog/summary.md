# AutoDream live monitor dialog

Evolution run cards now provide a **Monitor** action. It opens a read-only dialog that polls the selected run once per second while it is active and shows its current phase, state, attempt, and persisted safe progress events.

The Manager exposes a bounded per-run event timeline (maximum 200 events). The provider never supplies authority fields to this view, and the dialog cannot approve proposals or write Memory.
