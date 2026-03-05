use anyhow::{bail, Result};
use std::path::Path;
use std::process::Command;

/// Build the command string by substituting `{dir}` and `{name}` placeholders.
pub fn build_command(template: &str, dir: &str, name: &str) -> String {
    template.replace("{dir}", dir).replace("{name}", name)
}

/// Open a terminal session in the given directory.
pub fn open(dir: &Path, open_command: Option<&str>) -> Result<()> {
    let dir_str = dir.to_string_lossy();
    let name = dir
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    if let Some(template) = open_command {
        let cmd = build_command(template, &dir_str, &name);
        let status = Command::new("sh").arg("-c").arg(&cmd).status()?;
        if !status.success() {
            bail!("open command failed: {}", cmd);
        }
    } else {
        let terminal = std::env::var("TERMINAL").unwrap_or_else(|_| "xterm".to_string());
        Command::new(&terminal).current_dir(dir).spawn()?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_command_both_placeholders() {
        let result = build_command(
            "kitty --directory {dir} --title 'dev: {name}'",
            "/home/user/project",
            "project",
        );
        assert_eq!(
            result,
            "kitty --directory /home/user/project --title 'dev: project'"
        );
    }

    #[test]
    fn test_build_command_dir_only() {
        let result = build_command("code {dir}", "/tmp/myproject", "myproject");
        assert_eq!(result, "code /tmp/myproject");
    }

    #[test]
    fn test_build_command_name_only() {
        let result = build_command("tmux new -s {name}", "/tmp/foo", "foo");
        assert_eq!(result, "tmux new -s foo");
    }

    #[test]
    fn test_build_command_no_placeholders() {
        let result = build_command("echo hello", "/tmp/foo", "foo");
        assert_eq!(result, "echo hello");
    }

    #[test]
    fn test_build_command_multiple_occurrences() {
        let result = build_command("{dir} {dir} {name}", "/a", "b");
        assert_eq!(result, "/a /a b");
    }
}
