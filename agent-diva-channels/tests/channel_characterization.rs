use agent_diva_channels::base::{validate_neurolink_host, BaseChannel, ChannelError};
use agent_diva_core::config::Config;

#[test]
fn legacy_base_channel_empty_allowlist_defaults_to_allow_all() {
    let channel = BaseChannel::new("characterization", Config::default(), vec![]);
    assert!(channel.is_allowed("user-1"));
}

#[test]
fn legacy_base_channel_deny_by_default_is_explicit() {
    let channel =
        BaseChannel::with_default_policy("characterization", Config::default(), vec![], true);
    assert!(!channel.is_allowed("user-1"));
}

#[test]
fn legacy_base_channel_compound_allowlist_matches_each_component() {
    let channel = BaseChannel::new(
        "characterization",
        Config::default(),
        vec!["user-1".to_string()],
    );
    assert!(channel.is_allowed("external-id|user-1"));
    assert!(!channel.is_allowed("external-id|user-2"));
}

#[test]
fn legacy_neuro_link_host_guard_accepts_only_loopback_names() {
    assert!(validate_neurolink_host("127.0.0.1").is_ok());
    assert!(validate_neurolink_host("localhost").is_ok());
    assert!(validate_neurolink_host("0.0.0.0").is_err());
}

#[test]
fn legacy_channel_error_display_is_stable_for_characterization() {
    let error = ChannelError::AccessDenied("user-1".to_string());
    assert_eq!(error.to_string(), "Access denied for sender: user-1");
}
