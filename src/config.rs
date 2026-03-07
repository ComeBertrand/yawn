use anyhow::Result;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Default)]
pub struct ConfigFile {
    pub discovery: Option<DiscoveryConfig>,
    pub session: Option<SessionConfig>,
    pub worktree: Option<WorktreeConfig>,
}

#[derive(Debug, Deserialize)]
pub struct DiscoveryConfig {
    pub max_depth: Option<usize>,
    pub ignore: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct SessionConfig {
    pub opener: Option<String>,
    pub finder: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct WorktreeConfig {
    pub root: Option<String>,
}

#[derive(Debug)]
pub struct Config {
    pub max_depth: usize,
    pub ignore: Vec<String>,
    pub opener: Option<String>,
    pub finder: Option<String>,
    pub worktree_root: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            max_depth: 5,
            ignore: vec![".*".to_string(), "node_modules".to_string()],
            opener: None,
            finder: None,
            worktree_root: dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("/"))
                .join("worktrees"),
        }
    }
}

/// Expand a leading `~` to the user's home directory.
pub fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("/"))
            .join(rest)
    } else if path == "~" {
        dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"))
    } else {
        PathBuf::from(path)
    }
}

/// Parse a TOML string into a resolved Config.
pub fn parse_config(toml_str: &str) -> Result<Config> {
    let file: ConfigFile = toml::from_str(toml_str)?;
    let defaults = Config::default();

    let max_depth = file
        .discovery
        .as_ref()
        .and_then(|d| d.max_depth)
        .unwrap_or(defaults.max_depth);

    let ignore = file
        .discovery
        .as_ref()
        .and_then(|d| d.ignore.clone())
        .unwrap_or(defaults.ignore);

    let opener = file.session.as_ref().and_then(|s| s.opener.clone());
    let finder = file.session.as_ref().and_then(|s| s.finder.clone());

    let worktree_root = file
        .worktree
        .as_ref()
        .and_then(|w| w.root.as_ref())
        .map(|r| expand_tilde(r))
        .unwrap_or(defaults.worktree_root);

    Ok(Config {
        max_depth,
        ignore,
        opener,
        finder,
        worktree_root,
    })
}

/// Load config from the standard path, or return defaults if the file doesn't exist.
pub fn load_config() -> Result<Config> {
    let config_path = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("/"))
        .join("yawn")
        .join("config.toml");

    if config_path.exists() {
        let content = std::fs::read_to_string(&config_path)?;
        parse_config(&content)
    } else {
        Ok(Config::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_defaults() {
        let config = Config::default();
        assert_eq!(config.max_depth, 5);
        assert_eq!(config.ignore, vec![".*", "node_modules"]);
        assert!(config.opener.is_none());
        assert!(config.worktree_root.ends_with("worktrees"));
    }

    #[test]
    fn test_parse_empty_toml() {
        let config = parse_config("").unwrap();
        assert_eq!(config.max_depth, 5);
        assert_eq!(config.ignore, vec![".*", "node_modules"]);
        assert!(config.opener.is_none());
    }

    #[test]
    fn test_parse_partial_discovery() {
        let toml = r#"
[discovery]
max_depth = 3
"#;
        let config = parse_config(toml).unwrap();
        assert_eq!(config.max_depth, 3);
        // ignore should still be default since it wasn't specified
        assert_eq!(config.ignore, vec![".*", "node_modules"]);
    }

    #[test]
    fn test_parse_custom_ignore() {
        let toml = r#"
[discovery]
ignore = [".*", "node_modules", "target", "vendor"]
"#;
        let config = parse_config(toml).unwrap();
        assert_eq!(
            config.ignore,
            vec![".*", "node_modules", "target", "vendor"]
        );
    }

    #[test]
    fn test_parse_session() {
        let toml = r#"
[session]
opener = "kitty --directory {dir} --title 'dev: {name}'"
"#;
        let config = parse_config(toml).unwrap();
        assert_eq!(
            config.opener.unwrap(),
            "kitty --directory {dir} --title 'dev: {name}'"
        );
    }

    #[test]
    fn test_parse_session_finder() {
        let toml = r#"
[session]
finder = "fzf"
"#;
        let config = parse_config(toml).unwrap();
        assert_eq!(config.finder.unwrap(), "fzf");
    }

    #[test]
    fn test_parse_session_both() {
        let toml = r#"
[session]
opener = "kitty --directory {dir}"
finder = "rofi -dmenu -p project -i"
"#;
        let config = parse_config(toml).unwrap();
        assert_eq!(config.opener.unwrap(), "kitty --directory {dir}");
        assert_eq!(config.finder.unwrap(), "rofi -dmenu -p project -i");
    }

    #[test]
    fn test_parse_worktree_root_tilde() {
        let toml = r#"
[worktree]
root = "~/my-worktrees"
"#;
        let config = parse_config(toml).unwrap();
        let home = dirs::home_dir().unwrap();
        assert_eq!(config.worktree_root, home.join("my-worktrees"));
    }

    #[test]
    fn test_parse_worktree_root_absolute() {
        let toml = r#"
[worktree]
root = "/tmp/worktrees"
"#;
        let config = parse_config(toml).unwrap();
        assert_eq!(config.worktree_root, PathBuf::from("/tmp/worktrees"));
    }

    #[test]
    fn test_parse_full_config() {
        let toml = r#"
[discovery]
max_depth = 10
ignore = [".*"]

[session]
opener = "alacritty --working-directory {dir}"

[worktree]
root = "/opt/worktrees"
"#;
        let config = parse_config(toml).unwrap();
        assert_eq!(config.max_depth, 10);
        assert_eq!(config.ignore, vec![".*"]);
        assert_eq!(
            config.opener.unwrap(),
            "alacritty --working-directory {dir}"
        );
        assert_eq!(config.worktree_root, PathBuf::from("/opt/worktrees"));
    }

    #[test]
    fn test_parse_invalid_toml() {
        let result = parse_config("this is not valid toml {{{}}}");
        assert!(result.is_err());
    }

    #[test]
    fn test_expand_tilde_with_path() {
        let home = dirs::home_dir().unwrap();
        assert_eq!(expand_tilde("~/foo/bar"), home.join("foo/bar"));
    }

    #[test]
    fn test_expand_tilde_bare() {
        let home = dirs::home_dir().unwrap();
        assert_eq!(expand_tilde("~"), home);
    }

    #[test]
    fn test_expand_tilde_absolute_passthrough() {
        assert_eq!(expand_tilde("/tmp/foo"), PathBuf::from("/tmp/foo"));
    }

    #[test]
    fn test_expand_tilde_relative_passthrough() {
        assert_eq!(expand_tilde("foo/bar"), PathBuf::from("foo/bar"));
    }
}
