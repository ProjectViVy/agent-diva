//! Config-rooted Markdown authority for the single machine-wide Persona.

mod service;
mod text;
mod types;

pub use service::PersonaService;
pub use text::{extract_markdown_section, normalize_markdown, visible_len};
pub use types::{
    PersonaChangeRequest, PersonaDocument, PersonaError, PersonaFileState, PersonaHistoryEntry,
    PersonaHistoryRevision, PersonaInitialization, PersonaKind, PersonaRepair, PersonaRequestActor,
    PersonaRequestState, PersonaStatus, PersonaStatusView, PersonaWriteOutcome, PersonaWriteSource,
};
