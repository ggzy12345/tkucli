use serde::{Deserialize, Serialize};

/// Top-level deserialized config (`cli.toml` / `cli.yaml`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSchema {
    pub app: AppMeta,
    #[serde(default)]
    pub root: RootConfig,
    #[serde(default)]
    pub tui: TuiConfig,
    #[serde(default)]
    pub middleware: MiddlewareConfig,
    #[serde(rename = "resource", default)]
    pub resources: Vec<ResourceSchema>,
}

/// Operations that belong directly to the root of the CLI
/// (no resource subcommand prefix).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RootConfig {
    #[serde(rename = "operation", default)]
    pub operations: Vec<OperationSchema>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppMeta {
    pub name: String,
    pub version: String,
    pub description: String,
    #[serde(default = "default_output")]
    pub default_output: OutputFormat,
}

fn default_output() -> OutputFormat {
    OutputFormat::Table
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    #[default]
    Table,
    Json,
    Plain,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TuiConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_theme")]
    pub theme: String,
    pub default_screen: Option<String>,
}

fn default_theme() -> String {
    "dark".into()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MiddlewareConfig {
    pub auth: Option<AuthConfig>,
    pub logging: Option<LoggingConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    #[serde(rename = "type")]
    pub auth_type: String,
    pub env: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub file: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceSchema {
    pub name: String,
    pub description: String,
    #[serde(rename = "operation", default)]
    pub operations: Vec<OperationSchema>,
    #[serde(rename = "subresource", default)]
    pub subresources: Vec<ResourceSchema>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationSchema {
    pub verb: String,
    pub description: String,
    #[serde(default)]
    pub args: Vec<ArgSchema>,
    #[serde(default)]
    pub flags: Vec<FlagSchema>,
    /// If true, the framework prompts for confirmation before invoking.
    #[serde(default)]
    pub confirm: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArgSchema {
    pub name: String,
    #[serde(rename = "type")]
    pub arg_type: ArgType,
    #[serde(default)]
    pub required: bool,
    pub help: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlagSchema {
    pub name: String,
    pub short: Option<String>,
    #[serde(rename = "type")]
    pub arg_type: ArgType,
    pub default: Option<String>,
    pub help: Option<String>,
    pub values: Option<Vec<String>>, // for enum types
    #[serde(default)]
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ArgType {
    String,
    U32,
    U64,
    I64,
    F64,
    Bool,
    Enum,
}

impl AppSchema {
    pub fn total_resources(&self) -> usize {
        // root counts as 1 implicit resource when it has operations
        let root_count = if self.root.operations.is_empty() {
            0
        } else {
            1
        };
        root_count
            + self
                .resources
                .iter()
                .map(ResourceSchema::total_resources)
                .sum::<usize>()
    }

    pub fn total_operations(&self) -> usize {
        self.root.operations.len()
            + self
                .resources
                .iter()
                .map(ResourceSchema::total_operations)
                .sum::<usize>()
    }

    /// Load from a TOML string.
    pub fn from_toml(src: &str) -> Result<Self, String> {
        toml::from_str(src).map_err(|e| e.to_string())
    }

    /// Auto-detect format from file extension and parse.
    pub fn from_file(path: &std::path::Path) -> Result<Self, String> {
        let src = std::fs::read_to_string(path)
            .map_err(|e| format!("could not read {}: {e}", path.display()))?;
        match path.extension().and_then(|e| e.to_str()) {
            Some("toml") => Self::from_toml(&src),
            other => Err(format!("unsupported config format: {other:?}")),
        }
    }
}

impl ResourceSchema {
    pub fn total_resources(&self) -> usize {
        1 + self
            .subresources
            .iter()
            .map(ResourceSchema::total_resources)
            .sum::<usize>()
    }

    pub fn total_operations(&self) -> usize {
        self.operations.len()
            + self
                .subresources
                .iter()
                .map(ResourceSchema::total_operations)
                .sum::<usize>()
    }
}
