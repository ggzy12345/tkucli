use serde::{Deserialize, Serialize};

/// Top-level deserialized config (`cli.toml`).
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

/// TUI configuration in `cli.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TuiConfig {
    /// Whether the `--tui` flag is available at runtime.
    #[serde(default)]
    pub enabled: bool,
    /// Color palette: `dark` | `light`
    #[serde(default = "default_theme")]
    pub theme: String,
    /// The screen to show by default (e.g. "Chat").
    pub default_screen: Option<String>,
    /// The name of the profile to use by default.
    /// TOML key: `profile = "default"` | `"coder"` | any named profile
    #[serde(rename = "profile", default)]
    pub default_profile: Option<String>,
    /// Custom labels for the TUI.
    #[serde(default)]
    pub labels: TuiLabels,
    /// Named profiles that override theme/labels.
    /// TOML key: `[[tui.profiles]]`
    #[serde(default)]
    pub profiles: Vec<TuiProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TuiLabels {
    pub running: Option<String>,
    pub latest: Option<String>,
    pub welcome_title: Option<String>,
    pub welcome_body: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TuiProfile {
    pub name: String,
    pub theme: Option<String>,
    pub default_screen: Option<String>,
    #[serde(default)]
    pub labels: TuiLabels,
}

fn default_theme() -> String {
    "dark".into()
}

pub fn is_builtin_tui_profile(name: &str) -> bool {
    matches!(name, "default" | "coder")
}

impl Default for TuiConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            theme: default_theme(),
            default_screen: None,
            default_profile: None,
            labels: TuiLabels::default(),
            profiles: vec![],
        }
    }
}

/// The result of resolving a TUI profile (merging defaults + profile overrides).
pub struct ResolvedTuiProfile {
    pub theme: String,
    pub default_screen: Option<String>,
    pub labels: ResolvedTuiLabels,
}

pub struct ResolvedTuiLabels {
    pub running: String,
    pub latest: String,
    pub welcome_title: String,
    pub welcome_body: String,
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
    pub values: Option<Vec<String>>,
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

    pub fn resolve_tui_profile(&self, name: Option<&str>) -> Result<ResolvedTuiProfile, String> {
        let name = name.or(self.tui.default_profile.as_deref());

        let (p_theme, p_screen, p_labels) = if let Some(n) = name {
            if is_builtin_tui_profile(n) {
                static EMPTY: TuiLabels = TuiLabels {
                    running: None,
                    latest: None,
                    welcome_title: None,
                    welcome_body: None,
                };
                (None, None, &EMPTY)
            } else {
                let p = self
                    .tui
                    .profiles
                    .iter()
                    .find(|p| p.name == n)
                    .ok_or_else(|| format!("TUI profile '{}' not found", n))?;
                (p.theme.as_deref(), p.default_screen.as_deref(), &p.labels)
            }
        } else {
            // No profile selected — use empty override labels so the
            // or_else chain below correctly falls through to global labels.
            static EMPTY: TuiLabels = TuiLabels {
                running: None,
                latest: None,
                welcome_title: None,
                welcome_body: None,
            };
            (None, None, &EMPTY)
        };

        let theme = p_theme.unwrap_or(&self.tui.theme).to_string();
        let default_screen = p_screen
            .or(self.tui.default_screen.as_deref())
            .map(|s| s.to_string());

        let labels = ResolvedTuiLabels {
            running: p_labels
                .running
                .clone()
                .or_else(|| self.tui.labels.running.clone())
                .unwrap_or_else(|| "running".to_string()),
            latest: p_labels
                .latest
                .clone()
                .or_else(|| self.tui.labels.latest.clone())
                .unwrap_or_else(|| "latest".to_string()),
            welcome_title: p_labels
                .welcome_title
                .clone()
                .or_else(|| self.tui.labels.welcome_title.clone())
                .unwrap_or_else(|| "tkucli".to_string()),
            welcome_body: p_labels
                .welcome_body
                .clone()
                .or_else(|| self.tui.labels.welcome_body.clone())
                .unwrap_or_else(|| {
                    "Welcome to Tkucli TUI.\n\n\
                     1. Move through actions below with j/k or the arrow keys.\n\
                     2. Press Enter to run the selected action.\n\
                     3. Results will appear here in the same conversation.\n\
                     4. Use Ctrl-U / Ctrl-D or PageUp / PageDown to scroll through history."
                        .to_string()
                }),
        };

        Ok(ResolvedTuiProfile {
            theme,
            default_screen,
            labels,
        })
    }

    pub fn from_toml(src: &str) -> Result<Self, String> {
        toml::from_str(src).map_err(|e| e.to_string())
    }

    pub fn from_file(path: &std::path::Path) -> Result<Self, String> {
        let src = std::fs::read_to_string(path)
            .map_err(|e| format!("could not read {}: {e}", path.display()))?;
        match path.extension().and_then(|e| e.to_str()) {
            Some("toml") => Self::from_toml(&src),
            other => Err(format!("unsupported config format: {other:?}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::is_builtin_tui_profile;

    #[test]
    fn built_in_tui_profiles_are_recognized() {
        assert!(is_builtin_tui_profile("default"));
        assert!(is_builtin_tui_profile("coder"));
        assert!(!is_builtin_tui_profile("custom"));
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
