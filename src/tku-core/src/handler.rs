use crate::{
    context::Ctx,
    error::{TkucliError, TkucliResult},
    extract::{FromArgs, ParsedArgs},
    output::{IntoOutput, Render},
};
use async_trait::async_trait;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Type alias for a heap-allocated, type-erased render value.
pub type BoxRender = Box<dyn Render>;

/// Type alias for a pinned boxed async future.
pub type BoxFuture<'a> = Pin<Box<dyn Future<Output = TkucliResult<BoxRender>> + Send + 'a>>;

// ── HandlerMeta ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HandlerMeta {
    pub resource: String,
    pub verb:     String,
}

impl HandlerMeta {
    pub fn new(resource: impl Into<String>, verb: impl Into<String>) -> Self {
        Self { resource: resource.into(), verb: verb.into() }
    }

    /// Dot-notation registry key, e.g. "users.list".
    pub fn key(&self) -> String {
        format!("{}.{}", self.resource, self.verb)
    }
}

// ── ErasedHandler ─────────────────────────────────────────────────────────────

/// Internal type-erased handler stored in the registry.
/// Never implement this directly — use `handler_fn` or `#[tkucli::handler]`.
#[async_trait]
pub trait ErasedHandler: Send + Sync {
    fn meta(&self) -> &HandlerMeta;
    async fn call(&self, ctx: &Ctx, args: ParsedArgs) -> TkucliResult<BoxRender>;
}

// ── TypedHandler ─────────────────────────────────────────────────────────────

/// Typed handler bridging concrete `A: FromArgs` / `O: IntoOutput` types
/// to the erased `ErasedHandler` interface.
///
/// Mirrors Axum's `Handler<T, S>` + tuple-extractor blanket impl pattern,
/// simplified to a single args struct.
pub struct TypedHandler<F, A, O> {
    meta: HandlerMeta,
    f:    Arc<F>,
    _a:   std::marker::PhantomData<fn() -> A>,
    _o:   std::marker::PhantomData<fn() -> O>,
}

impl<F, A, O> TypedHandler<F, A, O> {
    pub fn new(meta: HandlerMeta, f: F) -> Self {
        Self {
            meta,
            f: Arc::new(f),
            _a: std::marker::PhantomData,
            _o: std::marker::PhantomData,
        }
    }
}

#[async_trait]
impl<F, A, O, Fut> ErasedHandler for TypedHandler<F, A, O>
where
    F:   Fn(Ctx, A) -> Fut + Send + Sync + 'static,
    A:   FromArgs + Send + 'static,
    O:   IntoOutput + 'static,
    Fut: Future<Output = TkucliResult<O>> + Send + 'static,
{
    fn meta(&self) -> &HandlerMeta {
        &self.meta
    }

    async fn call(&self, ctx: &Ctx, args: ParsedArgs) -> TkucliResult<BoxRender> {
        // Step 1 — Extract typed args from ParsedArgs.
        //          Mirrors Axum calling FromRequest::from_request on each arg.
        let typed_args = A::from_args(&args)?;

        // Step 2 — Call the user's handler with real types.
        let output = (self.f)(ctx.clone(), typed_args).await?;

        // Step 3 — Convert output to BoxRender.
        //          Mirrors Axum calling IntoResponse::into_response.
        Ok(output.into_output())
    }
}

// ── handler_fn constructor ────────────────────────────────────────────────────

/// Wrap any async function into a boxed `ErasedHandler`.
///
/// ```rust,ignore
/// registry.register(handler_fn(
///     HandlerMeta::new("users", "list"),
///     |ctx: Ctx, args: ListArgs| async move {
///         Ok(Table::from(fetch_users(args.limit).await?))
///     },
/// ));
/// ```
pub fn handler_fn<F, A, O, Fut>(meta: HandlerMeta, f: F) -> Box<dyn ErasedHandler>
where
    F:   Fn(Ctx, A) -> Fut + Send + Sync + 'static,
    A:   FromArgs + Send + 'static,
    O:   IntoOutput + 'static,
    Fut: Future<Output = TkucliResult<O>> + Send + 'static,
{
    Box::new(TypedHandler::new(meta, f))
}

// ── HandlerRegistry ───────────────────────────────────────────────────────────

#[derive(Default)]
pub struct HandlerRegistry {
    handlers: HashMap<String, Box<dyn ErasedHandler>>,
}

impl HandlerRegistry {
    pub fn new() -> Self { Self::default() }

    /// Panics on duplicate keys — misconfiguration should surface at startup.
    pub fn register(&mut self, handler: Box<dyn ErasedHandler>) {
        let key = handler.meta().key();
        if self.handlers.contains_key(&key) {
            panic!("duplicate handler registered for key: {key}");
        }
        self.handlers.insert(key, handler);
    }

    pub fn get(&self, resource: &str, verb: &str) -> Option<&dyn ErasedHandler> {
        self.handlers
            .get(&format!("{resource}.{verb}"))
            .map(|h| h.as_ref())
    }

    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.handlers.keys().map(String::as_str)
    }
}

// ── CliRequest / CliService ───────────────────────────────────────────────────

/// CLI equivalent of `http::Request`.
/// Passed through the middleware chain and ultimately consumed by the router.
#[derive(Clone)]
pub struct CliRequest {
    pub ctx:      Ctx,
    pub resource: String,
    pub verb:     String,
    pub args:     ParsedArgs,
}

impl CliRequest {
    pub fn new(
        ctx: Ctx,
        resource: impl Into<String>,
        verb: impl Into<String>,
        args: ParsedArgs,
    ) -> Self {
        Self { ctx, resource: resource.into(), verb: verb.into(), args }
    }
}

/// Lightweight Service trait — mirrors tower::Service<CliRequest> without
/// the tower dependency. Mechanical migration to tower later if needed.
#[async_trait]
pub trait CliService: Send + Sync {
    async fn call(&self, req: CliRequest) -> TkucliResult<BoxRender>;
}

/// The leaf service — registry lookup with no middleware.
pub struct RouterService {
    registry: Arc<HandlerRegistry>,
}

impl RouterService {
    pub fn new(registry: HandlerRegistry) -> Self {
        Self { registry: Arc::new(registry) }
    }
}

#[async_trait]
impl CliService for RouterService {
    async fn call(&self, req: CliRequest) -> TkucliResult<BoxRender> {
        let handler = self
            .registry
            .get(&req.resource, &req.verb)
            .ok_or_else(|| {
                TkucliError::CommandNotFound(format!("{} {}", req.resource, req.verb))
            })?;

        handler.call(&req.ctx, req.args).await
    }
}
