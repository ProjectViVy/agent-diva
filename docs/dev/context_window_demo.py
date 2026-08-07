#!/usr/bin/env python3
"""
Context Window Manager Demo for agent-diva-pro
===============================================

Demonstrates a multi-layer context window resolution strategy
inspired by Claude Code, OpenFang, and Hermes.

Features:
- 4-tier priority resolution (config -> hardcoded -> models.dev -> default)
- JSON file caching with TTL
- models.dev community registry integration
- Alias resolution
- Unknown model fallback with probing tiers

Usage:
    python context_window_demo.py
"""

from __future__ import annotations

import json
import os
import re
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Optional

# =============================================================================
# Configuration
# =============================================================================

CACHE_DIR = Path.home() / ".agent-diva" / "cache"
CACHE_FILE = CACHE_DIR / "model-capabilities.json"
MODELS_DEV_URL = "https://models.dev/api.json"
CACHE_TTL_SECONDS = 3600  # 1 hour
DEFAULT_FALLBACK_CONTEXT = 128_000
MINIMUM_CONTEXT_LENGTH = 64_000

# Probing tiers for unknown models (descending)
CONTEXT_PROBE_TIERS = [256_000, 128_000, 64_000, 32_000]


# =============================================================================
# Data Structures
# =============================================================================

@dataclass
class ModelCapability:
    """Model capability record — mirrors agent-diva-pro's ModelCapabilities in Rust."""
    model_id: str
    context_window: int
    max_output_tokens: int = 8_192
    supports_vision: bool = False
    supports_tools: bool = False
    supports_streaming: bool = True
    aliases: list[str] = field(default_factory=list)
    # Source tracking for debugging
    source: str = "unknown"
    cached_at: float = field(default_factory=time.time)

    def is_fresh(self, ttl: float = CACHE_TTL_SECONDS) -> bool:
        return (time.time() - self.cached_at) < ttl


@dataclass
class BudgetConfig:
    """Token budget configuration — mirrors agent-diva-pro's BudgetConfig."""
    max_tokens: int = 180_000
    keep_recent: int = 5
    summary_max_chars: int = 2_000

    @classmethod
    def from_model(cls, capability: ModelCapability) -> "BudgetConfig":
        """Create budget config from model capability with safety margin."""
        # Reserve tokens for output + buffer (same logic as Claude Code)
        reserved_for_output = min(capability.max_output_tokens, 20_000)
        buffer_tokens = _calculate_buffer(capability.context_window)
        effective_window = capability.context_window - reserved_for_output - buffer_tokens
        return cls(
            max_tokens=max(32_000, effective_window),  # Never go below 32K
            keep_recent=5,
            summary_max_chars=2_000,
        )


def _calculate_buffer(context_window: int) -> int:
    """Calculate safety buffer based on context window size (Claude Code strategy)."""
    if context_window >= 800_000:
        return 50_000
    if context_window >= 400_000:
        return 30_000
    return 13_000


# =============================================================================
# Tier 1: Hardcoded Model Catalog (OpenFang-style)
# =============================================================================

# tekaapi common models — based on actual usage patterns
HARDCODED_CATALOG: dict[str, ModelCapability] = {
    # Anthropic models via tekaapi
    "claude-sonnet-4-6": ModelCapability(
        model_id="claude-sonnet-4-6",
        context_window=1_000_000,
        max_output_tokens=128_000,
        supports_vision=True,
        supports_tools=True,
        aliases=["sonnet-4-6", "sonnet", "claude-sonnet"],
        source="hardcoded",
    ),
    "claude-opus-4-7": ModelCapability(
        model_id="claude-opus-4-7",
        context_window=1_000_000,
        max_output_tokens=128_000,
        supports_vision=True,
        supports_tools=True,
        aliases=["opus-4-7", "opus", "claude-opus"],
        source="hardcoded",
    ),
    "claude-haiku-4": ModelCapability(
        model_id="claude-haiku-4",
        context_window=200_000,
        max_output_tokens=64_000,
        supports_vision=True,
        supports_tools=True,
        aliases=["haiku-4", "haiku"],
        source="hardcoded",
    ),
    # OpenAI models via tekaapi
    "gpt-5": ModelCapability(
        model_id="gpt-5",
        context_window=400_000,
        max_output_tokens=32_768,
        supports_vision=True,
        supports_tools=True,
        aliases=["gpt5"],
        source="hardcoded",
    ),
    "gpt-4.1": ModelCapability(
        model_id="gpt-4.1",
        context_window=1_047_576,
        max_output_tokens=32_768,
        supports_vision=True,
        supports_tools=True,
        aliases=["gpt4.1"],
        source="hardcoded",
    ),
    # DeepSeek models via tekaapi
    "deepseek-v4-pro": ModelCapability(
        model_id="deepseek-v4-pro",
        context_window=1_000_000,
        max_output_tokens=8_192,
        supports_vision=False,
        supports_tools=True,
        aliases=["deepseek-v4", "deepseek-pro"],
        source="hardcoded",
    ),
    "deepseek-v4-chat": ModelCapability(
        model_id="deepseek-v4-chat",
        context_window=1_000_000,
        max_output_tokens=8_192,
        supports_vision=False,
        supports_tools=True,
        aliases=["deepseek-chat", "deepseek"],
        source="hardcoded",
    ),
    # Google models via tekaapi
    "gemini-2.5-pro": ModelCapability(
        model_id="gemini-2.5-pro",
        context_window=1_048_576,
        max_output_tokens=65_536,
        supports_vision=True,
        supports_tools=True,
        aliases=["gemini-2.5", "gemini-pro"],
        source="hardcoded",
    ),
    # xAI models via tekaapi
    "grok-4": ModelCapability(
        model_id="grok-4",
        context_window=256_000,
        max_output_tokens=8_192,
        supports_vision=True,
        supports_tools=True,
        aliases=["grok"],
        source="hardcoded",
    ),
    # Qwen models via tekaapi
    "qwen3-30b-a3b": ModelCapability(
        model_id="qwen3-30b-a3b",
        context_window=262_144,
        max_output_tokens=262_144,
        supports_vision=True,
        supports_tools=True,
        aliases=["qwen3", "qwen"],
        source="hardcoded",
    ),
    # MiniMax models via tekaapi
    "minimax-text-01": ModelCapability(
        model_id="minimax-text-01",
        context_window=204_800,
        max_output_tokens=8_192,
        supports_vision=False,
        supports_tools=True,
        aliases=["minimax"],
        source="hardcoded",
    ),
    # Xiaomi MiMo models via tekaapi
    "mimo-7b": ModelCapability(
        model_id="mimo-7b",
        context_window=262_144,
        max_output_tokens=8_192,
        supports_vision=False,
        supports_tools=True,
        aliases=["mimo"],
        source="hardcoded",
    ),
    # Default fallback for unknown tekaapi models
    "default-tekaapi": ModelCapability(
        model_id="default-tekaapi",
        context_window=128_000,
        max_output_tokens=8_192,
        supports_vision=False,
        supports_tools=True,
        aliases=[],
        source="hardcoded-default",
    ),
}


# =============================================================================
# Tier 2: models.dev Registry Integration
# =============================================================================

class ModelsDevRegistry:
    """Community model registry client — fetches from models.dev."""

    def __init__(self, cache_path: Path = CACHE_FILE):
        self.cache_path = cache_path
        self._memory_cache: dict[str, ModelCapability] = {}

    def _load_disk_cache(self) -> dict[str, ModelCapability]:
        """Load cached model capabilities from disk."""
        if not self.cache_path.exists():
            return {}
        try:
            with open(self.cache_path, "r", encoding="utf-8") as f:
                data = json.load(f)
            result = {}
            for model_id, entry in data.items():
                if isinstance(entry, dict) and "context_window" in entry:
                    result[model_id] = ModelCapability(
                        model_id=entry["model_id"],
                        context_window=entry["context_window"],
                        max_output_tokens=entry.get("max_output_tokens", 8_192),
                        supports_vision=entry.get("supports_vision", False),
                        supports_tools=entry.get("supports_tools", False),
                        source="disk-cache",
                        cached_at=entry.get("cached_at", 0),
                    )
            return result
        except (json.JSONDecodeError, KeyError, OSError):
            return {}

    def _save_disk_cache(self, models: dict[str, ModelCapability]) -> None:
        """Save model capabilities to disk cache."""
        self.cache_path.parent.mkdir(parents=True, exist_ok=True)
        data = {
            m.model_id: {
                "model_id": m.model_id,
                "context_window": m.context_window,
                "max_output_tokens": m.max_output_tokens,
                "supports_vision": m.supports_vision,
                "supports_tools": m.supports_tools,
                "cached_at": m.cached_at,
            }
            for m in models.values()
        }
        with open(self.cache_path, "w", encoding="utf-8") as f:
            json.dump(data, f, indent=2, ensure_ascii=False)

    def fetch_models_dev(self) -> dict[str, ModelCapability]:
        """Fetch model registry from models.dev (requires internet)."""
        try:
            import urllib.request
            req = urllib.request.Request(
                MODELS_DEV_URL,
                headers={"User-Agent": "agent-diva-pro/0.1"},
            )
            with urllib.request.urlopen(req, timeout=10) as resp:
                raw = json.loads(resp.read().decode("utf-8"))

            result = {}
            for provider_id, provider_data in raw.items():
                models = provider_data.get("models", {})
                for model_id, model_info in models.items():
                    limit = model_info.get("limit", {})
                    context = limit.get("context", 128_000)
                    output = limit.get("output", 8_192)

                    # Build a clean model ID
                    clean_id = model_id.split("/")[-1] if "/" in model_id else model_id

                    result[clean_id] = ModelCapability(
                        model_id=clean_id,
                        context_window=context,
                        max_output_tokens=output,
                        supports_vision="image" in str(model_info.get("modalities", {})),
                        supports_tools=model_info.get("tool_call", False),
                        source=f"models.dev/{provider_id}",
                    )
            return result
        except Exception as e:
            print(f"[WARN] Failed to fetch models.dev: {e}")
            return {}

    def get_registry(self, use_cache: bool = True) -> dict[str, ModelCapability]:
        """Get model registry with caching logic."""
        if self._memory_cache:
            return self._memory_cache

        # Try disk cache first
        if use_cache:
            disk = self._load_disk_cache()
            if disk:
                # Check if cache is fresh
                sample = next(iter(disk.values()))
                if sample.is_fresh():
                    self._memory_cache = disk
                    return disk

        # Fetch from network
        fetched = self.fetch_models_dev()
        if fetched:
            self._save_disk_cache(fetched)
            self._memory_cache = fetched
            return fetched

        # Fallback to disk even if stale
        if disk:
            self._memory_cache = disk
            return disk

        return {}


# =============================================================================
# Tier 3: Context Window Resolver (The Core)
# =============================================================================

class ContextWindowResolver:
    """
    4-tier priority resolver for model context windows.

    Priority (highest to lowest):
        1. Environment variable override (ANTHROPIC-style)
        2. User config file (HERMES-style)
        3. Hardcoded catalog (OPENFANG-style)
        4. models.dev registry (COMMUNITY)
        5. Default fallback
    """

    def __init__(self):
        self.catalog = HARDCODED_CATALOG
        self.registry = ModelsDevRegistry()
        self._alias_map: dict[str, str] = {}
        self._build_alias_map()

    def _build_alias_map(self) -> None:
        """Build alias -> canonical ID mapping."""
        for model_id, cap in self.catalog.items():
            self._alias_map[model_id.lower()] = model_id
            for alias in cap.aliases:
                self._alias_map[alias.lower()] = model_id

    def _resolve_alias(self, model_name: str) -> str:
        """Resolve alias to canonical model ID."""
        lower = model_name.lower()
        if lower in self._alias_map:
            return self._alias_map[lower]
        # Substring match (longest ID first, like Claude Code)
        matches = [
            (canonical, len(canonical))
            for alias, canonical in self._alias_map.items()
            if lower in alias or alias in lower
        ]
        if matches:
            matches.sort(key=lambda x: -x[1])  # Longest first
            return matches[0][0]
        return model_name

    def resolve(
        self,
        model_name: str,
        env_override: Optional[int] = None,
        config_override: Optional[int] = None,
    ) -> ModelCapability:
        """
        Resolve context window for a model using 4-tier priority.

        Args:
            model_name: Raw model ID or alias (e.g., "sonnet", "claude-sonnet-4-6")
            env_override: Tier 1 — environment variable override
            config_override: Tier 2 — user config override

        Returns:
            ModelCapability with resolved context_window
        """
        # Tier 1: Environment variable (ANTHROPIC-style)
        env_val = env_override or self._get_env_override()
        if env_val:
            return ModelCapability(
                model_id=model_name,
                context_window=env_val,
                source="env-override",
            )

        # Tier 2: User config (HERMES-style)
        config_val = config_override or self._get_config_override(model_name)
        if config_val:
            return ModelCapability(
                model_id=model_name,
                context_window=config_val,
                source="config-override",
            )

        # Tier 3: Hardcoded catalog (OPENFANG-style)
        canonical = self._resolve_alias(model_name)
        if canonical in self.catalog:
            cap = self.catalog[canonical]
            # Return a copy with the requested name
            return ModelCapability(
                model_id=model_name,
                context_window=cap.context_window,
                max_output_tokens=cap.max_output_tokens,
                supports_vision=cap.supports_vision,
                supports_tools=cap.supports_tools,
                aliases=list(cap.aliases),
                source=f"hardcoded({cap.model_id})",
            )

        # Tier 4: models.dev registry (COMMUNITY)
        registry = self.registry.get_registry()
        if canonical in registry:
            cap = registry[canonical]
            return ModelCapability(
                model_id=model_name,
                context_window=cap.context_window,
                max_output_tokens=cap.max_output_tokens,
                supports_vision=cap.supports_vision,
                supports_tools=cap.supports_tools,
                source="models.dev",
            )

        # Tier 5: Default fallback with probing
        return self._fallback_with_probing(model_name)

    def _get_env_override(self) -> Optional[int]:
        """Check for AGENT_DIVA_MAX_CONTEXT_TOKENS env var."""
        val = os.environ.get("AGENT_DIVA_MAX_CONTEXT_TOKENS")
        if val:
            try:
                parsed = int(val)
                if parsed > 0:
                    return parsed
            except ValueError:
                pass
        return None

    def _get_config_override(self, model_name: str) -> Optional[int]:
        """Check for user config file override."""
        config_path = Path.home() / ".agent-diva" / "config.yaml"
        if not config_path.exists():
            return None
        try:
            # Simple YAML parsing without external deps
            with open(config_path, "r", encoding="utf-8") as f:
                content = f.read()
            # Look for model-specific context_length
            pattern = rf"{re.escape(model_name)}.*?context_length:\s*(\d+)"
            match = re.search(pattern, content, re.IGNORECASE | re.DOTALL)
            if match:
                return int(match.group(1))
        except OSError:
            pass
        return None

    def _fallback_with_probing(self, model_name: str) -> ModelCapability:
        """
        Fallback for unknown models with probing tiers.
        In a real implementation, this would try API calls to determine limits.
        """
        # For demo, return the largest safe default
        return ModelCapability(
            model_id=model_name,
            context_window=DEFAULT_FALLBACK_CONTEXT,
            max_output_tokens=8_192,
            supports_vision=False,
            supports_tools=True,
            source="fallback",
        )

    def get_budget_config(self, model_name: str) -> BudgetConfig:
        """Get budget config for a model (convenience wrapper)."""
        capability = self.resolve(model_name)
        return BudgetConfig.from_model(capability)


# =============================================================================
# Demo / Test
# =============================================================================

def main():
    print("=" * 70)
    print("Context Window Manager Demo for agent-diva-pro")
    print("=" * 70)

    resolver = ContextWindowResolver()

    # Test cases
    test_cases = [
        # Alias resolution
        ("sonnet", None, None),
        ("claude-sonnet", None, None),
        ("deepseek", None, None),
        ("gpt-5", None, None),
        # Unknown model
        ("some-unknown-model-v3", None, None),
        # With env override
        ("sonnet", 500_000, None),
        # With config override
        ("gpt-5", None, 200_000),
    ]

    print("\n📋 Test Cases:")
    print("-" * 70)
    for model, env, config in test_cases:
        cap = resolver.resolve(model, env_override=env, config_override=config)
        budget = BudgetConfig.from_model(cap)
        print(f"\n  Model: {model}")
        print(f"    Resolved ID: {cap.model_id}")
        print(f"    Context Window: {cap.context_window:,} tokens")
        print(f"    Max Output: {cap.max_output_tokens:,} tokens")
        print(f"    Vision: {cap.supports_vision}, Tools: {cap.supports_tools}")
        print(f"    Source: {cap.source}")
        print(f"    Budget: {budget.max_tokens:,} tokens (keep_recent={budget.keep_recent})")

    # Show alias map
    print("\n\n📖 Alias Map (sample):")
    print("-" * 70)
    for alias, canonical in sorted(resolver._alias_map.items())[:10]:
        print(f"  {alias:<20} -> {canonical}")
    print(f"  ... ({len(resolver._alias_map)} total aliases)")

    # Show hardcoded catalog
    print("\n\n📚 Hardcoded Catalog:")
    print("-" * 70)
    for model_id, cap in sorted(resolver.catalog.items()):
        if model_id == "default-tekaapi":
            continue
        aliases_str = ", ".join(cap.aliases) if cap.aliases else "none"
        print(f"  {model_id:<25} ctx={cap.context_window:>8,}  aliases=[{aliases_str}]")

    # Show budget calculation breakdown
    print("\n\n🔢 Budget Calculation Breakdown (Claude Code strategy):")
    print("-" * 70)
    for model_id, cap in sorted(resolver.catalog.items()):
        if model_id == "default-tekaapi":
            continue
        reserved = min(cap.max_output_tokens, 20_000)
        buffer = _calculate_buffer(cap.context_window)
        effective = cap.context_window - reserved - buffer
        print(f"\n  {model_id}:")
        print(f"    context_window  = {cap.context_window:,}")
        print(f"    - reserved_out  = {reserved:,} (min(max_output, 20K))")
        print(f"    - buffer        = {buffer:,} (tiered)")
        print(f"    = effective     = {effective:,}")
        print(f"    -> budget       = {max(32_000, effective):,} (max(32K, effective))")

    # Try models.dev (if online)
    print("\n\n🌐 models.dev Registry (requires internet):")
    print("-" * 70)
    registry = resolver.registry.get_registry(use_cache=True)
    if registry:
        print(f"  Fetched {len(registry)} models from models.dev")
        # Show a few samples
        samples = list(registry.items())[:5]
        for model_id, cap in samples:
            print(f"    {model_id:<30} ctx={cap.context_window:,}  src={cap.source}")
    else:
        print("  Could not fetch models.dev (offline or rate limited)")
        print("  Cache file:", CACHE_FILE)

    print("\n" + "=" * 70)
    print("Demo complete!")
    print("=" * 70)


if __name__ == "__main__":
    main()
