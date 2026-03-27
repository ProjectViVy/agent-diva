//! Internal backend markers for gradual memory backend expansion.

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum MemoryBackendKind {
    #[default]
    File,
    Sqlite,
}
