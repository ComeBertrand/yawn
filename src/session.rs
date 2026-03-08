use anyhow::{bail, Result};
use std::path::Path;
use std::process::Command;

/// Shell-quote a string for safe interpolation into `sh -c` commands.
///
/// Strings containing only safe characters are returned as-is.
/// Everything else is wrapped in single quotes with internal `'` escaped.
fn shell_quote(s: &str) -> String {
    if s.is_empty() {
        return "''".to_string();
    }
    if s.bytes().all(|b| {
        matches!(b, b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9'
            | b'-' | b'_' | b'/' | b'.' | b':' | b'@' | b'=' | b'+' | b',')
    }) {
        return s.to_string();
    }
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// Build the command string by substituting `{dir}` and `{name}` placeholders.
///
/// Values are shell-quoted to prevent injection and handle paths with spaces.
pub fn build_command(template: &str, dir: &str, name: &str) -> String {
    template
        .replace("{dir}", &shell_quote(dir))
        .replace("{name}", &shell_quote(name))
}

/// Open a terminal session in the given directory.
pub fn open(dir: &Path, opener: Option<&str>) -> Result<()> {
    let dir_str = dir.to_string_lossy();
    let name = dir
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    if let Some(template) = opener {
        let cmd = build_command(template, &dir_str, &name);
        let status = Command::new("sh").arg("-c").arg(&cmd).status()?;
        if !status.success() {
            bail!("open command failed: {}", cmd);
        }
    } else if let Ok(terminal) = std::env::var("TERMINAL") {
        Command::new(&terminal).current_dir(dir).spawn()?;
    } else {
        default_open(dir)?;
    }

    Ok(())
}

/// Platform-specific fallback to open a terminal in a directory.
#[cfg(target_os = "macos")]
fn default_open(dir: &Path) -> Result<()> {
    let status = Command::new("open")
        .arg("-a")
        .arg("Terminal")
        .arg(dir)
        .status()?;
    if !status.success() {
        bail!("failed to open Terminal.app");
    }
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn default_open(dir: &Path) -> Result<()> {
    Command::new("xterm").current_dir(dir).spawn()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- shell_quote tests ---

    #[test]
    fn test_shell_quote_safe_string() {
        assert_eq!(shell_quote("/home/user/project"), "/home/user/project");
        assert_eq!(shell_quote("my-app"), "my-app");
        assert_eq!(shell_quote("foo.bar"), "foo.bar");
    }

    #[test]
    fn test_shell_quote_empty() {
        assert_eq!(shell_quote(""), "''");
    }

    #[test]
    fn test_shell_quote_spaces() {
        assert_eq!(
            shell_quote("/home/user/my project"),
            "'/home/user/my project'"
        );
    }

    #[test]
    fn test_shell_quote_single_quotes() {
        assert_eq!(shell_quote("it's"), "'it'\\''s'");
    }

    #[test]
    fn test_shell_quote_metacharacters() {
        assert_eq!(shell_quote("foo; rm -rf /"), "'foo; rm -rf /'");
        assert_eq!(shell_quote("$(whoami)"), "'$(whoami)'");
        assert_eq!(shell_quote("a`cmd`b"), "'a`cmd`b'");
    }

    // --- build_command tests ---

    #[test]
    fn test_build_command_both_placeholders() {
        let result = build_command(
            "kitty --directory {dir} --title {name}",
            "/home/user/project",
            "project",
        );
        assert_eq!(
            result,
            "kitty --directory /home/user/project --title project"
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

    #[test]
    fn test_build_command_dir_with_spaces() {
        let result = build_command("code {dir}", "/home/user/my project", "my project");
        assert_eq!(result, "code '/home/user/my project'");
    }

    #[test]
    fn test_build_command_injection_attempt() {
        let result = build_command("code {dir}", "/tmp/foo; rm -rf /", "bar");
        assert_eq!(result, "code '/tmp/foo; rm -rf /'");
    }
}
