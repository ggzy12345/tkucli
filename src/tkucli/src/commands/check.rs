use clap::Args;
use std::path::PathBuf;

#[derive(Args)]
pub struct CheckArgs {
    /// Path to the config file [default: cli.toml]
    #[arg(long, default_value = "cli.toml")]
    pub config: PathBuf,
}

pub async fn run(args: CheckArgs) -> anyhow::Result<()> {
    println!("🔍 Checking {}", args.config.display());

    let schema = tku_core::schema::AppSchema::from_file(&args.config)
        .map_err(|e| anyhow::anyhow!(e))?;

    tku_codegen::SchemaValidator::new(&schema).validate()?;

    println!("✓ Config is valid.");
    println!(
        "  {} resource(s), {} total operation(s)",
        schema.total_resources(),
        schema.total_operations(),
    );
    Ok(())
}
