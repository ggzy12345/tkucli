use crate::{
    error::TkucliResult,
    handler::{BoxRender, CliRequest, CliService},
};
use async_trait::async_trait;
use std::sync::Arc;

// ── Layer / Middleware ────────────────────────────────────────────────────────

/// A `Layer` wraps an inner `CliService` and produces a new `CliService`.
/// This is the CLI equivalent of `tower::Layer`.
///
/// Each middleware implements `Layer` to describe how it wraps the next
/// service in the chain. The `ServiceBuilder` assembles the layers.
pub trait Layer: Send + Sync {
    fn layer(&self, inner: Arc<dyn CliService>) -> Arc<dyn CliService>;
}

// ── ServiceBuilder ────────────────────────────────────────────────────────────

/// Assembles a middleware stack around a leaf `CliService`.
/// Mirrors `tower::ServiceBuilder`.
///
/// ```rust,ignore
/// let app = ServiceBuilder::new()
///     .layer(LoggingLayer)
///     .layer(AuthLayer::from_env("MY_APP_TOKEN"))
///     .service(RouterService::new(registry));
/// ```
///
/// Layers are applied in declaration order — the first `.layer()` call
/// becomes the outermost wrapper (first to see the request, last to see
/// the response). This matches tower's semantics.
pub struct ServiceBuilder {
    layers: Vec<Box<dyn Layer>>,
}

impl ServiceBuilder {
    pub fn new() -> Self {
        Self { layers: Vec::new() }
    }

    /// Add a middleware layer to the stack.
    pub fn layer(mut self, layer: impl Layer + 'static) -> Self {
        self.layers.push(Box::new(layer));
        self
    }

    /// Finalise the stack by wrapping the leaf service with all layers.
    /// Returns the outermost `CliService` ready for dispatch.
    pub fn service(self, leaf: impl CliService + 'static) -> Arc<dyn CliService> {
        let mut svc: Arc<dyn CliService> = Arc::new(leaf);
        // Apply layers in reverse so the first-declared is the outermost.
        for layer in self.layers.into_iter().rev() {
            svc = layer.layer(svc);
        }
        svc
    }
}

impl Default for ServiceBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ── Built-in layers ───────────────────────────────────────────────────────────

// ── LoggingLayer ──────────────────────────────────────────────────────────────

pub struct LoggingLayer;

impl Layer for LoggingLayer {
    fn layer(&self, inner: Arc<dyn CliService>) -> Arc<dyn CliService> {
        Arc::new(LoggingService { inner })
    }
}

struct LoggingService {
    inner: Arc<dyn CliService>,
}

#[async_trait]
impl CliService for LoggingService {
    async fn call(&self, req: CliRequest) -> TkucliResult<BoxRender> {
        tracing::debug!(
            resource = %req.resource,
            verb = %req.verb,
            format = %req.ctx.format(),
            "dispatching command"
        );

        let result = self.inner.call(req).await;

        if let Err(ref e) = result {
            tracing::error!(error = %e, "command failed");
        }

        result
    }
}

// ── AuthLayer ─────────────────────────────────────────────────────────────────

/// Token auth layer. Reads a token from the environment before
/// forwarding the request. Stores the token in the context extensions
/// so handlers can access it if needed.
pub struct AuthLayer {
    env_var: String,
}

impl AuthLayer {
    pub fn from_env(env_var: impl Into<String>) -> Self {
        Self {
            env_var: env_var.into(),
        }
    }
}

impl Layer for AuthLayer {
    fn layer(&self, inner: Arc<dyn CliService>) -> Arc<dyn CliService> {
        Arc::new(AuthService {
            inner,
            env_var: self.env_var.clone(),
        })
    }
}

struct AuthService {
    inner: Arc<dyn CliService>,
    env_var: String,
}

#[async_trait]
impl CliService for AuthService {
    async fn call(&self, req: CliRequest) -> TkucliResult<BoxRender> {
        use crate::error::TkucliError;

        match std::env::var(&self.env_var) {
            Ok(_token) => {
                // Token is present — forward the request.
                // TODO: attach token to ctx extensions once extensions are Arc-swappable.
                self.inner.call(req).await
            }
            Err(_) => Err(TkucliError::Auth(format!(
                "missing auth token — set ${}",
                self.env_var
            ))),
        }
    }
}

// ── ConfirmLayer ──────────────────────────────────────────────────────────────

/// Prompts the user for confirmation before forwarding destructive operations.
/// Activated when `confirm = true` in the operation schema.
pub struct ConfirmLayer {
    /// Set of "resource.verb" keys that require confirmation.
    pub requires_confirm: std::collections::HashSet<String>,
}

impl ConfirmLayer {
    pub fn new(keys: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            requires_confirm: keys.into_iter().map(|s| s.into()).collect(),
        }
    }
}

impl Layer for ConfirmLayer {
    fn layer(&self, inner: Arc<dyn CliService>) -> Arc<dyn CliService> {
        Arc::new(ConfirmService {
            inner,
            requires_confirm: self.requires_confirm.clone(),
        })
    }
}

struct ConfirmService {
    inner: Arc<dyn CliService>,
    requires_confirm: std::collections::HashSet<String>,
}

#[async_trait]
impl CliService for ConfirmService {
    async fn call(&self, req: CliRequest) -> TkucliResult<BoxRender> {
        use crate::error::TkucliError;

        let key = format!("{}.{}", req.resource, req.verb);
        if self.requires_confirm.contains(&key) {
            eprint!(
                "  Are you sure you want to run `{} {}`? [y/N] ",
                req.resource, req.verb
            );

            let mut input = String::new();
            std::io::stdin().read_line(&mut input).ok();

            if !matches!(input.trim().to_lowercase().as_str(), "y" | "yes") {
                return Err(TkucliError::Aborted);
            }
        }

        self.inner.call(req).await
    }
}
