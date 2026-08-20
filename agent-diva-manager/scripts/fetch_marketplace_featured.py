#!/usr/bin/env python3
"""Fetch the skills.sh featured leaderboard into an offline YAML snapshot.

The gateway embeds `agent-diva-manager/data/marketplace_featured.yaml` and
serves it from `/api/skills/marketplace/featured` without any runtime
network dependency. Re-run this script to refresh the snapshot.

Modes:
  * With a token (env AGENT_DIVA_SKILLS_MARKETPLACE_TOKEN, --token, or
    --token-file): calls the official leaderboard API
    `GET /api/v1/skills?view=all-time|trending|hot` using a Vercel OIDC
    bearer token.
  * Without a token: falls back to scraping the server-rendered homepage
    leaderboard (no authentication required, weekly-installs based).

Stdlib only; no third-party dependencies.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import sys
import urllib.error
import urllib.request
from datetime import datetime, timezone

DEFAULT_BASE_URL = "https://skills.sh"
DEFAULT_OUTPUT = os.path.join(
    os.path.dirname(os.path.abspath(__file__)),
    "..",
    "data",
    "marketplace_featured.yaml",
)
TOKEN_ENV = "AGENT_DIVA_SKILLS_MARKETPLACE_TOKEN"
USER_AGENT = "agent-diva-marketplace-snapshot/1.0"

SKILL_HREF_RE = re.compile(
    r'href="/([A-Za-z0-9_.\-]+/[A-Za-z0-9_.\-]+/[A-Za-z0-9_.\-]+)"'
)
WEEKLY_RE = re.compile(r'aria-label="Weekly installs: ([^"]+)"')


def http_get(url: str, headers: dict[str, str], timeout: int = 30) -> str:
    request = urllib.request.Request(url, headers={"User-Agent": USER_AGENT, **headers})
    with urllib.request.urlopen(request, timeout=timeout) as response:
        return response.read().decode("utf-8")


def fetch_api(base_url: str, token: str, view: str, limit: int) -> list[dict]:
    """Official leaderboard API (requires a Vercel OIDC bearer token)."""
    skills: list[dict] = []
    page = 0
    while len(skills) < limit:
        per_page = min(limit - len(skills), 500)
        url = f"{base_url}/api/v1/skills?view={view}&page={page}&per_page={per_page}"
        body = json.loads(http_get(url, {"Authorization": f"Bearer {token}"}))
        data = body.get("data", [])
        for entry in data:
            skill_id = str(entry.get("id", ""))
            if not skill_id:
                continue
            source = str(entry.get("source", ""))
            slug = skill_id.split("/")[-1]
            skills.append(
                {
                    "id": skill_id,
                    "name": str(entry.get("name") or entry.get("slug") or slug),
                    "source": source or "/".join(skill_id.split("/")[:2]),
                    "installs": int(entry.get("installs") or 0),
                }
            )
        pagination = body.get("pagination", {})
        if not pagination.get("hasMore") or not data:
            break
        page += 1
    return skills[:limit]


def fetch_web(base_url: str, limit: int) -> list[dict]:
    """Fallback: scrape the server-rendered homepage leaderboard.

    Each leaderboard row pairs a skill detail link with a sparkline whose
    aria-label carries the last eight weekly install counts.
    """
    html = http_get(f"{base_url}/", {})
    hrefs = [(m.start(), m.group(1)) for m in SKILL_HREF_RE.finditer(html)]
    skills: list[dict] = []
    seen: set[str] = set()
    for weekly_match in WEEKLY_RE.finditer(html):
        owner_href = None
        for position, href in hrefs:
            if position < weekly_match.start():
                owner_href = href
            else:
                break
        if not owner_href or owner_href in seen:
            continue
        seen.add(owner_href)
        weekly = [int(n.replace(",", "")) for n in re.findall(r"[\d,]+", weekly_match.group(1))]
        owner, repo, slug = owner_href.split("/", 2)
        skills.append(
            {
                "id": owner_href,
                "name": slug,
                "source": f"{owner}/{repo}",
                "installs": sum(weekly),
            }
        )
        if len(skills) >= limit:
            break
    return skills


def yaml_quote(value: str) -> str:
    return "'" + value.replace("'", "''") + "'"


def write_yaml(path: str, source: str, metric: str, skills: list[dict]) -> None:
    generated_at = datetime.now(timezone.utc).isoformat(timespec="seconds")
    lines = [
        "# skills.sh featured leaderboard snapshot (offline data for the",
        "# gateway's /api/skills/marketplace/featured route). Regenerate:",
        "#   python agent-diva-manager/scripts/fetch_marketplace_featured.py",
        f"generated_at: {yaml_quote(generated_at)}",
        f"source: {yaml_quote(source)}",
        f"metric: {yaml_quote(metric)}",
        "skills:",
    ]
    for skill in skills:
        lines.append(f"  - id: {yaml_quote(skill['id'])}")
        lines.append(f"    name: {yaml_quote(skill['name'])}")
        lines.append(f"    source: {yaml_quote(skill['source'])}")
        lines.append(f"    installs: {skill['installs']}")
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8", newline="\n") as handle:
        handle.write("\n".join(lines) + "\n")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--view", choices=["all-time", "trending", "hot"], default="all-time")
    parser.add_argument("--limit", type=int, default=100)
    parser.add_argument("--base-url", default=os.environ.get("AGENT_DIVA_SKILLS_MARKETPLACE_URL", DEFAULT_BASE_URL))
    parser.add_argument("--token", default=os.environ.get(TOKEN_ENV, ""))
    parser.add_argument("--token-file", default="", help="path to a file containing the bearer token")
    parser.add_argument("--output", default=DEFAULT_OUTPUT)
    args = parser.parse_args()

    token = args.token
    if not token and args.token_file:
        with open(args.token_file, "r", encoding="utf-8") as handle:
            token = handle.read().strip()

    base_url = args.base_url.rstrip("/")
    try:
        if token:
            skills = fetch_api(base_url, token, args.view, args.limit)
            source = f"api:v1:{args.view}"
            metric = "installs"
        else:
            skills = fetch_web(base_url, args.limit)
            source = "web:leaderboard"
            metric = "weekly_installs_sum"
    except (urllib.error.URLError, urllib.error.HTTPError, ValueError, OSError) as error:
        print(f"error: failed to fetch leaderboard: {error}", file=sys.stderr)
        return 1

    if not skills:
        print("error: leaderboard returned no skills", file=sys.stderr)
        return 1

    output = os.path.abspath(args.output)
    write_yaml(output, source, metric, skills)
    print(f"wrote {len(skills)} featured skills ({source}) -> {output}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
