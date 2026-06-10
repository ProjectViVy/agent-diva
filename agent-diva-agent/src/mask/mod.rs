//! Mask system for agent-diva
//!
//! Masks are markdown files with YAML frontmatter that define agent personas.
//! They control model selection, tool access, and system prompt overrides.

pub mod error;
pub mod mask_file;
pub mod mask_prompt_composer;
pub mod mask_registry;
pub mod tool_policy;

pub use error::MaskError;
pub use mask_file::MaskFile;
pub use mask_prompt_composer::MaskPromptComposer;
pub use mask_registry::MaskRegistry;
pub use tool_policy::ToolPolicy;
