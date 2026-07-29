# GMH-21 Release

This is a library-contract and isolated-artifact release. It performs no
automatic migration, does not register a new provider, and does not modify
legacy or Laputa authority files.

Rollback is a normal revert of the focused commit. Any explicitly generated
test artifact can be removed through its rollback manifest without touching
source data.
