use clap::Args;
use std::path::PathBuf;

#[derive(Args)]
pub struct BuildArgs {
    /// Path to the config file [default: cli.toml]
    #[arg(long, default_value = "cli.toml")]
    pub config: PathBuf,
    /// Output directory for generated files
    #[arg(long)]
    pub out_dir: Option<PathBuf>,
    /// Select a named TUI profile to bake into the generated app
    #[arg(long)]
    pub tui_profile: Option<String>,
}

pub async fn run(args: BuildArgs) -> anyhow::Result<()> {
    println!("⚙  Reading {}", args.config.display());

    let schema =
        tku_core::schema::AppSchema::from_file(&args.config).map_err(|e| anyhow::anyhow!(e))?;

    tku_codegen::SchemaValidator::new(&schema).validate()?;
    schema
        .resolve_tui_profile(args.tui_profile.as_deref())
        .map_err(|e| anyhow::anyhow!(e))?;

    let out_dir = args
        .out_dir
        .unwrap_or_else(|| PathBuf::from("target/tkucli_generated"));

    std::fs::create_dir_all(&out_dir)?;

    let gen = tku_codegen::CodeGenerator::new(&schema, args.tui_profile.clone());
    for (filename, content) in gen.generate_all() {
        let dest = out_dir.join(&filename);
        std::fs::write(&dest, &content)?;
        println!("  ✓ wrote {}", dest.display());
    }

    println!("✓ Build complete → {}", out_dir.display());
    Ok(())
}
