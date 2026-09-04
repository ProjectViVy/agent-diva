//! Typed Fabric admission characterization.
//!
//! Session admission is downstream of the typed envelope. These tests keep
//! the trust and identity boundary explicit without reviving a second message
//! transport.

use super::*;
use agent_diva_core::channel::{
    ChannelAddress, ChannelDirection, ChannelOrigin, ChannelPayloadV1, ContentPart, Correlation,
    OwnerTurnContextV1, OwnerTurnIntent,
};

fn message_envelope(
    origin: ChannelOrigin,
    session_key: &str,
    context: Option<OwnerTurnContextV1>,
) -> ChannelEnvelopeV1 {
    ChannelEnvelopeV1::new(
        ChannelDirection::Ingress,
        ChannelAddress::new("gui", "chat"),
        Correlation::new(session_key),
        origin,
        ChannelPayloadV1::Message {
            parts: vec![ContentPart::Text {
                text: "hello".to_string(),
            }],
            subject: None,
            locale: None,
            context,
        },
    )
}

fn owner_context() -> OwnerTurnContextV1 {
    OwnerTurnContextV1 {
        intent: OwnerTurnIntent::Agent,
        approval_policy: None,
        execution: None,
    }
}

#[test]
fn typed_admission_uses_envelope_session_identity() {
    let envelope = message_envelope(
        ChannelOrigin::OwnerFrontend,
        "profile/session-42",
        Some(owner_context()),
    );
    let admission = turn::admission::TurnAdmission::classify(&envelope);
    assert_eq!(admission.session_key, "profile/session-42");
    assert!(!admission.scheduled);
}

#[test]
fn owner_turn_without_context_is_rejected_at_admission_boundary() {
    let error = prepare_turn_envelope(message_envelope(
        ChannelOrigin::OwnerFrontend,
        "profile/session-42",
        None,
    ))
    .unwrap_err();
    assert!(error.contains("owner frontend message requires typed turn context"));
}

#[test]
fn external_and_runtime_turns_require_context_free_agent_semantics() {
    let external = message_envelope(ChannelOrigin::ExternalUser, "external/session", None);
    assert!(external.validate().is_ok());
    assert_eq!(
        turn::admission::TurnAdmission::classify(&external).mode,
        turn::admission::TurnMode::Agent
    );

    let runtime = message_envelope(ChannelOrigin::Runtime, "runtime/cron/job", None);
    assert!(runtime.validate().is_ok());
    let admission = turn::admission::TurnAdmission::classify(&runtime);
    assert_eq!(admission.mode, turn::admission::TurnMode::Agent);
    assert!(admission.scheduled);

    let invalid = message_envelope(
        ChannelOrigin::ExternalUser,
        "external/session",
        Some(owner_context()),
    );
    assert!(invalid.validate().is_err());
}
