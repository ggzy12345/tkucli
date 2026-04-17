use crate::error::{TkucliError, TkucliResult};
use std::collections::HashMap;

// ── ParsedArgs ────────────────────────────────────────────────────────────────

/// The typed wrapper around clap's parsed values passed into every extractor.
/// Think of this as the CLI equivalent of Axum's `http::Request` —
/// it's what `FromArgs` implementations pull values out of.
#[derive(Debug, Clone, Default)]
pub struct ParsedArgs {
    /// Positional arguments, in declaration order.
    pub positional: Vec<String>,
    /// Named flags, keyed by long name (without leading `--`).
    pub flags: HashMap<String, ArgValue>,
}

impl ParsedArgs {
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a named flag value.
    pub fn insert(&mut self, key: impl Into<String>, value: ArgValue) {
        self.flags.insert(key.into(), value);
    }

    /// Push a positional argument.
    pub fn push(&mut self, value: impl Into<String>) {
        self.positional.push(value.into());
    }

    /// Get a flag by name, returning an error if it's missing.
    pub fn require(&self, key: &str) -> TkucliResult<&ArgValue> {
        self.flags
            .get(key)
            .ok_or_else(|| TkucliError::MissingArgument(key.to_owned()))
    }

    /// Get an optional flag by name.
    pub fn get(&self, key: &str) -> Option<&ArgValue> {
        self.flags.get(key)
    }

    /// Get a positional arg by index.
    pub fn positional(&self, index: usize) -> TkucliResult<&str> {
        self.positional
            .get(index)
            .map(|s| s.as_str())
            .ok_or_else(|| TkucliError::MissingArgument(format!("positional[{index}]")))
    }
}

// ── ArgValue ──────────────────────────────────────────────────────────────────

/// A single parsed argument value. Mirrors the types declared in the schema.
#[derive(Debug, Clone)]
pub enum ArgValue {
    String(String),
    U32(u32),
    U64(u64),
    I64(i64),
    F64(f64),
    Bool(bool),
    /// Chosen variant of an enum flag.
    Enum(String),
}

impl ArgValue {
    pub fn as_str(&self) -> TkucliResult<&str> {
        match self {
            ArgValue::String(s) | ArgValue::Enum(s) => Ok(s.as_str()),
            other => Err(TkucliError::InvalidArgument {
                name: "?".into(),
                reason: format!("expected string, got {other:?}"),
            }),
        }
    }

    pub fn as_u32(&self) -> TkucliResult<u32> {
        match self {
            ArgValue::U32(n) => Ok(*n),
            ArgValue::String(s) => s.parse().map_err(|_| TkucliError::InvalidArgument {
                name: "?".into(),
                reason: format!("expected u32, got {s:?}"),
            }),
            other => Err(TkucliError::InvalidArgument {
                name: "?".into(),
                reason: format!("expected u32, got {other:?}"),
            }),
        }
    }

    pub fn as_u64(&self) -> TkucliResult<u64> {
        match self {
            ArgValue::U64(n) => Ok(*n),
            ArgValue::U32(n) => Ok(*n as u64),
            ArgValue::String(s) => s.parse().map_err(|_| TkucliError::InvalidArgument {
                name: "?".into(),
                reason: format!("expected u64, got {s:?}"),
            }),
            other => Err(TkucliError::InvalidArgument {
                name: "?".into(),
                reason: format!("expected u64, got {other:?}"),
            }),
        }
    }

    pub fn as_i64(&self) -> TkucliResult<i64> {
        match self {
            ArgValue::I64(n) => Ok(*n),
            ArgValue::String(s) => s.parse().map_err(|_| TkucliError::InvalidArgument {
                name: "?".into(),
                reason: format!("expected i64, got {s:?}"),
            }),
            other => Err(TkucliError::InvalidArgument {
                name: "?".into(),
                reason: format!("expected i64, got {other:?}"),
            }),
        }
    }

    pub fn as_f64(&self) -> TkucliResult<f64> {
        match self {
            ArgValue::F64(n) => Ok(*n),
            ArgValue::String(s) => s.parse().map_err(|_| TkucliError::InvalidArgument {
                name: "?".into(),
                reason: format!("expected f64, got {s:?}"),
            }),
            other => Err(TkucliError::InvalidArgument {
                name: "?".into(),
                reason: format!("expected f64, got {other:?}"),
            }),
        }
    }

    pub fn as_bool(&self) -> TkucliResult<bool> {
        match self {
            ArgValue::Bool(b) => Ok(*b),
            ArgValue::String(s) => match s.as_str() {
                "true" | "1" | "yes" => Ok(true),
                "false" | "0" | "no" => Ok(false),
                other => Err(TkucliError::InvalidArgument {
                    name: "?".into(),
                    reason: format!("expected bool, got {other:?}"),
                }),
            },
            other => Err(TkucliError::InvalidArgument {
                name: "?".into(),
                reason: format!("expected bool, got {other:?}"),
            }),
        }
    }
}

// ── FromArgs trait ────────────────────────────────────────────────────────────

/// The extractor trait — the CLI equivalent of Axum's `FromRequest`.
///
/// Implement this on your args structs to let the framework automatically
/// pull typed values out of `ParsedArgs` before invoking your handler.
///
/// The codegen derives this for every generated `*Args` struct so you
/// never implement it manually. But you *can* implement it for custom
/// types to use as handler arguments — just like Axum custom extractors.
///
/// # Example (what codegen produces)
///
/// ```rust,ignore
/// // Generated for: verb = "list", flags = [{name="limit", type="u32"}]
/// pub struct ListArgs {
///     pub limit: u32,
///     pub filter: Option<String>,
/// }
///
/// impl FromArgs for ListArgs {
///     fn from_args(args: &ParsedArgs) -> TkucliResult<Self> {
///         Ok(Self {
///             limit:  args.get("limit")
///                        .map(|v| v.as_u32())
///                        .transpose()?
///                        .unwrap_or(20),
///             filter: args.get("filter")
///                        .map(|v| v.as_str().map(|s| s.to_owned()))
///                        .transpose()?,
///         })
///     }
/// }
/// ```
pub trait FromArgs: Sized {
    fn from_args(args: &ParsedArgs) -> TkucliResult<Self>;
}

// ── Blanket impls for primitive types ────────────────────────────────────────
// These let you use bare `String`, `u64`, etc. as single-positional-arg handlers.

impl FromArgs for String {
    fn from_args(args: &ParsedArgs) -> TkucliResult<Self> {
        Ok(args.positional(0)?.to_owned())
    }
}

impl FromArgs for u64 {
    fn from_args(args: &ParsedArgs) -> TkucliResult<Self> {
        args.positional(0)?
            .parse()
            .map_err(|_| TkucliError::InvalidArgument {
                name: "positional[0]".into(),
                reason: "expected u64".into(),
            })
    }
}

impl FromArgs for u32 {
    fn from_args(args: &ParsedArgs) -> TkucliResult<Self> {
        args.positional(0)?
            .parse()
            .map_err(|_| TkucliError::InvalidArgument {
                name: "positional[0]".into(),
                reason: "expected u32".into(),
            })
    }
}

/// The unit type — for operations with no arguments at all.
impl FromArgs for () {
    fn from_args(_args: &ParsedArgs) -> TkucliResult<Self> {
        Ok(())
    }
}

// ── Optional wrapper ─────────────────────────────────────────────────────────

/// Extractor wrapper that makes an entire args struct optional.
/// If extraction fails, yields `None` rather than an error.
/// Useful for operations where all flags are optional.
pub struct Optional<T>(pub Option<T>);

impl<T: FromArgs> FromArgs for Optional<T> {
    fn from_args(args: &ParsedArgs) -> TkucliResult<Self> {
        Ok(Optional(T::from_args(args).ok()))
    }
}
