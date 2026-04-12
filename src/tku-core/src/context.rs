use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::mpsc;

use crate::output::RenderFormat;

// ── ProgressSender ────────────────────────────────────────────────────────────

/// A cheap-to-clone handle for streaming progress lines back to the TUI while
/// a handler is running. `None` when running in plain-CLI mode so all
/// `.send()` calls are no-ops — handlers never have to branch on mode.
#[derive(Clone, Default)]
pub struct ProgressSender(Option<mpsc::UnboundedSender<String>>);

impl ProgressSender {
    /// Send a progress message. Silently drops the message if there is no
    /// active TUI receiver (e.g. plain-CLI mode, or after the dispatch loop
    /// has already finished).
    pub fn send(&self, msg: impl Into<String>) {
        if let Some(tx) = &self.0 {
            let _ = tx.send(msg.into());
        }
    }

    /// Returns `true` when there is an active TUI receiver.
    pub fn is_active(&self) -> bool {
        self.0.is_some()
    }
}

// ── Ctx ───────────────────────────────────────────────────────────────────────

/// Runtime context passed to every handler invocation.
/// `Clone` is cheap — the inner data is reference-counted.
#[derive(Clone)]
pub struct Ctx {
    inner:    Arc<CtxInner>,
    /// Live progress channel, injected by the TUI dispatch loop.
    pub progress: ProgressSender,
}

struct CtxInner {
    pub format:   RenderFormat,
    pub tui_mode: bool,
    pub flags:    HashMap<String, String>,
}

impl Ctx {
    pub fn new(format: RenderFormat, tui_mode: bool, flags: HashMap<String, String>) -> Self {
        Self {
            inner:    Arc::new(CtxInner { format, tui_mode, flags }),
            progress: ProgressSender::default(),
        }
    }

    pub fn format(&self) -> RenderFormat {
        self.inner.format
    }

    pub fn tui_mode(&self) -> bool {
        self.inner.tui_mode
    }

    pub fn flag(&self, key: &str) -> Option<&str> {
        self.inner.flags.get(key).map(|s| s.as_str())
    }

    /// Return a clone of this `Ctx` with an active progress sender attached.
    /// Called by the TUI dispatch loop immediately before invoking a handler.
    pub fn with_progress(mut self, tx: mpsc::UnboundedSender<String>) -> Self {
        self.progress = ProgressSender(Some(tx));
        self
    }
}

// ── CtxBuilder ────────────────────────────────────────────────────────────────

/// Builder for constructing a `Ctx` before dispatch.
#[derive(Default)]
pub struct CtxBuilder {
    format:   RenderFormat,
    tui_mode: bool,
    flags:    HashMap<String, String>,
}

impl CtxBuilder {
    pub fn format(mut self, f: RenderFormat) -> Self {
        self.format = f;
        self
    }

    pub fn tui_mode(mut self, t: bool) -> Self {
        self.tui_mode = t;
        self
    }

    pub fn flag(mut self, k: impl Into<String>, v: impl Into<String>) -> Self {
        self.flags.insert(k.into(), v.into());
        self
    }

    pub fn build(self) -> Ctx {
        Ctx::new(self.format, self.tui_mode, self.flags)
    }
}
