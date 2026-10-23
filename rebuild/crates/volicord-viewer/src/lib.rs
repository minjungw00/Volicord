//! Thin local viewer and static snapshot renderer over projection and Local
//! Operations APIs.
//!
//! The viewer owns presentation, local HTTP transport, and explicit local
//! snapshot export only. It has no database, canonical authority, Candidate
//! lifecycle, or Guarded approval model of its own.

mod diagram;
mod http;
mod interaction;
mod render;
mod selection;
mod source_highlight;
pub use selection::ViewerView;

pub use http::ViewerServer;
pub use render::{
    ViewerAdapter, ViewerError, ViewerLocale, ViewerPage, ViewerRenderProfile, ViewerRequest,
};

mod observation;
