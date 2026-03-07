# yawn — Yet Another Worktree Navigator

[![CI](https://github.com/ComeBertrand/yawn/actions/workflows/ci.yml/badge.svg)](https://github.com/ComeBertrand/yawn/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/git-yawn.svg)](https://crates.io/crates/git-yawn)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

A CLI tool for managing git worktrees and discovering projects.

## Install

### From source

```bash
cargo install git-yawn
```

### Nix flake

```nix
inputs.yawn.url = "github:ComeBertrand/yawn";

# then in your packages:
inputs.yawn.packages.${system}.default
```

### GitHub Releases

Download a binary from the [releases page](https://github.com/ComeBertrand/yawn/releases), or use the shell installer:

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/ComeBertrand/yawn/releases/latest/download/yawn-installer.sh | sh
```

## Usage

```
yawn list [path] [--pretty]     Discover git projects under a directory
yawn resolve <pretty-name> [-P <path>]  Map a pretty name back to an absolute path
yawn pick [-F <finder>] [path]  Interactively pick a project and open it
yawn open <path> [-c <command>] Open a terminal in the given directory
yawn create <name> [--source <base>] [--open]   Create a git worktree
yawn delete <name>              Remove a worktree
```

### Listing projects

Recursively discovers git projects under a directory. Takes an optional path, defaults to the current directory.

```bash
yawn list ~/projects       # discover projects under ~/projects
yawn list                  # discover projects under cwd
yawn list ~/projects -p    # human-readable with worktree annotations
```

Pretty output example:

```
my-app
fix-branch [worktree of my-app]
dotfiles
notes (personal)
notes (work)
```

If run inside a git repo (without a path), it lists the worktrees of that repo instead:

```bash
cd ~/projects/my-app
yawn list
```

```
/home/user/worktrees/my-app--fix-branch
/home/user/worktrees/my-app--feature-x
```

### Interactive project switcher

Use `yawn pick` with any fuzzy finder:

```bash
# fzf
yawn pick -F fzf ~/projects

# rofi
yawn pick -F "rofi -dmenu -p project -i" ~

# from current directory
yawn pick -F fzf
```

This discovers projects, pipes pretty names into the finder, resolves the selection, and opens a terminal — all in one command. Easy to bind in i3/sway/hyprland.

The equivalent manual pipeline still works:

```bash
yawn open "$(yawn resolve -P ~ "$(yawn list ~ --pretty | fzf)")"

# override the configured open command for a single invocation
yawn open /path/to/project -c "code {dir}"
```

### Worktrees

Worktrees are created under a configurable root directory (default: `~/worktrees`) using the naming convention `<project>--<name>`. For example, running `yawn create feature-x` from inside a repo called `my-app` creates:

```
~/worktrees/my-app--feature-x
```

When listing with `--pretty`, the `<project>--` prefix is stripped and the worktree is annotated:

```
feature-x [worktree of my-app]
```

Branch resolution when creating a worktree follows this order:

1. If `<name>` exists as a local branch, check it out.
2. If `<name>` exists as `origin/<name>`, track it.
3. If `--source <base>` is provided, create a new branch from `<base>`.
4. Otherwise, create a new branch from the default branch (`origin/HEAD`, falling back to `main` then `master`).

If a `.yawninclude` file exists in the main repo root, the files it lists are copied into the new worktree. This is useful for local config files like `.env` that aren't tracked by git. Glob patterns are supported.

```
# .yawninclude
.env
.env.local
config/*.toml
data_*.csv
```

```bash
# Create a worktree (new branch from default branch)
yawn create feature-x

# Create from a specific base branch
yawn create feature-x --source develop

# Create and immediately open a terminal in it
yawn create feature-x --open

# Delete a worktree
yawn delete feature-x
```

## Configuration

`~/.config/yawn/config.toml` — all fields are optional.

### `[discovery]`

| Key | Type | Default | Description |
|---|---|---|---|
| `max_depth` | integer | `5` | Maximum recursion depth when searching for git projects. |
| `ignore` | list of strings | `[".*", "node_modules"]` | Glob patterns matched against directory names. Matching directories are skipped during discovery. Hidden directories (except `.git` itself) are ignored by default. |

### `[session]`

| Key | Type | Default | Description |
|---|---|---|---|
| `open_command` | string | unset | Command template to open a terminal session. Placeholders: `{dir}` (absolute path), `{name}` (directory basename). When unset, uses `$TERMINAL`, or falls back to `Terminal.app` on macOS and `xterm` on Linux. |

### `[worktree]`

| Key | Type | Default | Description |
|---|---|---|---|
| `root` | string | `~/worktrees` | Directory where worktrees are created. Supports `~` expansion. |

### Example

```toml
[discovery]
max_depth = 3
ignore = [".*", "node_modules", "target", "vendor"]

[session]
open_command = "kitty --directory {dir} --title 'dev: {name}'"

[worktree]
root = "~/worktrees"
```

## Shell Completion

### Bash

```bash
cp completions/yawn.bash ~/.local/share/bash-completion/completions/yawn
```

### Zsh

```bash
cp completions/yawn.zsh ~/.local/share/zsh/site-functions/_yawn
```

Or place it anywhere in your `$fpath`.

### Fish

```bash
cp completions/yawn.fish ~/.config/fish/completions/yawn.fish
```

## Man Page

A man page is generated at build time. After building from source:

```bash
man target/*/build/git-yawn-*/out/man/yawn.1
```

## License

MIT
