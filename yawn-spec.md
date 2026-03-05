# yawn — Yet Another Worktree Navigator

A CLI tool for managing git worktrees and discovering projects.

## CLI Interface

```
yawn list [path] [--pretty]
yawn resolve <pretty-name>
yawn open <path>
yawn create <name> [--source <base>] [--open]
yawn delete <name>
yawn complete <subcommand>
```

### `yawn list [path] [--pretty]`

Recursively discover git projects under a directory.

- `path` defaults to current working directory.
- A "git project" is any directory containing a `.git` entry (directory for regular repos, file for worktrees).
- Discovery prunes directories matching configurable ignore patterns (default: hidden dirs except `.git`, `node_modules`).
- Max search depth is configurable (default: 5).

**Default output:** absolute paths, one per line, suitable for piping.

```
/home/user/Workspace/cargostack-backend
/home/user/Workspace/worktrees/cargostack-backend--fix-branch
/home/user/dotfiles
/home/user/Documents/projects/mnemo
```

**`--pretty` output:** human-readable names with worktree annotations and collision disambiguation.

Rules:
1. Base display name is the directory basename.
2. If the project is a git worktree (`.git` is a file, not a directory), strip the `<project>--` prefix from the name and annotate with `[worktree of <project>]`.
3. If two or more projects share the same display name, append `(<disambiguating path>)` using the shortest unique parent path suffix.

```
cargostack-backend
fix-branch [worktree of cargostack-backend]
dotfiles
mnemo (Workspace)
mnemo (Documents/projects)
```

### `yawn resolve <pretty-name>`

Map a pretty name back to an absolute path.

- Runs the same discovery logic as `list`.
- Builds the same pretty names using the same rules.
- Returns the absolute path for the matching entry.
- Errors if no match or ambiguous match.

This enables the pattern:

```bash
yawn open "$(yawn resolve "$(yawn list ~ --pretty | rofi -dmenu -p project -i)")"
```

### `yawn open <path>`

Open a terminal in the given directory.

- `path` must be an absolute path to an existing directory.
- If `open_command` is set in config, runs it with `{dir}` and `{name}` (basename of path) substituted.
- If `open_command` is not set, opens the user's default terminal in the directory (respects `$TERMINAL`, falls back to a sensible default).

### `yawn create <name> [--source <base>] [--open]`

Create a git worktree for the current project.

**Precondition:** must be inside a git repository (regular repo or worktree).

**Steps:**

1. Resolve the main repository root (via `git rev-parse --git-common-dir`).
2. Derive project name from the main repo basename.
3. Target directory: `<worktree_root>/<project>--<name>`.
4. If the target directory already exists, report and skip creation.
5. Run `git fetch --quiet`.
6. Branch resolution (in order):
   a. If `<name>` exists as a local branch, check it out.
   b. If `<name>` exists as `origin/<name>`, track it.
   c. If `--source <base>` is provided, create new branch from `<base>`.
   d. Otherwise, create new branch from the default branch (detected via `origin/HEAD`, falling back to `main` then `master`).
7. If a `.devwork` file exists in the main repo root, copy listed files to the worktree (one path per line, comments and blank lines ignored). If `.devwork` is absent, nothing is copied.

**Flags:**
- `--source <base>`: base branch/ref to create the new branch from.
- `--open`: after creation, run `yawn open <worktree-path>`.

### `yawn delete <name>`

Remove a worktree for the current project.

**Precondition:** must be inside a git repository.

**Steps:**

1. Resolve project name (same as `create`).
2. Target directory: `<worktree_root>/<project>--<name>`.
3. Run `git worktree remove <target>`. If the worktree is already gone from git but the directory remains, remove the directory.
4. If the branch `<name>` still exists locally, print a note (do not auto-delete).

### `yawn complete <subcommand>`

Output completion candidates for dynamic arguments. Hidden from help.

- `yawn complete delete` — list worktree short names for the current project (strips `<project>--` prefix).
- `yawn complete open` — equivalent to `yawn list` from the current directory.

Used by the shell completion script.

## Configuration

File: `~/.config/yawn/config.toml`

All fields are optional with sensible defaults.

```toml
[discovery]
max_depth = 5                # Max recursion depth for list
# Directories to skip during discovery. Glob patterns matched against directory names.
# Default: hidden directories (.*) and node_modules
ignore = [".*", "node_modules"]

[session]
# Command template to open a session in a directory.
# Available placeholders: {dir}, {name}
# Default: opens $TERMINAL in {dir}, or falls back to a sensible default.
# open_command = "kitty --directory {dir} --title 'dev: {name}'"

[worktree]
# Where worktrees are created.
# Default: ~/worktrees
root = "~/worktrees"
```

## Bash Completion

Installed alongside the binary (e.g. via package manager, home-manager, or manual copy to bash-completion directory).

```bash
_yawn() {
    local cur prev
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev="${COMP_WORDS[COMP_CWORD-1]}"
    case "$prev" in
        delete)
            COMPREPLY=($(compgen -W "$(yawn complete delete 2>/dev/null)" -- "$cur"))
            ;;
        open)
            COMPREPLY=($(compgen -W "$(yawn complete open 2>/dev/null)" -- "$cur"))
            ;;
        *)
            COMPREPLY=($(compgen -W "list create delete open resolve" -- "$cur"))
            ;;
    esac
}
complete -F _yawn yawn
```

## Project Architecture

```
yawn/
├── Cargo.toml
├── Cargo.lock
├── flake.nix                  # Nix build
├── flake.lock
├── LICENSE                    # MIT
├── README.md
├── dist-workspace.toml        # cargo-dist config
├── completions/
│   └── yawn.bash             # Bash completion script
├── src/
│   ├── main.rs                # Entry point, clap CLI definition
│   ├── cli.rs                 # Clap structs and argument definitions
│   ├── config.rs              # Load and parse ~/.config/yawn/config.toml
│   ├── discovery.rs           # Recursive git project discovery (find .git)
│   ├── pretty.rs              # Pretty name formatting + resolve logic
│   ├── worktree.rs            # Create and delete worktree operations
│   ├── session.rs             # Open terminal / run open_command
│   └── git.rs                 # Git helper functions (fetch, branch resolution, etc.)
└── .github/
    └── workflows/
        └── release.yml        # cargo-dist: build + publish on tag
```

### Module responsibilities

- **cli.rs** — Clap derive structs. Defines subcommands, flags, arguments.
- **config.rs** — Reads `~/.config/yawn/config.toml`, provides defaults for missing fields, expands `~`.
- **discovery.rs** — Walks directories looking for `.git` entries. Skips directories matching configured ignore patterns (default: `.*` except `.git`, `node_modules`). Respects `max_depth`. Returns list of absolute paths.
- **pretty.rs** — Takes a list of discovered paths. Detects worktrees (`.git` is a file). Builds pretty display names with collision disambiguation. Also implements `resolve`: given a pretty name and a list of paths, returns the matching absolute path.
- **worktree.rs** — Implements `create` and `delete` logic. Calls git commands, handles `.devwork` file copying.
- **session.rs** — Runs configured `open_command` with placeholder substitution, or falls back to opening `$TERMINAL` in the directory.
- **git.rs** — Thin wrappers around git commands: `fetch`, `show-ref`, `worktree add/remove/list`, `rev-parse`, `symbolic-ref`. Returns typed results.

## Distribution

### crates.io

Standard `cargo publish`. Users install with:

```bash
cargo install yawn
```

### GitHub Releases (cargo-dist)

Automated via `cargo-dist`. On pushing a version tag (e.g. `v0.1.0`):

1. GitHub Actions workflow (`.github/workflows/release.yml`) triggers.
2. Builds binaries for: `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin`, `aarch64-apple-darwin`.
3. Creates a GitHub Release with attached tarballs.
4. Generates a shell installer script.

Setup: `cargo dist init` scaffolds the config and CI workflow.

### Nix Flake

`flake.nix` in the repo builds the package:

```nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.05";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let pkgs = nixpkgs.legacyPackages.${system}; in {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "yawn";
          version = "0.1.0";
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;
        };
      }
    );
}
```

Users consume as a flake input:

```nix
# In another flake.nix
inputs.yawn.url = "github:<user>/yawn";

# In configuration.nix
environment.systemPackages = [ inputs.yawn.packages.${system}.default ];
```

### Homebrew (via cargo-dist)

cargo-dist generates a Homebrew tap automatically. Creates a separate `homebrew-tap` repo with the formula.

```bash
brew install <user>/tap/yawn
```
