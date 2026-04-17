use serde::Serialize;
use std::fmt;

// ── IntoOutput ────────────────────────────────────────────────────────────────

/// The output conversion trait — the CLI equivalent of Axum's `IntoResponse`.
///
/// Handlers return `impl IntoOutput`. The framework calls `.into_output()`
/// to get a type-erased `Box<dyn Render>` without the handler ever
/// having to write `Box::new(...)`.
///
/// Anything that implements `Render` automatically implements `IntoOutput`
/// via the blanket impl below. You can also implement it directly for
/// types that need custom boxing logic.
pub trait IntoOutput: Send {
    fn into_output(self) -> Box<dyn Render>;
}

/// Blanket impl — every `Render` value is automatically `IntoOutput`.
impl<T: Render + 'static> IntoOutput for T {
    fn into_output(self) -> Box<dyn Render> {
        Box::new(self)
    }
}

/// Already-boxed renders pass through unchanged.
impl IntoOutput for Box<dyn Render> {
    fn into_output(self) -> Box<dyn Render> {
        self
    }
}

// ── RenderFormat ──────────────────────────────────────────────────────────────

/// All possible output formats a user can request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RenderFormat {
    #[default]
    Table,
    Json,
    Plain,
    /// Output is suppressed; only the exit code matters.
    Quiet,
}

impl fmt::Display for RenderFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Table => write!(f, "table"),
            Self::Json => write!(f, "json"),
            Self::Plain => write!(f, "plain"),
            Self::Quiet => write!(f, "quiet"),
        }
    }
}

impl std::str::FromStr for RenderFormat {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "table" => Ok(Self::Table),
            "json" => Ok(Self::Json),
            "plain" => Ok(Self::Plain),
            "quiet" => Ok(Self::Quiet),
            other => Err(format!("unknown format: {other}")),
        }
    }
}

/// Any value that can be rendered to the terminal.
/// Handlers return `impl Render`; the output engine calls the
/// correct variant based on the active `RenderFormat`.
pub trait Render: Send + Sync {
    fn render_table(&self) -> String;
    fn render_json(&self) -> String;
    fn render_plain(&self) -> String;

    fn render(&self, format: RenderFormat) -> String {
        match format {
            RenderFormat::Table => self.render_table(),
            RenderFormat::Json => self.render_json(),
            RenderFormat::Plain => self.render_plain(),
            RenderFormat::Quiet => String::new(),
        }
    }
}

// ── Built-in renderers ────────────────────────────────────────────────────────

/// Generic success message.
pub struct Success {
    pub message: String,
}

impl Success {
    pub fn new(msg: impl Into<String>) -> Self {
        Self {
            message: msg.into(),
        }
    }
}

impl Render for Success {
    fn render_table(&self) -> String {
        format!("✓ {}", self.message)
    }
    fn render_json(&self) -> String {
        format!(r#"{{"ok":true,"message":"{}"}}"#, self.message)
    }
    fn render_plain(&self) -> String {
        self.message.clone()
    }
}

/// Generic tabular data wrapper. `T` must be `Serialize` so we can
/// produce JSON, and must implement `tabled::Tabled` for table output.
pub struct Table<T> {
    pub rows: Vec<T>,
}

impl<T: Serialize + tabled::Tabled + Send + Sync> Table<T> {
    pub fn from(rows: Vec<T>) -> Self {
        Self { rows }
    }
}

impl<T: Serialize + tabled::Tabled + Send + Sync> Render for Table<T> {
    fn render_table(&self) -> String {
        tabled::Table::new(&self.rows).to_string()
    }

    fn render_json(&self) -> String {
        serde_json::to_string_pretty(&self.rows)
            .unwrap_or_else(|e| format!("{{\"error\":\"{e}\"}}"))
    }

    fn render_plain(&self) -> String {
        self.rows
            .iter()
            .map(|r| serde_json::to_string(r).unwrap_or_default())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// A single key/value record.
pub struct Record {
    pub fields: Vec<(String, String)>,
}

impl Record {
    pub fn new(fields: Vec<(impl Into<String>, impl Into<String>)>) -> Self {
        Self {
            fields: fields
                .into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        }
    }
}

impl Render for Record {
    fn render_table(&self) -> String {
        self.fields
            .iter()
            .map(|(k, v)| format!("{:<20} {}", k, v))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn render_json(&self) -> String {
        let obj: serde_json::Map<String, serde_json::Value> = self
            .fields
            .iter()
            .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
            .collect();
        serde_json::to_string_pretty(&obj).unwrap_or_default()
    }

    fn render_plain(&self) -> String {
        self.fields
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join("\n")
    }
}
