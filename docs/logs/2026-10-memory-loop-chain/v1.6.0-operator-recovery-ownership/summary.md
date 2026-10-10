# Operator recovery ownership fence

The public background/recover action used startup Service.Recover while an owned primary was executing. A real held model response reproduced premature terminal failure. The handler now uses the existing Service.RecoverBackground ownership check and existing conflict mapping. Busy recovery is refused; idle recovery succeeds without recapturing or rerunning the original input.

No new runtime, recovery algorithm, storage schema or dependency. Startup recovery remains the startup path. Formal crash matrix and final product candidate gates are still pending.
