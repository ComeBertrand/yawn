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
            Command::List { path, pretty } => {
                assert!(path.is_none());
                assert!(!pretty);
            }
            _ => panic!("expected List"),
        }
    }

    #[test]
    fn test_list_with_path() {
        let cli = parse(&["yawn", "list", "/home/user"]);
        match cli.command {
            Command::List { path, pretty } => {
                assert_eq!(path.unwrap(), PathBuf::from("/home/user"));
                assert!(!pretty);
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
            Command::List { path, pretty } => {
                assert_eq!(path.unwrap(), PathBuf::from("/tmp"));
                assert!(pretty);
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
            Command::Open { path } => assert_eq!(path, PathBuf::from("/home/user/project")),
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
