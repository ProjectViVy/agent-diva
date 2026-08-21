# v0.9.9 Sandbox Linux Compatibility Release

## Release method

The implementation commits are `164fdb4e`, `6dcca721`, and `76716b3a`. The existing `v0.9.9` tag is intentionally force-updated, as previously authorized, to the final documentation commit for this iteration. The tag is pushed with:

```text
git push origin +refs/tags/v0.9.9
```

Only the tag is pushed; the `dev` branch is not pushed by this iteration.

## Release gate

The tag-triggered GitHub Action must pass its Ubuntu, macOS, and Windows jobs. If a platform reports another source-level failure, the tag is not accepted and the failure is fixed before the next retag.
