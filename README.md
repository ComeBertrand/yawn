# yawn — Yet Another Worktree Navigator

A CLI tool for managing git worktrees and discovering projects.

## Install

### From source

```bash
cargo install yawn
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
yawn resolve <pretty-name>      Map a pretty name back to an absolute path
yawn open <path>                Open a terminal in the given directory
yawn create <name> [--source <base>] [--open]   Create a git worktree
yawn delete <name>              Remove a worktree
```

### Discover projects

```bash
# List all git projects under ~/projects
yawn list ~/projects

# Human-readable output with worktree annotations
yawn list ~/projects --pretty
```

Pretty output example:

```
my-app
fix-branch [worktree of my-app]
dotfiles
notes (personal)
notes (work)
```

### Interactive project switcher

Combine with a fuzzy finder like `rofi` or `fzf`:

```bash
# rofi
yawn open "$(yawn resolve "$(yawn list ~ --pretty | rofi -dmenu -p project -i)")"

# fzf
yawn open "$(yawn resolve "$(yawn list ~ --pretty | fzf)")"
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

If a `.devwork` file exists in the main repo root, the files it lists (one path per line) are copied into the new worktree. This is useful for local config files like `.env` that aren't tracked by git.

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
| `open_command` | string | unset | Command template to open a terminal session. Placeholders: `{dir}` (absolute path), `{name}` (directory basename). When unset, uses `$TERMINAL` or falls back to `xterm`. |

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

Copy `completions/yawn.bash` to your bash-completion directory:

```bash
cp completions/yawn.bash ~/.local/share/bash-completion/completions/yawn
```

## License

MIT
