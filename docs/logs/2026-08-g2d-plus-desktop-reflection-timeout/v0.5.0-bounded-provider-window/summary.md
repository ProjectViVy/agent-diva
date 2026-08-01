# Bounded reflection provider window

AutoDream reflection now allows 90 seconds for a provider response instead of 45 seconds. Its response budget is reduced from 2,048 to 1,024 tokens because proposals are bounded, review-only candidates and do not need long-form output.

This targets the observed timeout after the provider request successfully passed the schema adapter.
