pub mod context;
pub mod error;
pub mod extract;
pub mod handler;
pub mod middleware;
pub mod output;
pub mod router;
pub mod schema;

/// Convenient re-exports for handler implementors.
pub mod prelude {
    pub use crate::context::Ctx;
    pub use crate::error::{TkucliError, TkucliResult};
    pub use crate::extract::{ArgValue, FromArgs, Optional, ParsedArgs};
    pub use crate::handler::{
        handler_fn, BoxRender, CliRequest, CliService, ErasedHandler, HandlerMeta, HandlerRegistry,
        RouterService,
    };
    pub use crate::middleware::{AuthLayer, ConfirmLayer, Layer, LoggingLayer, ServiceBuilder};
    pub use crate::output::{IntoOutput, Record, Render, RenderFormat, Success, Table};
    pub use crate::router::Router;
    pub use async_trait::async_trait;
}
