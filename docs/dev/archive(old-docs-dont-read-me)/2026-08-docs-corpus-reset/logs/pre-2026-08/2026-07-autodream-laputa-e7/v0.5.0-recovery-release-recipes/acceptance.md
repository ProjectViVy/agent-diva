# Acceptance

Recovery acceptance requires every injected failure to preserve the previous
authority or recover exactly one committed result, with no duplicate
changelog/audit/store revision and with verified rollback integrity.

All focused recovery drills pass. Human desktop workflows remain deferred until
the complete automated E7 gate passes.
