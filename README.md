# tkucli

**A resource-oriented CLI framework for Rust**, inspired by web frameworks like Express and Spring.

Define your CLI in a single `cli.toml` config file. Tkucli generates the entire command tree, argument parsers, handler trait stubs, and an interactive Ratatui TUI — you write only the business logic.

---

## Workspace layout

```
tkucli/
├── tku-core/       Runtime: router, middleware, output engine, handler traits
├── tku-tui/        Ratatui TUI shell: sidebar, screen stack, widgets, themes
├── tku-macros/     Proc-macros: #[tkucli::handler], tkucli::register!()
├── tku-codegen/    Build-time code generator (reads cli.toml → emits Rust)
└── tkucli/        The `tkucli` developer tool binary (new, build, check, init)
```

---

## Quick start

### 1. Install the `tkucli` tool

```bash
cargo install tkucli
```

### 2. Create a new project

```bash
tkucli new my-app
cd my-app
```

The generated app should depend on the published crates from crates.io, like the examples in [`examples/`](/Volumes/ORICO/projects/github/tkucli/examples):

```toml
[dependencies]
tku-core    = "0.1.0"
tku-tui     = "0.1.0"
tku-macros  = "0.1.0"
tokio       = { version = "1", features = ["full"] }
serde       = { version = "1", features = ["derive"] }
clap        = { version = "4", features = ["derive"] }
tabled      = "0.15"
async-trait = "0.1"
anyhow      = "1"

[build-dependencies]
tku-codegen = "0.1.0"
```

Use `path = ...` dependencies only when you are developing against this repository locally.

### 3. Edit `cli.toml`

```toml
[app]
name        = "my-app"
version     = "0.1.0"
description = "My Tkucli CLI"

[tui]
enabled = true
theme   = "dark"

[root]
  [[root.operation]]
  verb        = "status"
  description = "Check system status"

[[resource]]
name        = "users"
description = "Manage users"

  [[resource.operation]]
  verb        = "list"
  description = "List all users"
  flags = [
    { name = "limit", short = "n", type = "u32", default = "20" },
  ]

  [[resource.operation]]
  verb        = "get"
  description = "Fetch a user by ID"
  args = [{ name = "id", type = "u64", required = true }]

  [[resource.subresource]]
  name        = "roles"
  description = "User roles"

    [[resource.subresource.operation]]
    verb        = "list"
    description = "List roles for a user"
    args = [{ name = "user_id", type = "u64", required = true }]
```

### 4. Implement handlers

`cargo build` runs `build.rs` → `tku_codegen` → generates `handler_traits.rs`. Implement the generated trait:

```rust
// src/handlers/users.rs
use tku_core::prelude::*;

pub struct UsersHandler;

#[async_trait]
impl crate::generated::handler_traits::UsersHandler for UsersHandler {
    async fn list(&self, ctx: &Ctx, args: ListArgs) -> TkucliResult<Box<dyn Render>> {
        // your logic here
        Ok(Box::new(Success::new("listed users")))
    }

    async fn get(&self, ctx: &Ctx, args: GetArgs) -> TkucliResult<Box<dyn Render>> {
        Ok(Box::new(Success::new(format!("got user {}", args.id))))
    }
}
```

### 5. Run

```bash
cargo run -- status
cargo run -- users list
cargo run -- users list --format json
cargo run -- users get 42
cargo run -- users roles list 42
cargo run -- --tui          # interactive TUI
```

---

## Config reference

### `[app]`

| Key              | Type   | Default   | Description                        |
|------------------|--------|-----------|------------------------------------|
| `name`           | string | required  | Binary name                        |
| `version`        | string | required  | Shown in `--version`               |
| `description`    | string | required  | Shown in `--help`                  |
| `default_output` | string | `"table"` | `table` \| `json` \| `plain`       |

### `[tui]`

| Key              | Type   | Default   | Description                        |
|------------------|--------|-----------|------------------------------------|
| `enabled`        | bool   | `false`   | Allow `--tui` flag                 |
| `theme`          | string | `"dark"`  | `dark` \| `light`                  |
| `default_screen` | string | —         | Resource name to show on launch    |

### `[root]`

| Key         | Type  | Description                              |
|-------------|-------|------------------------------------------|
| `operation` | array | List of operations at the root level.    |

### `[[resource]]`

| Key           | Type   | Description                              |
|---------------|--------|------------------------------------------|
| `name`        | string | Resource identifier (e.g. `users`)       |
| `description` | string | Shown in help                            |
| `operation`   | array  | List of operations (see below)           |
| `subresource` | array  | List of nested subresources              |

### `[[resource.subresource]]`

| Key           | Type   | Description                              |
|---------------|--------|------------------------------------------|
| `name`        | string | Subresource identifier (e.g. `roles`)    |
| `description` | string | Shown in help                            |
| `operation`   | array  | List of operations for the subresource   |

### `[[*.operation]]` (Applies to root, resource, and subresource)

| Key           | Type   | Description                                         |
|---------------|--------|-----------------------------------------------------|
| `verb`        | string | Command verb: `list`, `get`, `create`, `delete`, …  |
| `description` | string | Shown in help                                       |
| `args`        | array  | Positional arguments                                |
| `flags`       | array  | Named flags (`--name value`)                        |
| `confirm`     | bool   | Prompt for confirmation before executing            |

### Arg/flag types

`string` · `u32` · `u64` · `i64` · `f64` · `bool` · `enum`

For `enum`, also provide `values = ["a", "b", "c"]`.

---

## Crate roles

| Crate           | Role                                                            |
|-----------------|-----------------------------------------------------------------|
| `tku-core`    | `AppSchema`, `Render` trait, `Ctx`, `Router`, `Middleware`      |
| `tku-codegen` | `build()` entry point, `CodeGenerator`, `SchemaValidator`       |
| `tku-tui`     | `TuiApp`, `Screen` trait, `Theme`, `Sidebar`, `StatusBar`       |
| `tku-macros`  | `#[tkucli::handler]` attribute, `tkucli::register!()` macro       |
| `tkucli`     | `tkucli new`, `tkucli build`, `tkucli check`, `tkucli init`         |

---

## Roadmap

- [ ] Full `#[tkucli::handler]` proc-macro codegen
- [ ] `tkucli dev` — watch mode that re-generates on `cli.toml` changes
- [ ] TUI form screen auto-generated from `create`/`update` operations
- [ ] Shell completion generation (`tkucli completions bash`)
- [ ] Plugin system via dynamic dispatch
- [ ] Published to crates.io as `tkucli-framework`
