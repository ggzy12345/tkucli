mod handlers;
mod shell;

mod generated {
    pub mod args {
        include!(concat!(env!("OUT_DIR"), "/args.rs"));
    }
    pub mod commands {
        include!(concat!(env!("OUT_DIR"), "/commands.rs"));
    }
    pub mod handler_traits {
        include!(concat!(env!("OUT_DIR"), "/handler_traits.rs"));
    }
    pub mod router {
        include!(concat!(env!("OUT_DIR"), "/router.rs"));
    }
}

use clap::Parser;
use tku_core::{
    context::CtxBuilder,
    handler::CliRequest,
    output::RenderFormat,
    schema::AppSchema,
};
use tku_tui::{Theme, TuiApp};
use tku_tui::screen::ScreenLabels;
use generated::commands::{Cli, extract_dispatch};
use generated::router::{build_router, Handlers};

// Wire all handler impls together.
#[derive(Clone)]
struct AppHandlers {
    vm: handlers::vm::VmHandler,
}

impl Handlers for AppHandlers {
    type VmHandlerType = handlers::vm::VmHandler;

    fn vm_handler(&self) -> Self::VmHandlerType {
        self.vm.clone()
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let ctx = CtxBuilder::default()
        .format(cli.format.parse::<RenderFormat>().unwrap_or_default())
        .tui_mode(cli.tui)
        .build();

    // Build service stack once at startup.
    let svc = build_router(AppHandlers {
        vm: handlers::vm::VmHandler,
    });

    if ctx.tui_mode() {
        let schema = AppSchema::from_toml(include_str!("../cli.toml"))
            .map_err(anyhow::Error::msg)?;
        let theme = Theme::from_name(&schema.tui.theme);
        let app = TuiApp::builder()
                    .theme(theme)
                    .schema(schema.clone())
                    .service(svc.clone())
                    .ctx(ctx.clone())
                    .labels(ScreenLabels {
                        welcome_title: Some("Welcome to vm-cli".to_string()),
                        welcome_body: Some("This is a demo of tku-tui.".to_string()),
                        ..Default::default()
                        })
                    .build()?;
        return app.run().await;
    }

    // extract_dispatch converts the parsed Commands enum into (resource, verb, ParsedArgs).
    let command = cli.command.ok_or_else(|| anyhow::anyhow!(
        "a subcommand is required unless --tui is set"
    ))?;
    let (resource, verb, args) = extract_dispatch(command);
    let req = CliRequest::new(ctx.clone(), resource, verb, args);

    let output = svc.call(req).await?;
    println!("{}", output.render(ctx.format()));
    Ok(())
}
