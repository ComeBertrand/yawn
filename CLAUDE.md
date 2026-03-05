# CLAUDE.md

## Project

Rust CLI tool for managing git worktrees and discovering projects. Crate name is `git-yawn`, binary name is `yawn`.

## Toolchain

Rust is not in PATH. Use `nix-shell` to access it:

```bash
nix-shell -p cargo rustc git clippy rustfmt --run "<command>"
```

## Before committing

Always run these in order:

```bash
nix-shell -p cargo rustc git clippy rustfmt --run "cargo fmt"
nix-shell -p cargo rustc git clippy rustfmt --run "cargo clippy -- -D warnings"
nix-shell -p cargo rustc git clippy rustfmt --run "cargo test"
```

## Releasing a new version

1. Bump `version` in **both** `Cargo.toml` and `flake.nix`.
2. Run `cargo check` to regenerate `Cargo.lock`.
3. Commit all three files (`Cargo.toml`, `Cargo.lock`, `flake.nix`).
4. Tag with `v<version>` and push both the commit and the tag.

Forgetting to commit the updated `Cargo.lock` will cause the crates.io publish to fail.

## GitHub CLI

`gh` is installed and authenticated. Use it directly (no nix-shell needed).
