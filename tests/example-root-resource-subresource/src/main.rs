mod handlers;

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
use generated::commands::{Cli, extract_dispatch};
use generated::router::{build_router, Handlers};

#[derive(Clone)]
struct AppHandlers {
    root: handlers::root::RootHandler,
    vm: handlers::vm::VmHandler,
    vm_disk: handlers::vm_disk::VmDiskHandler,
}

impl Handlers for AppHandlers {
    type RootHandlerType = handlers::root::RootHandler;
    type VmHandlerType = handlers::vm::VmHandler;
    type VmDiskHandlerType = handlers::vm_disk::VmDiskHandler;

    fn root_handler(&self) -> Self::RootHandlerType {
        self.root.clone()
    }

    fn vm_handler(&self) -> Self::VmHandlerType {
        self.vm.clone()
    }

    fn vm_disk_handler(&self) -> Self::VmDiskHandlerType {
        self.vm_disk.clone()
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let ctx = CtxBuilder::default()
        .format(cli.format.parse::<RenderFormat>().unwrap_or_default())
        .tui_mode(cli.tui)
        .build();

    let svc = build_router(AppHandlers {
        root: handlers::root::RootHandler,
        vm: handlers::vm::VmHandler,
        vm_disk: handlers::vm_disk::VmDiskHandler,
    });

    if ctx.tui_mode() {
        let schema = AppSchema::from_toml(include_str!("../cli.toml"))
            .map_err(anyhow::Error::msg)?;
        let theme = Theme::from_name(&schema.tui.theme);
        let app = TuiApp::from_schema(theme, &schema, svc.clone(), ctx.clone());
        return app.run().await;
    }

    let command = cli.command.ok_or_else(|| anyhow::anyhow!(
        "a subcommand is required unless --tui is set"
    ))?;
    // resource will be "$root" for root verbs, a resource name otherwise
    let (resource, verb, args) = extract_dispatch(command);
    let req = CliRequest::new(ctx.clone(), resource, verb, args);

    let output = svc.call(req).await?;
    println!("{}", output.render(ctx.format()));
    Ok(())
}
