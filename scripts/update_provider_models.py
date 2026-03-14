#!/usr/bin/env python3
"""Developer utility to refresh static provider model catalogs in providers.yaml.

This script is intentionally manual. It fetches live model ids from selected
OpenAI-compatible providers and replaces the `models:` block in
`agent-diva-providers/src/providers.yaml`.
"""

from __future__ import annotations

import argparse
import json
import os
import sys
import textwrap
import urllib.error
import urllib.request
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable


REPO_ROOT = Path(__file__).resolve().parents[1]
PROVIDERS_YAML = REPO_ROOT / "agent-diva-providers" / "src" / "providers.yaml"


@dataclass(frozen=True)
class ProviderUpdater:
    name: str
    api_base: str
    env_key: str | None = None
    needs_bearer: bool = True

    @property
    def models_url(self) -> str:
        return f"{self.api_base.rstrip('/')}/models"


SUPPORTED_UPDATERS: dict[str, ProviderUpdater] = {
    "openrouter": ProviderUpdater(
        name="openrouter",
        api_base="https://openrouter.ai/api/v1",
        env_key=None,
        needs_bearer=False,
    ),
    "openai": ProviderUpdater(
        name="openai",
        api_base="https://api.openai.com/v1",
        env_key="OPENAI_API_KEY",
    ),
    "deepseek": ProviderUpdater(
        name="deepseek",
        api_base="https://api.deepseek.com/v1",
        env_key="DEEPSEEK_API_KEY",
    ),
    "groq": ProviderUpdater(
        name="groq",
        api_base="https://api.groq.com/openai/v1",
        env_key="GROQ_API_KEY",
    ),
    "aihubmix": ProviderUpdater(
        name="aihubmix",
        api_base="https://aihubmix.com/v1",
        env_key="OPENAI_API_KEY",
    ),
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Refresh static model lists in agent-diva-providers/src/providers.yaml",
    )
    parser.add_argument(
        "--providers",
        nargs="+",
        default=["openrouter"],
        help="Providers to refresh. Supported: %(choices)s",
        choices=sorted(SUPPORTED_UPDATERS.keys()),
    )
    parser.add_argument(
        "--file",
        type=Path,
        default=PROVIDERS_YAML,
        help="Path to providers.yaml",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Print planned changes without writing the file",
    )
    parser.add_argument(
        "--sort",
        action="store_true",
        help="Sort model ids alphabetically before writing",
    )
    parser.add_argument(
        "--keep-existing",
        action="store_true",
        help="Merge new models with existing models instead of replacing the block",
    )
    parser.add_argument(
        "--verbose",
        action="store_true",
        help="Print request and replacement details",
    )
    return parser.parse_args()


def fetch_models(updater: ProviderUpdater, verbose: bool) -> list[str]:
    headers = {
        "Accept": "application/json",
        "User-Agent": "agent-diva-dev-tool/1.0",
    }
    if updater.env_key:
        api_key = os.getenv(updater.env_key, "").strip()
        if not api_key:
            raise RuntimeError(
                f"{updater.name} requires env var {updater.env_key} to fetch live models"
            )
        if updater.needs_bearer:
            headers["Authorization"] = f"Bearer {api_key}"
        else:
            headers["Authorization"] = f"Bearer {api_key}"

    request = urllib.request.Request(updater.models_url, headers=headers, method="GET")
    if verbose:
        print(f"[fetch] {updater.name}: {updater.models_url}")
    try:
        with urllib.request.urlopen(request, timeout=20) as response:
            payload = json.load(response)
    except urllib.error.HTTPError as exc:
        detail = exc.read().decode("utf-8", errors="replace").strip()
        raise RuntimeError(
            f"{updater.name} returned HTTP {exc.code}: {truncate(detail, 200)}"
        ) from exc
    except urllib.error.URLError as exc:
        raise RuntimeError(f"{updater.name} request failed: {exc}") from exc

    data = payload.get("data")
    if not isinstance(data, list):
        raise RuntimeError(f"{updater.name} response missing JSON array field 'data'")

    models: list[str] = []
    for item in data:
        if not isinstance(item, dict):
            continue
        model_id = str(item.get("id", "")).strip()
        if model_id:
            models.append(model_id)

    deduped = list(dict.fromkeys(models))
    if not deduped:
        raise RuntimeError(f"{updater.name} returned no model ids")
    return deduped


def truncate(value: str, limit: int) -> str:
    if len(value) <= limit:
        return value
    return value[:limit] + "..."


def split_provider_blocks(lines: list[str]) -> list[tuple[str, int, int]]:
    blocks: list[tuple[str, int, int]] = []
    current_name: str | None = None
    current_start: int | None = None

    for index, line in enumerate(lines):
        if line.startswith("- name: "):
            if current_name is not None and current_start is not None:
                blocks.append((current_name, current_start, index))
            current_name = line[len("- name: ") :].strip()
            current_start = index

    if current_name is not None and current_start is not None:
        blocks.append((current_name, current_start, len(lines)))
    return blocks


def extract_models(block: list[str]) -> list[str]:
    models: list[str] = []
    start_index = None
    for index, line in enumerate(block):
        if line.startswith("  models:"):
            start_index = index + 1
            break
    if start_index is None:
        return models

    for line in block[start_index:]:
        if not line.startswith("    - "):
            break
        models.append(line[len("    - ") :].rstrip("\n"))
    return models


def replace_models_block(block: list[str], models: Iterable[str]) -> list[str]:
    replacement = ["  models:\n"] + [f"    - {model}\n" for model in models]

    start_index = None
    end_index = None
    for index, line in enumerate(block):
        if line.startswith("  models:"):
            start_index = index
            end_index = index + 1
            while end_index < len(block) and block[end_index].startswith("    - "):
                end_index += 1
            break

    if start_index is None:
        insert_at = next(
            (index for index, line in enumerate(block) if line.startswith("  model_overrides:")),
            len(block),
        )
        return block[:insert_at] + replacement + block[insert_at:]

    return block[:start_index] + replacement + block[end_index:]


def update_yaml_text(
    source_text: str,
    provider_name: str,
    models: list[str],
    keep_existing: bool,
) -> tuple[str, list[str], list[str]]:
    lines = source_text.splitlines(keepends=True)
    for name, start, end in split_provider_blocks(lines):
        if name != provider_name:
            continue

        block = lines[start:end]
        existing = extract_models(block)
        final_models = models
        if keep_existing:
            final_models = list(dict.fromkeys(existing + models))
        updated_block = replace_models_block(block, final_models)
        updated_lines = lines[:start] + updated_block + lines[end:]
        return "".join(updated_lines), existing, final_models

    raise RuntimeError(f"Provider '{provider_name}' not found in {PROVIDERS_YAML}")


def main() -> int:
    args = parse_args()
    yaml_path = args.file.resolve()
    source_text = yaml_path.read_text(encoding="utf-8")
    updated_text = source_text

    for provider_name in args.providers:
        updater = SUPPORTED_UPDATERS[provider_name]
        live_models = fetch_models(updater, args.verbose)
        if args.sort:
            live_models = sorted(live_models)
        updated_text, existing, final_models = update_yaml_text(
            updated_text,
            provider_name=provider_name,
            models=live_models,
            keep_existing=args.keep_existing,
        )
        print(
            f"[update] {provider_name}: existing={len(existing)} fetched={len(live_models)} final={len(final_models)}"
        )
        if args.verbose:
            print(textwrap.indent("\n".join(final_models[:20]), prefix="  "))
            if len(final_models) > 20:
                print(f"  ... ({len(final_models) - 20} more)")

    if updated_text == source_text:
        print("[result] no changes")
        return 0

    if args.dry_run:
        print("[result] dry-run only, providers.yaml not written")
        return 0

    yaml_path.write_text(updated_text, encoding="utf-8", newline="")
    print(f"[result] wrote {yaml_path}")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except RuntimeError as exc:
        print(f"[error] {exc}", file=sys.stderr)
        raise SystemExit(1)
