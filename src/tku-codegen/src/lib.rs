mod generator;
mod validator;

pub use generator::CodeGenerator;
pub use validator::SchemaValidator;

use tku_core::schema::AppSchema;
use std::path::{Path, PathBuf};

/// Main entry point — call from your project's `build.rs`.
///
/// ```rust,no_run
/// // build.rs
/// fn main() {
///     tku_codegen::build("cli.toml").unwrap();
/// }
/// ```
pub fn build(config_path: impl AsRef<Path>) -> anyhow::Result<()> {
    let config_path = config_path.as_ref();

    println!("cargo:rerun-if-changed={}", config_path.display());

    let schema = AppSchema::from_file(config_path)
        .map_err(|e| anyhow::anyhow!("Failed to parse config: {e}"))?;

    SchemaValidator::new(&schema).validate()?;

    let out_dir: PathBuf = std::env::var("OUT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("target/tkucli_generated"));

    std::fs::create_dir_all(&out_dir)?;

    let gen = CodeGenerator::new(&schema);

    // Writes: commands.rs, args.rs, handler_traits.rs, router.rs
    for (filename, content) in gen.generate_all() {
        let dest = out_dir.join(filename);
        std::fs::write(&dest, content)?;
    }

    Ok(())
}
