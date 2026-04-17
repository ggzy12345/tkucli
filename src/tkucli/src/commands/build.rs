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
}

pub async fn run(args: BuildArgs) -> anyhow::Result<()> {
    println!("⚙  Reading {}", args.config.display());

    let schema =
        tku_core::schema::AppSchema::from_file(&args.config).map_err(|e| anyhow::anyhow!(e))?;

    tku_codegen::SchemaValidator::new(&schema).validate()?;

    let out_dir = args
        .out_dir
        .unwrap_or_else(|| PathBuf::from("target/tkucli_generated"));

    std::fs::create_dir_all(&out_dir)?;

    let gen = tku_codegen::CodeGenerator::new(&schema);
    for (filename, content) in gen.generate_all() {
        let dest = out_dir.join(&filename);
        std::fs::write(&dest, &content)?;
        println!("  ✓ wrote {}", dest.display());
    }

    println!("✓ Build complete → {}", out_dir.display());
    Ok(())
}
