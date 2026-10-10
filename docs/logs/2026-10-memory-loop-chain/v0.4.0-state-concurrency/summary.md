# Shared cognitive state concurrency

VIVY `114ad97a` fixes three reproduced lost-update paths: same policy base revision accepted twice, accepted source high-watermark erased, and admitted native workflow identity/window erased. Admission, policy and input updates now serialize bounded transactions on the existing Service and save with the original snapshot CAS version. Model/workflow execution remains asynchronous on the existing native Eino/INOFY path.

Three RED cases each pass ten repetitions; four concurrency cases pass three race-enabled repetitions. Focused runtime85 and actual diagnostic DIVA composition12 pass with zero skips and exit0. Raw counts, hashes and exact source identities are in checkpoint.json. Prior rejected-update/SQLite path repairs remain in v0.3.

Formal Story states are unchanged. Continuous repair remains the active goal; only Windows and real-provider resources are external waits. Same-operation recovery, source/activity/recall/lifecycle/isolation and final source sealing still require local work.
