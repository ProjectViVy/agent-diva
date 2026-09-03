#!/usr/bin/env python3
"""Fail when retired channel production surfaces are reintroduced."""

from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[2]
SELF = Path(__file__).resolve()

# Only active product/build roots are scanned. Historical decisions, research,
# and iteration records deliberately live under docs/ and are not rewritten by
# a clean-break implementation. Archive directories are excluded as an extra
# guard when a product root contains a checked-in fixture/archive.
SCAN_ROOTS = (
    ROOT / "Cargo.toml",
    ROOT / "Cargo.lock",
    ROOT / "justfile",
    ROOT / ".github",
    ROOT / "scripts" / "ci",
    ROOT / "agent-diva-core",
    ROOT / "agent-diva-agent",
    ROOT / "agent-diva-manager",
    ROOT / "agent-diva-tools",
    ROOT / "agent-diva-tooling",
    ROOT / "agent-diva-channels",
    ROOT / "agent-diva-cli",
    ROOT / "agent-diva-e2e",
    ROOT / "agent-diva-service",
    ROOT / "agent-diva-migration",
    ROOT / "agent-diva-gui",
)

SKIP_DIR_NAMES = {
    ".git",
    "target",
    "node_modules",
    "dist",
    "archive",
    "archives",
    "historical",
    "history",
}
TEXT_SUFFIXES = {
    ".rs",
    ".toml",
    ".lock",
    ".yml",
    ".yaml",
    ".json",
    ".ts",
    ".tsx",
    ".vue",
    ".js",
    ".py",
    ".md",
    ".just",
}

# These names are the deleted AgentLoop transport and the old MessageBus
# surfaces. Keep the list as source strings only; the checker itself is
# excluded from the recursive scan so its denylist cannot self-trigger.
FORBIDDEN_ACTIVE_TOKENS = (
    "InboundMessage",
    "OutboundMessage",
    "channel_envelope_to_inbound",
    "publish_inbound",
    "publish_outbound",
    "take_inbound_receiver",
    "take_outbound_receiver",
    "subscribe_inbound",
    "subscribe_outbound",
    "MessageBus",
    "InboundSender",
    "InboundReceiver",
    "OutboundSender",
    "OutboundReceiver",
    "OutboundCallback",
    "legacy_execution_start",
    "approved_plan_markdown",
)

FORBIDDEN_FILES = [
    "agent-diva-channels/src/base.rs",
    "agent-diva-channels/src/manager.rs",
    "agent-diva-channels/src/neuro_link.rs",
    "agent-diva-channels/src/slack.rs",
    "agent-diva-channels/src/whatsapp.rs",
    "agent-diva-channels/src/matrix.rs",
    "agent-diva-channels/src/irc.rs",
    "agent-diva-channels/src/mattermost.rs",
    "agent-diva-channels/src/nextcloud_talk.rs",
]

TEXT_RULES = {
    "agent-diva-channels/src/lib.rs": ["ChannelHandler", "ChannelManager", "NeuroLinkHandler"],
    "agent-diva-channels/Cargo.toml": ["channel-slack", "channel-whatsapp", "slack-morphism"],
    "agent-diva-core/src/config/schema.rs": [
        "NeuroLinkConfig",
        "WhatsAppConfig",
        "SlackConfig",
        "MatrixConfig",
        "MattermostConfig",
        "NextcloudTalkConfig",
    ],
    "agent-diva-manager/src/server.rs": [
        '.route("/api/chat"',
        '.route("/api/chat/stop"',
        '.route("/api/events"',
    ],
    "agent-diva-manager/src/runtime/task_runtime.rs": [
        "ChannelManager",
        "OLV_AVATAR_CHAT_ID",
        "spawn_neuro_link_gui_bridge",
        "subscribe_outbound",
    ],
    "agent-diva-core/src/bus/queue.rs": ["unbounded_channel", "UnboundedSender", "OutboundCallback"],
    "agent-diva-gui/src-tauri/src/lib.rs": [
        "commands::send_message,",
        "commands::continue_approved_plan_execution,",
        "commands::stop_generation,",
        "commands::start_background_stream,",
    ],
    "agent-diva-gui/src/App.vue": ["agent-background-response", "start_background_stream"],
    "agent-diva-cli/src/client.rs": ['format!("{}/chat"', 'format!("{}/chat/stop"'],
}


def files_to_scan(roots=SCAN_ROOTS):
    for root in roots:
        if not root.exists():
            continue
        if root.is_file():
            if root.resolve() != SELF:
                yield root
            continue
        for path in root.rglob("*"):
            if not path.is_file():
                continue
            if path.resolve() == SELF:
                continue
            if any(part.lower() in SKIP_DIR_NAMES for part in path.parts):
                continue
            if path.suffix.lower() in TEXT_SUFFIXES or path.name == "justfile":
                yield path


def active_token_violations(roots=SCAN_ROOTS):
    found = []
    for path in files_to_scan(roots):
        relative = path.relative_to(ROOT) if path.is_relative_to(ROOT) else path
        text = path.read_text(encoding="utf-8", errors="replace")
        for line_number, line in enumerate(text.splitlines(), start=1):
            matched = [token for token in FORBIDDEN_ACTIVE_TOKENS if token in line]
            for token in matched:
                found.append(f"{relative}:{line_number}: forbidden active token {token!r}")
    return found


def violations() -> list[str]:
    found: list[str] = []
    for relative in FORBIDDEN_FILES:
        if (ROOT / relative).exists():
            found.append(f"retired file exists: {relative}")
    for relative, needles in TEXT_RULES.items():
        text = (ROOT / relative).read_text(encoding="utf-8")
        for needle in needles:
            if needle in text:
                found.append(f"{relative}: forbidden production token {needle!r}")
    found.extend(active_token_violations())
    return found


def self_test() -> int:
    sample = '.route("/api/chat", post(handler))'
    assert '.route("/api/chat"' in sample
    assert "ChannelManager" not in "ChannelRuntime"
    import tempfile

    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp) / "active"
        (root / "src").mkdir(parents=True)
        (root / "docs" / "historical").mkdir(parents=True)
        (root / "src" / "bad.rs").write_text(
            "fn old() { InboundMessage::new(); }\nMessageBus\n", encoding="utf-8"
        )
        (root / "docs" / "historical" / "record.md").write_text(
            "InboundMessage and MessageBus are historical evidence.\n", encoding="utf-8"
        )
        hits = active_token_violations([root])
        assert any("bad.rs" in hit and "InboundMessage" in hit for hit in hits)
        assert any("bad.rs" in hit and "MessageBus" in hit for hit in hits)
        assert not any("record.md" in hit for hit in hits)
    print("Channel clean-break self-test passed")
    return 0


def main() -> int:
    if "--self-test" in sys.argv:
        return self_test()
    found = violations()
    if found:
        print("Channel clean-break violations:")
        for item in found:
            print(f"- {item}")
        return 1
    print("Channel clean-break verified")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
