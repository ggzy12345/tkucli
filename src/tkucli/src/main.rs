mod commands;

use clap::{Parser, Subcommand};
use tracing_subscriber::{fmt, EnvFilter};

#[derive(Parser)]
#[command(
    name    = "tkucli",
    about   = "The Tkucli CLI framework toolkit",
    version = env!("CARGO_PKG_VERSION"),
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Scaffold a new Tkucli project
    New(commands::new::NewArgs),
    /// Validate cli.toml and generate Rust source into target/tkucli_generated
    Build(commands::build::BuildArgs),
    /// Validate cli.toml without generating any files
    Check(commands::check::CheckArgs),
    /// Print a starter cli.toml / cli.yaml skeleton to stdout
    Init(commands::init::InitArgs),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let filter = if cli.verbose { "debug" } else { "info" };
    fmt()
        .with_env_filter(EnvFilter::new(filter))
        .with_target(false)
        .init();

    match cli.command {
        Commands::New(args)   => commands::new::run(args).await,
        Commands::Build(args) => commands::build::run(args).await,
        Commands::Check(args) => commands::check::run(args).await,
        Commands::Init(args)  => commands::init::run(args).await,
    }
}
