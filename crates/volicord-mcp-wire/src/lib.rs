//! Canonical MCP wire values and generated wire schemas.

pub mod contracts;
pub mod json_rpc;
pub mod methods;
pub mod semantic_schema;
pub mod tool_contracts;
pub mod tools;

pub use contracts::{wire_contract_descriptors, WireContractDescriptor};
pub use methods::*;
pub use semantic_schema::*;
pub use tool_contracts::*;
pub use tools::*;
