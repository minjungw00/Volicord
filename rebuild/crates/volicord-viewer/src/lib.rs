//! Thin local viewer and static snapshot renderer over projection and Local
//! Operations APIs.
//!
//! The viewer owns presentation, local HTTP transport, and explicit local
//! snapshot export only. It has no database, canonical authority, Candidate
//! lifecycle, or Guarded approval model of its own.

mod http;
mod render;
mod selection;
pub use selection::{CodeScope, ViewerTool, ViewerView};

pub use http::ViewerServer;
pub use render::{
    ViewerAdapter, ViewerError, ViewerLocale, ViewerPage, ViewerRenderProfile, ViewerRequest,
};
