use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "yawn", about = "Yet Another Worktree Navigator", version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Recursively discover git projects under a directory
    List {
        /// Directory to search (defaults to current directory)
        path: Option<PathBuf>,

        /// Show human-readable names with annotations
        #[arg(short, long)]
        pretty: bool,

        /// Output as JSON array
        #[arg(long)]
        json: bool,

        /// Force stable flat output (for scripting with --pretty)
        #[arg(long)]
        porcelain: bool,
    },

    /// Map a pretty name back to an absolute path
    Resolve {
        /// The pretty name to resolve
        name: String,

        /// Directory to search (must match the path used with list)
        #[arg(short = 'P', long)]
        path: Option<PathBuf>,
    },

    /// Open a terminal in the given directory
    Open {
        /// Absolute path to the directory
        path: PathBuf,

        /// Command to open the terminal (overrides config opener)
        #[arg(short, long)]
        command: Option<String>,
    },

    /// Create a git worktree for the current project
    Create {
        /// Branch/worktree name
        name: String,

        /// Base branch/ref to create the new branch from
        #[arg(short, long)]
        source: Option<String>,

        /// Open a terminal in the worktree after creation
        #[arg(short, long)]
        open: bool,
    },

    /// Interactively pick a project and open a terminal in it
    Pick {
        /// Directory to search (defaults to current directory)
        path: Option<PathBuf>,

        /// Finder command to use (overrides config finder, e.g. fzf, "rofi -dmenu -p project -i")
        #[arg(short = 'F', long)]
        finder: Option<String>,
    },

    /// Remove a worktree for the current project
    Delete {
        /// Worktree name to remove
        name: String,
    },

    /// Output completion candidates (hidden)
    #[command(hide = true)]
    Complete {
        /// Subcommand to complete for
        subcommand: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    fn parse(args: &[&str]) -> Cli {
        Cli::parse_from(args)
    }

    #[test]
    fn test_list_no_args() {
        let cli = parse(&["yawn", "list"]);
        match cli.command {
            Command::List {
                path,
                pretty,
                json,
                porcelain,
            } => {
                assert!(path.is_none());
                assert!(!pretty);
                assert!(!json);
                assert!(!porcelain);
            }
            _ => panic!("expected List"),
        }
    }

    #[test]
    fn test_list_with_path() {
        let cli = parse(&["yawn", "list", "/home/user"]);
        match cli.command {
            Command::List {
                path,
                pretty,
                json,
                porcelain,
            } => {
                assert_eq!(path.unwrap(), PathBuf::from("/home/user"));
                assert!(!pretty);
                assert!(!json);
                assert!(!porcelain);
            }
            _ => panic!("expected List"),
        }
    }

    #[test]
    fn test_list_pretty() {
        let cli = parse(&["yawn", "list", "--pretty"]);
        match cli.command {
            Command::List { pretty, .. } => assert!(pretty),
            _ => panic!("expected List"),
        }
    }

    #[test]
    fn test_list_path_and_pretty() {
        let cli = parse(&["yawn", "list", "/tmp", "--pretty"]);
        match cli.command {
            Command::List { path, pretty, .. } => {
                assert_eq!(path.unwrap(), PathBuf::from("/tmp"));
                assert!(pretty);
            }
            _ => panic!("expected List"),
        }
    }

    #[test]
    fn test_list_json() {
        let cli = parse(&["yawn", "list", "--json"]);
        match cli.command {
            Command::List { json, pretty, .. } => {
                assert!(json);
                assert!(!pretty);
            }
            _ => panic!("expected List"),
        }
    }

    #[test]
    fn test_list_json_with_path() {
        let cli = parse(&["yawn", "list", "/tmp", "--json"]);
        match cli.command {
            Command::List { path, json, .. } => {
                assert_eq!(path.unwrap(), PathBuf::from("/tmp"));
                assert!(json);
            }
            _ => panic!("expected List"),
        }
    }

    #[test]
    fn test_list_porcelain() {
        let cli = parse(&["yawn", "list", "--pretty", "--porcelain"]);
        match cli.command {
            Command::List {
                pretty, porcelain, ..
            } => {
                assert!(pretty);
                assert!(porcelain);
            }
            _ => panic!("expected List"),
        }
    }

    #[test]
    fn test_resolve() {
        let cli = parse(&["yawn", "resolve", "my-project"]);
        match cli.command {
            Command::Resolve { name, path } => {
                assert_eq!(name, "my-project");
                assert!(path.is_none());
            }
            _ => panic!("expected Resolve"),
        }
    }

    #[test]
    fn test_resolve_with_path() {
        let cli = parse(&["yawn", "resolve", "my-project", "--path", "/home/user"]);
        match cli.command {
            Command::Resolve { name, path } => {
                assert_eq!(name, "my-project");
                assert_eq!(path.unwrap(), PathBuf::from("/home/user"));
            }
            _ => panic!("expected Resolve"),
        }
    }

    #[test]
    fn test_resolve_with_short_path() {
        let cli = parse(&["yawn", "resolve", "my-project", "-P", "/tmp"]);
        match cli.command {
            Command::Resolve { path, .. } => {
                assert_eq!(path.unwrap(), PathBuf::from("/tmp"));
            }
            _ => panic!("expected Resolve"),
        }
    }

    #[test]
    fn test_open() {
        let cli = parse(&["yawn", "open", "/home/user/project"]);
        match cli.command {
            Command::Open { path, command } => {
                assert_eq!(path, PathBuf::from("/home/user/project"));
                assert!(command.is_none());
            }
            _ => panic!("expected Open"),
        }
    }

    #[test]
    fn test_open_with_command() {
        let cli = parse(&[
            "yawn",
            "open",
            "/home/user/project",
            "-c",
            "kitty --directory {dir}",
        ]);
        match cli.command {
            Command::Open { path, command } => {
                assert_eq!(path, PathBuf::from("/home/user/project"));
                assert_eq!(command.unwrap(), "kitty --directory {dir}");
            }
            _ => panic!("expected Open"),
        }
    }

    #[test]
    fn test_open_with_long_command() {
        let cli = parse(&[
            "yawn",
            "open",
            "/tmp",
            "--command",
            "alacritty --working-directory {dir}",
        ]);
        match cli.command {
            Command::Open { path, command } => {
                assert_eq!(path, PathBuf::from("/tmp"));
                assert_eq!(command.unwrap(), "alacritty --working-directory {dir}");
            }
            _ => panic!("expected Open"),
        }
    }

    #[test]
    fn test_create_minimal() {
        let cli = parse(&["yawn", "create", "feature-x"]);
        match cli.command {
            Command::Create { name, source, open } => {
                assert_eq!(name, "feature-x");
                assert!(source.is_none());
                assert!(!open);
            }
            _ => panic!("expected Create"),
        }
    }

    #[test]
    fn test_create_with_source() {
        let cli = parse(&["yawn", "create", "feature-x", "--source", "develop"]);
        match cli.command {
            Command::Create { name, source, .. } => {
                assert_eq!(name, "feature-x");
                assert_eq!(source.unwrap(), "develop");
            }
            _ => panic!("expected Create"),
        }
    }

    #[test]
    fn test_create_with_open() {
        let cli = parse(&["yawn", "create", "feature-x", "--open"]);
        match cli.command {
            Command::Create { open, .. } => assert!(open),
            _ => panic!("expected Create"),
        }
    }

    #[test]
    fn test_create_all_flags() {
        let cli = parse(&["yawn", "create", "feature-x", "--source", "main", "--open"]);
        match cli.command {
            Command::Create { name, source, open } => {
                assert_eq!(name, "feature-x");
                assert_eq!(source.unwrap(), "main");
                assert!(open);
            }
            _ => panic!("expected Create"),
        }
    }

    #[test]
    fn test_pick_with_finder() {
        let cli = parse(&["yawn", "pick", "-F", "fzf"]);
        match cli.command {
            Command::Pick { path, finder } => {
                assert!(path.is_none());
                assert_eq!(finder.unwrap(), "fzf");
            }
            _ => panic!("expected Pick"),
        }
    }

    #[test]
    fn test_pick_no_finder() {
        let cli = parse(&["yawn", "pick"]);
        match cli.command {
            Command::Pick { path, finder } => {
                assert!(path.is_none());
                assert!(finder.is_none());
            }
            _ => panic!("expected Pick"),
        }
    }

    #[test]
    fn test_pick_with_path() {
        let cli = parse(&["yawn", "pick", "/home/user", "-F", "fzf"]);
        match cli.command {
            Command::Pick { path, finder } => {
                assert_eq!(path.unwrap(), PathBuf::from("/home/user"));
                assert_eq!(finder.unwrap(), "fzf");
            }
            _ => panic!("expected Pick"),
        }
    }

    #[test]
    fn test_pick_complex_finder() {
        let cli = parse(&["yawn", "pick", "-F", "rofi -dmenu -p project -i"]);
        match cli.command {
            Command::Pick { finder, .. } => {
                assert_eq!(finder.unwrap(), "rofi -dmenu -p project -i");
            }
            _ => panic!("expected Pick"),
        }
    }

    #[test]
    fn test_pick_long_finder_flag() {
        let cli = parse(&["yawn", "pick", "--finder", "fzf"]);
        match cli.command {
            Command::Pick { finder, .. } => assert_eq!(finder.unwrap(), "fzf"),
            _ => panic!("expected Pick"),
        }
    }

    #[test]
    fn test_delete() {
        let cli = parse(&["yawn", "delete", "feature-x"]);
        match cli.command {
            Command::Delete { name } => assert_eq!(name, "feature-x"),
            _ => panic!("expected Delete"),
        }
    }

    #[test]
    fn test_complete() {
        let cli = parse(&["yawn", "complete", "delete"]);
        match cli.command {
            Command::Complete { subcommand } => assert_eq!(subcommand, "delete"),
            _ => panic!("expected Complete"),
        }
    }

    #[test]
    fn test_complete_hidden_from_help() {
        // "complete" should not appear in help text
        let help = Cli::try_parse_from(&["yawn", "--help"])
            .unwrap_err()
            .to_string();
        assert!(
            !help.contains("complete"),
            "complete should be hidden from help"
        );
    }
}
