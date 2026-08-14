//! Thin local viewer over projection and Local Operations APIs.
//!
//! The viewer owns presentation and local HTTP transport only. It has no
//! database, canonical authority, Candidate lifecycle, or Guarded approval
//! model of its own.

mod http;
mod render;

pub use http::ViewerServer;
pub use render::{
    ExplanationLevel, ViewerAdapter, ViewerError, ViewerLocale, ViewerPage, ViewerRequest,
};
