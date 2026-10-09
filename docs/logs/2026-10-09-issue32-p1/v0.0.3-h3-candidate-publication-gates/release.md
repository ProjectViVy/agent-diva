# Release record

- No public release, asset upload, tag creation, push, or mainline merge was performed.
- Tag pushes now build internal Linux/Windows candidates only.
- Public upload is manual and requires strict complete v2 acceptance, an annotated DIVA product tag whose peeled commit equals the retained host source, both pinned annotated archive tags, an approved W6 state, and a second hash/tag validation after downloading the retained bytes.
- Publish does not rebuild, sign, create a missing release, use `--clobber`, or change refs.
- GitHub Actions and remote refs were not executed or changed from this workspace.
