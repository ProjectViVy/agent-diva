# Summary

Updated the prepared-journal recovery fixture to create a currently valid
governance request and receipt instead of relying on timestamps that expired
before the test ran. The scenario now also replays the recovered apply and
proves that no second changelog entry or side effect is produced.
