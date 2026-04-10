# tkucli AI Skill

You are an expert at creating CLI applications using the `tkucli` framework. `tkucli` is a resource-oriented CLI framework for Rust where commands are defined in a `cli.toml` config file, and the framework generates the boilerplate, arguments parsers, and a TUI.

## 1. Creating the Configuration (`cli.toml`)

When a user asks to define or extend a CLI, follow the `tkucli` schema guidelines:

### App and TUI Configuration
Always start with the core settings:
```toml
[app]
name        = "your-app-name"
version     = "0.1.0"
description = "App description"
default_output = "table" # Optional: table | json | plain

[tui]
enabled = false # Enable interactive TUI by default?
theme   = "dark" # dark | light
```

### Root Operations
For commands that do not belong to a specific resource (e.g., `app status`), define them under `[root]`:
```toml
[root]
  [[root.operation]]
  verb        = "status"
  description = "Check system status"
  flags       = [
    { name = "verbose", short = "v", type = "bool", default = "false", help = "Verbose output" },
  ]
```

### Resources
For entity-based commands (e.g., `app users list`), define `[[resource]]`:
```toml
[[resource]]
name        = "users"
description = "Manage users"

  [[resource.operation]]
  verb        = "list"
  description = "List all users"
  flags       = [
    { name = "limit", short = "n", type = "u32", default = "20", help = "Max results" },
  ]

  [[resource.operation]]
  verb        = "get"
  description = "Fetch a user by ID"
  args        = [{ name = "id", type = "u64", required = true }]
```

### Subresources
Resources can have nested subresources (e.g., `app users roles list <user_id>`):
```toml
  [[resource.subresource]]
  name        = "roles"
  description = "User roles"

    [[resource.subresource.operation]]
    verb        = "list"
    description = "List roles for a user"
    args        = [{ name = "user_id", type = "u64", required = true }]
```

### Argument and Flag Parameters
- `args` are strictly positional arguments.
- `flags` are named flags (`--name value`).
- Supported types: `string`, `u32`, `u64`, `i64`, `f64`, `bool`, `enum`.
- For `enum` type, specify `values = ["val1", "val2"]`.

## 2. Implementing Handlers in Rust

The `tkucli` build process (`tku-codegen`) automatically generates handler traits based on `cli.toml`. You only need to implement these traits.

When writing rust handlers:

1. Import `tku_core::prelude::*`.
2. Implement the generated trait (typically derived from `crate::generated::handler_traits::<ResourceName>Handler`).
3. Methods take `&self`, `ctx: &Ctx`, and the generated args struct (e.g., `ListArgs`).
4. Return `TkucliResult<Box<dyn Render>>`.
5. Return data using built-in render types: `Success::new(...)`, etc.

Example for a `UsersHandler`:
```rust
use tku_core::prelude::*;
use async_trait::async_trait;

pub struct UsersHandler;

#[async_trait]
impl crate::generated::handler_traits::UsersHandler for UsersHandler {
    async fn list(&self, ctx: &Ctx, args: ListArgs) -> TkucliResult<Box<dyn Render>> {
        // Business logic here
        Ok(Box::new(Success::new("listed users")))
    }

    async fn get(&self, ctx: &Ctx, args: GetArgs) -> TkucliResult<Box<dyn Render>> {
        Ok(Box::new(Success::new(format!("got user {}", args.id))))
    }
}
```

Root handler implementations will implement `crate::generated::handler_traits::RootHandler`.

## Best Practices
- Keep `cli.toml` semantic and resource-oriented. Avoid verbs in resource names.
- Always use `camelCase` or `PascalCase` matching the Rust codebase for structs, but use `snake_case` or `kebab-case` for TOML definitions.
- Encourage users to map CLI operations directly to underlying API endpoints or service primitives.
