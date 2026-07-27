//! Canonical MCP wire values and generated wire schemas.

pub mod contracts;
pub mod json_rpc;
pub mod methods;
pub mod tools;

pub use contracts::{wire_contract_descriptors, WireContractDescriptor};
pub use methods::*;
pub use tools::*;
