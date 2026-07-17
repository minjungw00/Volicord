#![deny(unsafe_code)]

pub mod catalog;
#[path = "../contracts/mod.rs"]
pub mod contracts;
pub mod error;
pub mod gate;
#[path = "../hosts/mod.rs"]
pub mod hosts;
pub mod io;
pub mod pipeline;
#[path = "../platforms/mod.rs"]
pub mod platforms;
#[path = "../scenarios/mod.rs"]
pub mod scenarios;

#[cfg(test)]
mod pipeline_tests;
#[cfg(test)]
mod tests;
