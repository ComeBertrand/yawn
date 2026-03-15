# yawn - Yet Another Worktree Navigator

[![CI](https://github.com/ComeBertrand/yawn/actions/workflows/ci.yml/badge.svg)](https://github.com/ComeBertrand/yawn/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/git-yawn.svg)](https://crates.io/crates/git-yawn)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

A fast project switcher and worktree manager for git. Fuzzy-find any project on your machine and open it: terminal, IDE, whatever you want. Also makes git worktrees painless.

![yawn pick with rofi opening VS Code](demo.gif)

## Features

- **Fuzzy-find and open** projects with any finder (fzf, rofi, dmenu...)
- **Create worktrees** with automatic branch resolution and one-step setup
- **Auto-initialize** worktrees: copy `.env`, run `npm install`, etc.
- **Discover** all git repos recursively, displayed as a tree with worktrees grouped under their parent
- **Bind to a WM hotkey**, works great with i3/sway/hyprland
- JSON/porcelain output for scripting

## Quick start

```bash
cargo install git-yawn
yawn pick -F fzf ~/projects
```

## Project picker

`yawn pick` discovers projects, pipes them into a fuzzy finder, resolves the selection, and opens it. One command.

```bash
yawn pick -F fzf ~/projects
yawn pick -F "rofi -dmenu -p project -i" ~
```

Bind it to a hotkey in your window manager:

```bash
# sway / i3
bindsym $mod+p exec yawn pick -F "rofi -dmenu -p project -i" ~/projects
```

You can also set a default finder in your config and just run `yawn pick`.

Under the hood, `yawn pick` is equivalent to:

```bash
yawn open "$(yawn resolve -P ~ "$(yawn list ~ --porcelain | fzf)")"
```

## Name resolution

`yawn resolve` maps a pretty display name back to an absolute path. `yawn prettify` does the inverse — maps an absolute path to its pretty name:

```bash
yawn resolve "feature @myapp" -P ~/projects    # → /home/user/worktrees/myapp--feature
yawn prettify /home/user/worktrees/myapp--feature -P ~/projects   # → feature @myapp
```

## Worktree management

```bash
# The manual way:
git worktree add ~/worktrees/my-app--feature-x -b feature-x origin/main
cd ~/worktrees/my-app--feature-x
cp ../my-app/.env .
npm install

# With yawn:
yawn create feature-x --init --open
```

Worktrees are created under a configurable root directory (default: `~/worktrees`) using the convention `<project>--<name>`.

```bash
yawn create feature-x                           # new branch from default branch
yawn create feature-x --source develop          # branch from a specific base
yawn create feature-x --init --open             # setup + open

yawn delete feature-x                           # remove (prompts for branch deletion)
yawn delete feature-x --branch --force          # remove worktree + branch, no prompts
```

Branch resolution: checks out existing local branches, tracks remote branches, or creates a new branch from `--source` or the default branch.

### Per-project setup with `.yawn.toml`

Place a `.yawn.toml` at the repo root to configure what happens during `yawn init` or `yawn create --init`:

```toml
[init]
include = [".env", ".env.local", "config/*.toml"]
commands = ["npm install", "cargo build"]
```

- **`include`**: files, directories, or glob patterns to copy from the main repo into worktrees. Directories are copied recursively.
- **`commands`**: shell commands to run sequentially in the target directory. Stops on first failure.

## Project discovery

`yawn list` recursively finds git projects. In a terminal, they're shown as a colored tree:

```
my-app
├─ fix-branch
└─ feature-x
dotfiles
notes (personal)
notes (work)
```

When piped, output falls back to flat names compatible with fzf and other tools. Use `--raw` for absolute paths or `--json` for structured output.

```bash
yawn list ~/projects                # tree in terminal, flat when piped
yawn list --porcelain               # flat pretty names (stable for scripts)
yawn list --raw                     # absolute paths
yawn list --json                    # structured JSON
```

## Configuration

Global config lives at `~/.config/yawn/config.toml`. All fields are optional.

```toml
[discovery]
max_depth = 3
ignore = [".*", "node_modules", "target", "vendor"]

[session]
opener = "code {dir}"
finder = "fzf"

[worktree]
root = "~/worktrees"
auto_init = false
```

- **`discovery.max_depth`**: recursion depth when searching for projects (default: `5`)
- **`discovery.ignore`**: directory name patterns to skip (default: `[".*", "node_modules"]`)
- **`session.opener`**: command template to open a project. `{dir}` and `{name}` are shell-quoted automatically. Examples: `code {dir}`, `kitty --directory {dir}`. Falls back to `$TERMINAL`, then `Terminal.app` (macOS) or `xterm` (Linux).
- **`session.finder`**: default finder for `yawn pick` (overridden by `-F`)
- **`worktree.root`**: where worktrees are created (default: `~/worktrees`)
- **`worktree.auto_init`**: always run init after creating a worktree (default: `false`)

## Install

### From source

```bash
cargo install git-yawn
```

### GitHub Releases

Download a binary from the [releases page](https://github.com/ComeBertrand/yawn/releases), or use the shell installer:

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/ComeBertrand/yawn/releases/latest/download/yawn-installer.sh | sh
```

### Nix flake

```nix
inputs.yawn.url = "github:ComeBertrand/yawn";

# then in your packages:
inputs.yawn.packages.${system}.default
```

## Shell completion & man page

Shell completions are included:

```bash
# Bash
cp completions/yawn.bash ~/.local/share/bash-completion/completions/yawn

# Zsh (or place it anywhere in your $fpath)
cp completions/yawn.zsh ~/.local/share/zsh/site-functions/_yawn

# Fish
cp completions/yawn.fish ~/.config/fish/completions/yawn.fish
```

A man page is generated at build time:

```bash
man target/*/build/git-yawn-*/out/man/yawn.1
```

## License

MIT
