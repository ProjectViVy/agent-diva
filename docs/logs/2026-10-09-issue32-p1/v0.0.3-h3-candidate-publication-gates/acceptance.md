# Acceptance boundary

This iteration accepts the **local gate implementation** only:

- a complete synthetic v2 candidate passes;
- incomplete or mutated reports/candidate files reject;
- the workflow has separate candidate, acceptance and publish jobs, and publish depends on successful aggregate acceptance;
- external acceptance and candidate run references are immutable/pinned, and the workflow revalidates immediately before upload;
- historical v1 evidence does not promote.

This does not claim Linux/Windows build success, W5 installed-product behavior, real-provider/audio behavior, archive-tag completion, W6 approval, or release acceptance. The checked-in v2 `.example.json` deliberately has pending W5 rows and pending W6 state with placeholder hashes. It must never be used as a release report.

The workflow remains fail-closed until the full native/product matrix, the two exact annotated archive tag peels, and an approved W6 state are recorded in a reviewed full-SHA acceptance ref. No public action was performed in this iteration.
