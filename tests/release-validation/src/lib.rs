#![deny(unsafe_code)]

#[path = "../contracts/mod.rs"]
pub mod contracts;
pub mod error;
#[path = "../hosts/mod.rs"]
pub mod hosts;
pub mod io;
#[path = "../platforms/mod.rs"]
pub mod platforms;
#[path = "../scenarios/mod.rs"]
pub mod scenarios;
pub mod schema;

#[cfg(test)]
mod tests;
