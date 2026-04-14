use clap::Args;

#[derive(Args)]
pub struct InitArgs {}

pub async fn run(_args: InitArgs) -> anyhow::Result<()> {
    print!("{}", TOML_TEMPLATE);
    Ok(())
}

const TOML_TEMPLATE: &str = r#"[app]
name           = "my-app"
version        = "0.1.0"
description    = "My Tkucli CLI"
default_output = "table"

[tui]
enabled = true
theme   = "dark"

# Optional: operations with no resource prefix (my-app list, my-app deploy …)
# [root]
#   [[root.operation]]
#   verb        = "status"
#   description = "Show app status"

[[resource]]
name        = "users"
description = "Manage users"

  [[resource.operation]]
  verb        = "list"
  description = "List all users"
  flags = [
    { name = "filter", short = "f", type = "string", help = "Filter by name" },
    { name = "limit",  short = "n", type = "u32",    default = "20" },
  ]

  [[resource.operation]]
  verb        = "get"
  description = "Get a user by ID"
  args = [{ name = "id", type = "u64", required = true }]

  [[resource.operation]]
  verb        = "create"
  description = "Create a new user"
  confirm     = true
  flags = [
    { name = "name",  type = "string", required = true },
    { name = "email", type = "string", required = true },
    { name = "role",  type = "enum", values = ["admin","user","viewer"], default = "user" },
  ]
"#;
