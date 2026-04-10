use std::collections::HashMap;
use std::sync::Arc;

use crate::output::RenderFormat;

/// Runtime context passed to every handler invocation.
/// `Clone` is cheap — the inner data is reference-counted.
#[derive(Clone)]
pub struct Ctx {
    inner: Arc<CtxInner>,
}

struct CtxInner {
    pub format:   RenderFormat,
    pub tui_mode: bool,
    pub flags:    HashMap<String, String>,
}

impl Ctx {
    pub fn new(format: RenderFormat, tui_mode: bool, flags: HashMap<String, String>) -> Self {
        Self {
            inner: Arc::new(CtxInner { format, tui_mode, flags }),
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
}

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
        // Clean construction — no Arc::get_mut needed.
        Ctx::new(self.format, self.tui_mode, self.flags)
    }
}
