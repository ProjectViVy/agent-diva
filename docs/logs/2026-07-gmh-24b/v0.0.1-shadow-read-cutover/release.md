# GMH-24B Release

`legacy` remains the compatibility default for configurations that omit
`memory.authority_mode`. Operators may explicitly select `shadow` after
running GMH-24A import. `typed` enables production Recall v2 reads but does not
yet authorize typed production writes.

Mode changes require a runtime restart. A degraded shadow/typed health result
must be corrected or rolled back by configuration; runtime does not silently
select another authority.

No push or deployment is performed.
