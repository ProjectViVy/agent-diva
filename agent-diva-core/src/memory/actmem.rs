//! Provider-neutral ACTMEM and MEMRULES tool contracts.

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActmemReadTarget {
    Pulse,
    Recap,
    Work,
    Head,
    Capsules,
    Capsule,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ActmemReadRequest {
    pub target: ActmemReadTarget,
    pub capsule_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ActmemReadResponse {
    pub revision: u64,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ActmemEditWorkRequest {
    pub section: String,
    pub replacement: String,
    pub base_revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ActmemItemRequest {
    pub section: String,
    pub item_index: usize,
    pub base_revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ActmemMutationResponse {
    pub revision: u64,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MemoryRulesResponse {
    pub content: String,
    pub source: String,
}
