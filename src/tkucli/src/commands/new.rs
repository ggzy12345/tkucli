use clap::Args;
use std::path::PathBuf;

#[derive(Args)]
pub struct NewArgs {
    /// Project name
    pub name: String,
    /// Directory to create (defaults to `./<name>`)
    #[arg(long)]
    pub path: Option<PathBuf>,
    /// Scaffold a starter that includes a sub-resource example (`vm disk ...`)
    #[arg(long)]
    pub subresource_example: bool,
    /// Scaffold a single-resource CLI where verbs sit at the top level (no resource prefix)
    #[arg(long)]
    pub root_example: bool,
}

pub async fn run(args: NewArgs) -> anyhow::Result<()> {
    let root = args.path.unwrap_or_else(|| PathBuf::from(&args.name));
    let name = &args.name;
    let framework_version = env!("CARGO_PKG_VERSION");
    let use_subresource_example = args.subresource_example;
    let use_root_example = args.root_example;

    println!(
        "🔨 Creating new Tkucli project `{name}` at {}",
        root.display()
    );

    std::fs::create_dir_all(root.join("src/handlers"))?;

    // ── Cargo.toml ────────────────────────────────────────────────────────────
    std::fs::write(
        root.join("Cargo.toml"),
        format!(
            r#"[package]
name    = "{name}"
version = "0.1.0"
edition = "2021"

[dependencies]
tku-core      = "{framework_version}"
tku-tui       = "{framework_version}"
tku-macros    = "{framework_version}"
tokio         = {{ version = "1", features = ["full"] }}
serde         = {{ version = "1", features = ["derive"] }}
clap          = {{ version = "4", features = ["derive"] }}
tabled        = "0.15"
async-trait   = "0.1"
anyhow        = "1"

[build-dependencies]
tku-codegen = "{framework_version}"
"#,
        ),
    )?;

    // ── build.rs ──────────────────────────────────────────────────────────────
    std::fs::write(
        root.join("build.rs"),
        r#"fn main() {
    tku_codegen::build("cli.toml").expect("tkucli build failed");
}
"#,
    )?;

    // ── cli.toml ──────────────────────────────────────────────────────────────
    std::fs::write(
        root.join("cli.toml"),
        if use_root_example {
            format!(
                r#"[app]
name           = "{name}"
version        = "0.1.0"
description    = "A Tkucli-powered CLI"
default_output = "table"

[tui]
enabled        = false
theme          = "dark"

[root]

  [[root.operation]]
  verb        = "list"
  description = "List items"
  flags       = [
    {{ name = "limit", short = "n", type = "u32", default = "20", help = "Max results" }},
  ]

  [[root.operation]]
  verb        = "get"
  description = "Get an item by ID"
  args        = [{{ name = "id", type = "u64", required = true }}]
"#
            )
        } else if use_subresource_example {
            format!(
                r#"[app]
name           = "{name}"
version        = "0.1.0"
description    = "A Tkucli-powered CLI"
default_output = "table"

[tui]
enabled        = true
theme          = "dark"

[[resource]]
name        = "vm"
description = "Virtual machines"

  [[resource.operation]]
  verb        = "list"
  description = "List all virtual machines"
  flags       = [
    {{ name = "limit", short = "n", type = "u32", default = "20", help = "Max results" }},
  ]

  [[resource.operation]]
  verb        = "get"
  description = "Get a virtual machine by ID"
  args        = [{{ name = "id", type = "u64", required = true }}]

  [[resource.subresource]]
  name        = "disk"
  description = "Virtual machine disks"

    [[resource.subresource.operation]]
    verb        = "list"
    description = "List disks for a VM"
    args        = [{{ name = "vm_id", type = "u64", required = true }}]

    [[resource.subresource.operation]]
    verb        = "attach"
    description = "Attach a disk to a VM"
    args        = [
      {{ name = "vm_id", type = "u64", required = true }},
      {{ name = "disk_id", type = "u64", required = true }},
    ]
"#
            )
        } else {
            format!(
                r#"[app]
name           = "{name}"
version        = "0.1.0"
description    = "A Tkucli-powered CLI"
default_output = "table"

[tui]
enabled        = true
theme          = "dark"

[[resource]]
name        = "example"
description = "Example resource — replace with your own"

  [[resource.operation]]
  verb        = "list"
  description = "List all examples"
  flags       = [
    {{ name = "limit", short = "n", type = "u32", default = "20", help = "Max results" }},
  ]

  [[resource.operation]]
  verb        = "get"
  description = "Get an example by ID"
  args        = [{{ name = "id", type = "u64", required = true }}]
"#
            )
        },
    )?;

    // ── src/main.rs ───────────────────────────────────────────────────────────
    std::fs::write(
        root.join("src/main.rs"),
        if use_root_example {
            r#"mod handlers;

mod generated {{
    pub mod args {{
        include!(concat!(env!("OUT_DIR"), "/args.rs"));
    }}
    pub mod commands {{
        include!(concat!(env!("OUT_DIR"), "/commands.rs"));
    }}
    pub mod handler_traits {{
        include!(concat!(env!("OUT_DIR"), "/handler_traits.rs"));
    }}
    pub mod router {{
        include!(concat!(env!("OUT_DIR"), "/router.rs"));
    }}
}}

use clap::Parser;
use tku_core::{{
    context::CtxBuilder,
    handler::CliRequest,
    output::RenderFormat,
    schema::AppSchema,
}};
use tku_tui::{{Theme, TuiApp}};
use generated::commands::{{Cli, extract_dispatch}};
use generated::router::{{build_router, Handlers}};

#[derive(Clone)]
struct AppHandlers {{
    root: handlers::root::RootHandler,
}}

impl Handlers for AppHandlers {{
    type RootHandlerType = handlers::root::RootHandler;

    fn root_handler(&self) -> Self::RootHandlerType {{
        self.root.clone()
    }}
}}

#[tokio::main]
async fn main() -> anyhow::Result<()> {{
    let cli = Cli::parse();

    let ctx = CtxBuilder::default()
        .format(cli.format.parse::<RenderFormat>().unwrap_or_default())
        .tui_mode(cli.tui)
        .build();

    let svc = build_router(AppHandlers {{
        root: handlers::root::RootHandler,
    }});

    if ctx.tui_mode() {{
        let schema = AppSchema::from_toml(include_str!("../cli.toml"))
            .map_err(anyhow::Error::msg)?;
        let theme = Theme::from_name(&schema.tui.theme);
        let app = TuiApp::from_schema(theme, &schema, svc.clone(), ctx.clone());
        return app.run().await;
    }}

    let command = cli.command.ok_or_else(|| anyhow::anyhow!(
        "a subcommand is required unless --tui is set"
    ))?;
    // resource will be "$root" for root verbs, a resource name otherwise
    let (resource, verb, args) = extract_dispatch(command);
    let req = CliRequest::new(ctx.clone(), resource, verb, args);

    let output = svc.call(req).await?;
    println!("{{}}", output.render(ctx.format()));
    Ok(())
}}
"#
            .to_string()
        } else if use_subresource_example {
            r#"mod handlers;

mod generated {{
    pub mod args {{
        include!(concat!(env!("OUT_DIR"), "/args.rs"));
    }}
    pub mod commands {{
        include!(concat!(env!("OUT_DIR"), "/commands.rs"));
    }}
    pub mod handler_traits {{
        include!(concat!(env!("OUT_DIR"), "/handler_traits.rs"));
    }}
    pub mod router {{
        include!(concat!(env!("OUT_DIR"), "/router.rs"));
    }}
}}

use clap::Parser;
use tku_core::{{
    context::CtxBuilder,
    handler::CliRequest,
    output::RenderFormat,
    schema::AppSchema,
}};
use tku_tui::{{Theme, TuiApp}};
use generated::commands::{{Cli, extract_dispatch}};
use generated::router::{{build_router, Handlers}};

#[derive(Clone)]
struct AppHandlers {{
    vm: handlers::vm::VmHandler,
    vm_disk: handlers::vm_disk::VmDiskHandler,
}}

impl Handlers for AppHandlers {{
    type VmHandlerType = handlers::vm::VmHandler;
    type VmDiskHandlerType = handlers::vm_disk::VmDiskHandler;

    fn vm_handler(&self) -> Self::VmHandlerType {{
        self.vm.clone()
    }}

    fn vm_disk_handler(&self) -> Self::VmDiskHandlerType {{
        self.vm_disk.clone()
    }}
}}

#[tokio::main]
async fn main() -> anyhow::Result<()> {{
    let cli = Cli::parse();

    let ctx = CtxBuilder::default()
        .format(cli.format.parse::<RenderFormat>().unwrap_or_default())
        .tui_mode(cli.tui)
        .build();

    let svc = build_router(AppHandlers {{
        vm: handlers::vm::VmHandler,
        vm_disk: handlers::vm_disk::VmDiskHandler,
    }});

    if ctx.tui_mode() {{
        let schema = AppSchema::from_toml(include_str!("../cli.toml"))
            .map_err(anyhow::Error::msg)?;
        let theme = Theme::from_name(&schema.tui.theme);
        let app = TuiApp::from_schema(theme, &schema, svc.clone(), ctx.clone());
        return app.run().await;
    }}

    let command = cli.command.ok_or_else(|| anyhow::anyhow!(
        "a subcommand is required unless --tui is set"
    ))?;
    let (resource, verb, args) = extract_dispatch(command);
    let req = CliRequest::new(ctx.clone(), resource, verb, args);

    let output = svc.call(req).await?;
    println!("{{}}", output.render(ctx.format()));
    Ok(())
}}
"#
            .to_string()
        } else {
            r#"mod handlers;

mod generated {{
    pub mod args {{
        include!(concat!(env!("OUT_DIR"), "/args.rs"));
    }}
    pub mod commands {{
        include!(concat!(env!("OUT_DIR"), "/commands.rs"));
    }}
    pub mod handler_traits {{
        include!(concat!(env!("OUT_DIR"), "/handler_traits.rs"));
    }}
    pub mod router {{
        include!(concat!(env!("OUT_DIR"), "/router.rs"));
    }}
}}

use clap::Parser;
use tku_core::{{
    context::CtxBuilder,
    handler::CliRequest,
    output::RenderFormat,
    schema::AppSchema,
}};
use tku_tui::{{Theme, TuiApp}};
use generated::commands::{{Cli, extract_dispatch}};
use generated::router::{{build_router, Handlers}};

// Wire all handler impls together.
#[derive(Clone)]
struct AppHandlers {{
    example: handlers::example::ExampleHandler,
}}

impl Handlers for AppHandlers {{
    type ExampleHandlerType = handlers::example::ExampleHandler;

    fn example_handler(&self) -> Self::ExampleHandlerType {{
        self.example.clone()
    }}
}}

#[tokio::main]
async fn main() -> anyhow::Result<()> {{
    let cli = Cli::parse();

    let ctx = CtxBuilder::default()
        .format(cli.format.parse::<RenderFormat>().unwrap_or_default())
        .tui_mode(cli.tui)
        .build();

    // Build service stack once at startup.
    let svc = build_router(AppHandlers {{
        example: handlers::example::ExampleHandler,
    }});

    if ctx.tui_mode() {{
        let schema = AppSchema::from_toml(include_str!("../cli.toml"))
            .map_err(anyhow::Error::msg)?;
        let theme = Theme::from_name(&schema.tui.theme);
        let app = TuiApp::from_schema(theme, &schema, svc.clone(), ctx.clone());
        return app.run().await;
    }}

    // extract_dispatch converts the parsed Commands enum into (resource, verb, ParsedArgs).
    let command = cli.command.ok_or_else(|| anyhow::anyhow!(
        "a subcommand is required unless --tui is set"
    ))?;
    let (resource, verb, args) = extract_dispatch(command);
    let req = CliRequest::new(ctx.clone(), resource, verb, args);

    let output = svc.call(req).await?;
    println!("{{}}", output.render(ctx.format()));
    Ok(())
}}
"#
            .to_string()
        },
    )?;

    // ── src/handlers/mod.rs ───────────────────────────────────────────────────
    std::fs::write(
        root.join("src/handlers/mod.rs"),
        if use_root_example {
            "pub mod root;\n"
        } else if use_subresource_example {
            "pub mod vm;\npub mod vm_disk;\n"
        } else {
            "pub mod example;\n"
        },
    )?;

    if use_root_example {
        std::fs::write(
            root.join("src/handlers/root.rs"),
            r#"use tku_core::prelude::*;
use crate::generated::{args::*, handler_traits::RootHandler as RootHandlerTrait};

#[derive(Clone, Copy, Default)]
pub struct RootHandler;

#[async_trait]
impl RootHandlerTrait for RootHandler {
    async fn list(&self, _ctx: Ctx, args: RootListArgs) -> TkucliResult<impl IntoOutput> {
        Ok(Success::new(format!("list called with limit={}", args.limit)))
    }

    async fn get(&self, _ctx: Ctx, args: RootGetArgs) -> TkucliResult<impl IntoOutput> {
        Ok(Success::new(format!("got id={}", args.id)))
    }
}
"#,
        )?;
    } else if use_subresource_example {
        std::fs::write(
            root.join("src/handlers/vm.rs"),
            r#"use tku_core::prelude::*;
use crate::generated::{args::*, handler_traits::VmHandler as VmHandlerTrait};

#[derive(Clone, Copy, Default)]
pub struct VmHandler;

#[async_trait]
impl VmHandlerTrait for VmHandler {
    async fn list(&self, _ctx: Ctx, args: VmListArgs) -> TkucliResult<impl IntoOutput> {
        Ok(Success::new(format!("vm list called with limit={}", args.limit)))
    }

    async fn get(&self, _ctx: Ctx, args: VmGetArgs) -> TkucliResult<impl IntoOutput> {
        Ok(Success::new(format!("vm get called with id={}", args.id)))
    }
}
"#,
        )?;

        std::fs::write(
            root.join("src/handlers/vm_disk.rs"),
            r#"use tku_core::prelude::*;
use crate::generated::{args::*, handler_traits::VmDiskHandler as VmDiskHandlerTrait};

#[derive(Clone, Copy, Default)]
pub struct VmDiskHandler;

#[async_trait]
impl VmDiskHandlerTrait for VmDiskHandler {
    async fn list(&self, _ctx: Ctx, args: VmDiskListArgs) -> TkucliResult<impl IntoOutput> {
        Ok(Success::new(format!("vm disk list called with vm_id={}", args.vm_id)))
    }

    async fn attach(&self, _ctx: Ctx, args: VmDiskAttachArgs) -> TkucliResult<impl IntoOutput> {
        Ok(Success::new(format!(
            "vm disk attach called with vm_id={} disk_id={}",
            args.vm_id, args.disk_id
        )))
    }
}
"#,
        )?;
    } else {
        // ── src/handlers/example.rs ───────────────────────────────────────────────
        std::fs::write(
            root.join("src/handlers/example.rs"),
            r#"use tku_core::prelude::*;
use crate::generated::{args::*, handler_traits::ExampleHandler as ExampleHandlerTrait};

#[derive(Clone, Copy, Default)]
pub struct ExampleHandler;

#[async_trait]
impl ExampleHandlerTrait for ExampleHandler {
    async fn list(&self, _ctx: Ctx, args: ExampleListArgs) -> TkucliResult<impl IntoOutput> {
        // args.limit is already a typed u32 — no manual parsing needed.
        let _ = args.limit;
        Ok(Success::new("list called — replace with real data"))
    }

    async fn get(&self, _ctx: Ctx, args: ExampleGetArgs) -> TkucliResult<impl IntoOutput> {
        // args.id is already a typed u64.
        Ok(Success::new(format!("got id={}", args.id)))
    }
}
"#,
        )?;
    }

    println!("✓ Project created.");
    println!();
    println!("  cd {}", root.display());
    println!("  cargo build          # runs codegen via build.rs");
    if use_root_example {
        println!("  cargo run -- list");
        println!("  cargo run -- get 42");
    } else if use_subresource_example {
        println!("  cargo run -- vm list");
        println!("  cargo run -- vm disk attach 42 7");
    } else {
        println!("  cargo run -- example list");
    }
    println!("  cargo run -- --tui   # launch interactive TUI");

    Ok(())
}
