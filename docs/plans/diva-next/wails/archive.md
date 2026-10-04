# Paired Tauri/C ABI source archive

Freeze date: **2026-10-04**. These references preserve the last merged
Tauri/C ABI source line before the approved Wails/Go migration. The archive
branches were created at exact commits; mainline code is unchanged by this
documentation publication.

| Repository | Frozen commit | Archive branch | Requested annotated tag |
| --- | --- | --- | --- |
| [agent-diva](https://github.com/ProjectViVy/agent-diva) | `5444795a2d9db31e158c2cf009d64697e6289e50` | `archive/tauri-cabi` | `archive/tauri-cabi-20261004` |
| [agent-vivy](https://github.com/ProjectViVy/agent-vivy) | `fc559e6b03ce4e65c0099b9745855dccc4fb067e` | `archive/tauri-cabi` | `archive/tauri-cabi-20261004` |

**Tag status: pending creation.** The connected GitHub tool supports branch,
tree, commit and branch-ref writes, but exposes no tag creation operation.
The local Git transport has no authenticated credentials. No tool endpoint
or branch-name workaround is used to circumvent that limit. Completing tags
requires an authorized tag-capable Git transport or supported GitHub surface.
The requested tag must resolve to the frozen commit, not the new docs commit.
W6 deletion is blocked until both tags are verified.

New documents are published only on `docs/wails-migration-20261004` in each
repository. Never commit planning updates onto the archive branches, move
existing archive refs, force-update main or publish an unrelated release to
create a tag. A later emergency legacy fix gets a distinct ref.

## What is preserved

The source freeze includes DIVA #17 and VIVY #29 merged closure work:
Tauri host, Go shared-library bridge, speech Rust implementation, selected
cognitive/runtime composition, frontend consumers and recorded fixtures.
It excludes unmerged work, including DIVA `fix/windows-speech-shell`.
Existing branches remain untouched.

DIVA's tracked `src-tauri/vivy-runtime/generation.json` records Generation
`331bb89d975770edb9255e49c6d0e8496ed99f77a510bcb497733ec54d7e01f2`,
spec `vivy.module/v1`, with 21 modules and 20 cognitive actions. The historical
packaged fixture reports a Linux shared-library candidate:

| Recorded item | Value / scope |
| --- | --- |
| Binary | vivy-shared.so, Linux x64, 107,489,640 bytes |
| SHA-256 | `d0155e26fddbb4426f7ddc2ab82615980a174d002bfc57d84c9386eac03659e5` |
| VIVY candidate source | `db7f0c558d4e8fff52d044333cce6c81c9a210b2` |
| Laputa candidate source | `04b89324306099f913e8ab9d53195fb14fba3a16` |
| INOFY candidate source | `71e2c9bbe47d5d095b27d756e38eaff565d93d36` |
| Evidence | closure-build-inputs.json, closure-packaged-obs.json, closure-speech.json |
| Provider coverage | Local scripted provider in the packaged OBS probe; real installed-product acceptance is separate |

The fixture pins precede the merged main source freezes; do not conflate
them. The binary files are gitignored and were not available as tracked
source or as a corresponding current release asset in the inspected release
list. **This is a complete source-reference freeze, not preservation or
reverification of that executable.** Obtain a surviving original binary from
its engineering owner and verify its recorded checksum before claiming a
binary archive. Rebuilding generates a new artifact identity and evidence.

Known recorded limitations remain visible: fresh FrozenCore initialization,
embedded logging/rotation, process-local session authorization and explicit
quit cancellation semantics. Old matrices are scoped evidence, not a
known-good Windows rollback guarantee.

## Exact tag operation when tag-capable access is available

From authenticated local checkouts, first fetch the frozen commit and verify
the archive branch matches the table. Refuse an existing mismatched tag;
never use force. Then create/push only the explicit tag ref.

DIVA:

```sh
git fetch origin main archive/tauri-cabi
git cat-file -e 5444795a2d9db31e158c2cf009d64697e6289e50^{commit}
git tag -a archive/tauri-cabi-20261004 5444795a2d9db31e158c2cf009d64697e6289e50 \
  -m "Archive paired Tauri/C ABI source baseline before Wails migration; agent-diva 5444795a2d9db31e158c2cf009d64697e6289e50; agent-vivy fc559e6b03ce4e65c0099b9745855dccc4fb067e"
git push origin refs/tags/archive/tauri-cabi-20261004
git ls-remote origin refs/heads/archive/tauri-cabi refs/tags/archive/tauri-cabi-20261004 'refs/tags/archive/tauri-cabi-20261004^{}'
```

VIVY:

```sh
git fetch origin main archive/tauri-cabi
git cat-file -e fc559e6b03ce4e65c0099b9745855dccc4fb067e^{commit}
git tag -a archive/tauri-cabi-20261004 fc559e6b03ce4e65c0099b9745855dccc4fb067e \
  -m "Archive paired Tauri/C ABI source baseline before Wails migration; agent-diva 5444795a2d9db31e158c2cf009d64697e6289e50; agent-vivy fc559e6b03ce4e65c0099b9745855dccc4fb067e"
git push origin refs/tags/archive/tauri-cabi-20261004
git ls-remote origin refs/heads/archive/tauri-cabi refs/tags/archive/tauri-cabi-20261004 'refs/tags/archive/tauri-cabi-20261004^{}'
```

Annotated tag object SHA differs from its commit; verify the peeled
`^{}` commit against the table. Record the successful remote readback and
update the manifest's tag statuses only then.

## Recovery boundary

Use the old paired source refs in an isolated checkout for investigation or
a separately authorized rollback build. Do not reset main history, overwrite
the new state root, reuse a newer database with an older binary, or delete
old user state. Historical import remains cancelled. Release binaries and
state compatibility require their own evidence; source refs alone do not
provide either.

