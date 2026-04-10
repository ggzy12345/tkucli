use crate::{
    error::TkucliResult,
    handler::{BoxRender, CliRequest, CliService, HandlerRegistry, RouterService},
    middleware::ServiceBuilder,
};
use std::sync::Arc;

// ── Router ────────────────────────────────────────────────────────────────────

/// Fluent builder that assembles a `RouterService` + middleware stack.
///
/// ```rust,ignore
/// let svc = Router::new(registry)
///     .layer(LoggingLayer)
///     .layer(AuthLayer::from_env("MY_APP_TOKEN"))
///     .layer(ConfirmLayer::new(["users.delete", "users.create"]))
///     .build();
///
/// // Dispatch a request:
/// let result = svc.call(CliRequest::new(ctx, "users", "list", args)).await?;
/// ```
pub struct Router {
    registry: HandlerRegistry,
    builder:  ServiceBuilder,
}

impl Router {
    pub fn new(registry: HandlerRegistry) -> Self {
        Self {
            registry,
            builder: ServiceBuilder::new(),
        }
    }

    /// Add a middleware layer (same API as `tower::ServiceBuilder::layer`).
    pub fn layer(mut self, layer: impl crate::middleware::Layer + 'static) -> Self {
        self.builder = self.builder.layer(layer);
        self
    }

    /// Finalise the router. Returns an `Arc<dyn CliService>` ready for dispatch.
    pub fn build(self) -> Arc<dyn CliService> {
        self.builder.service(RouterService::new(self.registry))
    }
}

// ── Convenience top-level dispatch ───────────────────────────────────────────

/// Stateless dispatch helper — useful in the generated `router.rs` where
/// the service is already built and stored in a once-cell.
pub async fn dispatch(svc: &dyn CliService, req: CliRequest) -> TkucliResult<BoxRender> {
    svc.call(req).await
}
