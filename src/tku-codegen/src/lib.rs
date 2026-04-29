mod generator;
mod validator;

pub use generator::CodeGenerator;
pub use validator::SchemaValidator;

use std::path::{Path, PathBuf};
use tku_core::schema::AppSchema;

#[derive(Debug, Clone, Default)]
pub struct BuildOptions {
    pub tui_profile: Option<String>,
}

impl BuildOptions {
    pub fn with_tui_profile(mut self, profile: impl Into<String>) -> Self {
        self.tui_profile = Some(profile.into());
        self
    }
}

/// Main entry point — call from your project's `build.rs`.
///
/// ```rust,no_run
/// // build.rs
/// fn main() {
///     tku_codegen::build("cli.toml").unwrap();
/// }
/// ```
pub fn build(config_path: impl AsRef<Path>) -> anyhow::Result<()> {
    let mut options = BuildOptions::default();
    if let Ok(profile) = std::env::var("TKU_TUI_PROFILE") {
        let profile = profile.trim();
        if !profile.is_empty() {
            options.tui_profile = Some(profile.to_string());
            println!("cargo:rerun-if-env-changed=TKU_TUI_PROFILE");
        }
    } else {
        println!("cargo:rerun-if-env-changed=TKU_TUI_PROFILE");
    }
    build_with_options(config_path, options)
}

pub fn build_with_options(
    config_path: impl AsRef<Path>,
    options: BuildOptions,
) -> anyhow::Result<()> {
    let config_path = config_path.as_ref();

    println!("cargo:rerun-if-changed={}", config_path.display());

    let schema = AppSchema::from_file(config_path)
        .map_err(|e| anyhow::anyhow!("Failed to parse config: {e}"))?;

    SchemaValidator::new(&schema).validate()?;
    schema
        .resolve_tui_profile(options.tui_profile.as_deref())
        .map_err(|e| anyhow::anyhow!("Failed to resolve TUI profile: {e}"))?;

    let out_dir: PathBuf = std::env::var("OUT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("target/tkucli_generated"));

    std::fs::create_dir_all(&out_dir)?;

    let gen = CodeGenerator::new(&schema, options.tui_profile.clone());

    // Writes: commands.rs, args.rs, handler_traits.rs, router.rs, tui.rs
    for (filename, content) in gen.generate_all() {
        let dest = out_dir.join(filename);
        std::fs::write(&dest, content)?;
    }

    Ok(())
}
