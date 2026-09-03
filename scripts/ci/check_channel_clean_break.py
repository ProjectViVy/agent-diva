#!/usr/bin/env python3
"""Fail when retired channel production surfaces are reintroduced."""

from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[2]

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
    return found


def self_test() -> int:
    sample = '.route("/api/chat", post(handler))'
    assert '.route("/api/chat"' in sample
    assert "ChannelManager" not in "ChannelRuntime"
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
