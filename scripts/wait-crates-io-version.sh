#!/usr/bin/env bash
# Poll crates.io until a given crate version appears in the registry API (post-publish indexing).
set -euo pipefail
: "${CRATE:?set CRATE to the crate name}"
: "${VERSION:?set VERSION to the semver string}"

UA="agent-diva-release-ci (https://github.com/ProjectViVy/agent-diva)"
MAX_ATTEMPTS="${WAIT_CRATES_IO_MAX_ATTEMPTS:-60}"
SLEEP_SECS="${WAIT_CRATES_IO_SLEEP_SECS:-10}"

for i in $(seq 1 "${MAX_ATTEMPTS}"); do
  if curl -sfS -H "User-Agent: ${UA}" \
    "https://crates.io/api/v1/crates/${CRATE}" \
    | jq -e --arg v "${VERSION}" '.versions[] | select(.num == $v)' >/dev/null 2>&1; then
    echo "crate ${CRATE} ${VERSION} is visible on crates.io"
    exit 0
  fi
  echo "wait-crates-io: attempt ${i}/${MAX_ATTEMPTS} — ${CRATE} ${VERSION} not yet visible, sleeping ${SLEEP_SECS}s..."
  sleep "${SLEEP_SECS}"
done

echo "timeout waiting for ${CRATE} ${VERSION} on crates.io"
exit 1
