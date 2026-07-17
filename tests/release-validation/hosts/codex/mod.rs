use crate::schema::CodexCapability;

pub const FIRST_RELEASE_CODEX_CAPABILITIES: [CodexCapability; 4] = CodexCapability::FIRST_RELEASE;

pub fn has_exact_first_release_capabilities(capabilities: &[CodexCapability]) -> bool {
    capabilities == FIRST_RELEASE_CODEX_CAPABILITIES
}
