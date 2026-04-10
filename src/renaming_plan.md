# Renaming Project 'tkucli' to 'tkucli'

This plan outlines the steps to rename the entire project and its components.

## Proposed Name Mapping

| Old Name | New Name |
| :--- | :--- |
| `tkucli` (binary) | `tkucli` |
| `tkucli` (crate) | `tkucli` |
| `tku-core` (crate) | `tku-core` |
| `tku-tui` (crate) | `tku-tui` |
| `tku-macros` (crate) | `tku-macros` |
| `tku-codegen` (crate) | `tku-codegen` |
| `tku_core` (code) | `tku_core` |
| `tku_tui` (code) | `tku_tui` |
| `tku_macros` (code) | `tku_macros` |
| `tku_codegen` (code) | `tku_codegen` |

## Steps

### 1. Rename Workspace Directories
- [ ] `src/tku-core` -> `src/tku-core`
- [ ] `src/tku-tui` -> `src/tku-tui`
- [ ] `src/tku-macros` -> `src/tku-macros`
- [ ] `src/tku-codegen` -> `src/tku-codegen`
- [ ] `src/tkucli` -> `src/tkucli`

### 2. Update Root `Cargo.toml`
- [ ] Update `members` array.
- [ ] Update `repository` URL.

### 3. Update Crate `Cargo.toml` Files
- [ ] `tku-core/Cargo.toml`: `name = "tku-core"`
- [ ] `tku-tui/Cargo.toml`: `name = "tku-tui"`, update dependencies.
- [ ] `tku-macros/Cargo.toml`: `name = "tku-macros"`, update dependencies.
- [ ] `tku-codegen/Cargo.toml`: `name = "tku-codegen"`, update dependencies.
- [ ] `tkucli/Cargo.toml`: `name = "tkucli"`, `[[bin]].name = "tkucli"`, update dependencies.

### 4. Search and Replace in Source Code
- [ ] Replace all `tku_core` with `tku_core`
- [ ] Replace all `tku_tui` with `tku_tui`
- [ ] Replace all `tku_macros` with `tku_macros`
- [ ] Replace all `tku_codegen` with `tku_codegen`
- [ ] Replace all `Tkucli` with `Tkucli`
- [ ] Replace all `tkucli` with `tkucli` (carefully, checking case)

### 5. Update Documentation
- [ ] Update `README.md`
- [ ] Update any other `.md` files or comments.
