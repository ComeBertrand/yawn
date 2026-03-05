use anyhow::{bail, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::git;

/// Parse a `.devwork` file and return the list of file paths to copy.
/// Blank lines and lines starting with `#` are ignored.
pub fn parse_devwork(content: &str) -> Vec<String> {
    content
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| line.to_string())
        .collect()
}

/// Copy files listed in `.devwork` from the main repo to the worktree.
pub fn copy_devwork_files(main_repo: &Path, worktree: &Path) -> Result<()> {
    let devwork_path = main_repo.join(".devwork");
    if !devwork_path.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(&devwork_path)?;
    let files = parse_devwork(&content);

    for file in &files {
        let src = main_repo.join(file);
        let dst = worktree.join(file);
        if src.exists() {
            if let Some(parent) = dst.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&src, &dst)
                .with_context(|| format!("failed to copy {} to worktree", file))?;
        }
    }

    Ok(())
}

/// Create a worktree for the current project.
///
/// Returns the path to the created worktree.
pub fn create(name: &str, source: Option<&str>, config: &Config, cwd: &Path) -> Result<PathBuf> {
    let main_root = git::repo_root(cwd)?;
    let project_name = main_root
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let target = config
        .worktree_root
        .join(format!("{}--{}", project_name, name));

    if target.exists() {
        bail!(
            "worktree directory already exists: {}",
            target.display()
        );
    }

    git::fetch(cwd)?;

    // Branch resolution
    if git::local_branch_exists(cwd, name)? {
        // a. Local branch exists — check it out
        git::worktree_add(cwd, &target, name)?;
    } else if git::remote_branch_exists(cwd, name)? {
        // b. Remote branch exists — track it
        git::worktree_add(cwd, &target, name)?;
    } else if let Some(base) = source {
        // c. --source provided — create new branch from base
        git::worktree_add_new_branch(cwd, &target, name, base)?;
    } else {
        // d. Create new branch from default branch
        let default = git::default_branch(cwd)?;
        git::worktree_add_new_branch(cwd, &target, name, &default)?;
    }

    copy_devwork_files(&main_root, &target)?;

    Ok(target)
}

/// Delete a worktree for the current project.
pub fn delete(name: &str, config: &Config, cwd: &Path) -> Result<()> {
    let main_root = git::repo_root(cwd)?;
    let project_name = main_root
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let target = config
        .worktree_root
        .join(format!("{}--{}", project_name, name));

    // Try git worktree remove first
    if let Err(_) = git::worktree_remove(cwd, &target) {
        // If git worktree remove failed but directory exists, remove it manually
        if target.exists() {
            fs::remove_dir_all(&target)
                .with_context(|| format!("failed to remove directory {}", target.display()))?;
        }
    }

    // Check if the branch still exists locally
    if git::local_branch_exists(cwd, name)? {
        eprintln!(
            "note: local branch '{}' still exists. Remove it manually with: git branch -d {}",
            name, name
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use tempfile::TempDir;

    fn git_cmd(dir: &Path, args: &[&str]) -> String {
        let output = Command::new("git")
            .args(args)
            .current_dir(dir)
            .output()
            .unwrap();
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }

    fn init_repo(path: &Path) {
        fs::create_dir_all(path).unwrap();
        git_cmd(path, &["init"]);
        git_cmd(path, &["config", "user.email", "test@test.com"]);
        git_cmd(path, &["config", "user.name", "Test"]);
        fs::write(path.join("README.md"), "# test").unwrap();
        git_cmd(path, &["add", "."]);
        git_cmd(path, &["commit", "-m", "init"]);
        // Ensure we're on main
        let _ = git_cmd(path, &["branch", "-M", "main"]);
    }

    fn test_config(worktree_root: PathBuf) -> Config {
        Config {
            max_depth: 5,
            ignore: vec![".*".to_string(), "node_modules".to_string()],
            open_command: None,
            worktree_root,
        }
    }

    // --- parse_devwork tests ---

    #[test]
    fn test_parse_devwork_basic() {
        let content = ".env\n.env.local\nconfig/dev.toml\n";
        let files = parse_devwork(content);
        assert_eq!(files, vec![".env", ".env.local", "config/dev.toml"]);
    }

    #[test]
    fn test_parse_devwork_comments_and_blanks() {
        let content = "# Local environment files\n.env\n\n# Config\nconfig/dev.toml\n\n";
        let files = parse_devwork(content);
        assert_eq!(files, vec![".env", "config/dev.toml"]);
    }

    #[test]
    fn test_parse_devwork_empty() {
        let files = parse_devwork("");
        assert!(files.is_empty());
    }

    #[test]
    fn test_parse_devwork_only_comments() {
        let files = parse_devwork("# comment\n# another\n");
        assert!(files.is_empty());
    }

    #[test]
    fn test_parse_devwork_whitespace_trimming() {
        let content = "  .env  \n  config/dev.toml  \n";
        let files = parse_devwork(content);
        assert_eq!(files, vec![".env", "config/dev.toml"]);
    }

    // --- copy_devwork_files tests ---

    #[test]
    fn test_copy_devwork_files_no_devwork_file() {
        let tmp = TempDir::new().unwrap();
        let main_repo = tmp.path().join("main");
        let worktree = tmp.path().join("wt");
        fs::create_dir_all(&main_repo).unwrap();
        fs::create_dir_all(&worktree).unwrap();

        // Should succeed silently when .devwork doesn't exist
        copy_devwork_files(&main_repo, &worktree).unwrap();
    }

    #[test]
    fn test_copy_devwork_files_copies_listed_files() {
        let tmp = TempDir::new().unwrap();
        let main_repo = tmp.path().join("main");
        let worktree = tmp.path().join("wt");
        fs::create_dir_all(&main_repo).unwrap();
        fs::create_dir_all(&worktree).unwrap();

        // Create .devwork and the files it references
        fs::write(main_repo.join(".devwork"), ".env\nconfig/local.toml\n").unwrap();
        fs::write(main_repo.join(".env"), "SECRET=123").unwrap();
        fs::create_dir_all(main_repo.join("config")).unwrap();
        fs::write(main_repo.join("config/local.toml"), "[db]\nhost=localhost").unwrap();

        copy_devwork_files(&main_repo, &worktree).unwrap();

        assert_eq!(fs::read_to_string(worktree.join(".env")).unwrap(), "SECRET=123");
        assert_eq!(
            fs::read_to_string(worktree.join("config/local.toml")).unwrap(),
            "[db]\nhost=localhost"
        );
    }

    #[test]
    fn test_copy_devwork_files_skips_missing_source() {
        let tmp = TempDir::new().unwrap();
        let main_repo = tmp.path().join("main");
        let worktree = tmp.path().join("wt");
        fs::create_dir_all(&main_repo).unwrap();
        fs::create_dir_all(&worktree).unwrap();

        fs::write(main_repo.join(".devwork"), ".env\nmissing-file\n").unwrap();
        fs::write(main_repo.join(".env"), "SECRET=123").unwrap();

        // Should not error on missing source file
        copy_devwork_files(&main_repo, &worktree).unwrap();
        assert!(worktree.join(".env").exists());
        assert!(!worktree.join("missing-file").exists());
    }

    // --- create tests ---

    #[test]
    fn test_create_new_branch_from_default() {
        let tmp = TempDir::new().unwrap();
        let repo = tmp.path().join("myproject");
        init_repo(&repo);

        let wt_root = tmp.path().join("worktrees");
        fs::create_dir_all(&wt_root).unwrap();
        let config = test_config(wt_root.clone());

        let result = create("feature-x", None, &config, &repo).unwrap();
        assert_eq!(result, wt_root.join("myproject--feature-x"));
        assert!(result.exists());

        // The worktree should have the README from the repo
        assert!(result.join("README.md").exists());
    }

    #[test]
    fn test_create_with_source() {
        let tmp = TempDir::new().unwrap();
        let repo = tmp.path().join("myproject");
        init_repo(&repo);

        let wt_root = tmp.path().join("worktrees");
        fs::create_dir_all(&wt_root).unwrap();
        let config = test_config(wt_root.clone());

        let result = create("feature-y", Some("main"), &config, &repo).unwrap();
        assert!(result.exists());
    }

    #[test]
    fn test_create_existing_local_branch() {
        let tmp = TempDir::new().unwrap();
        let repo = tmp.path().join("myproject");
        init_repo(&repo);
        git_cmd(&repo, &["branch", "existing-branch"]);

        let wt_root = tmp.path().join("worktrees");
        fs::create_dir_all(&wt_root).unwrap();
        let config = test_config(wt_root.clone());

        let result = create("existing-branch", None, &config, &repo).unwrap();
        assert!(result.exists());
    }

    #[test]
    fn test_create_target_already_exists() {
        let tmp = TempDir::new().unwrap();
        let repo = tmp.path().join("myproject");
        init_repo(&repo);

        let wt_root = tmp.path().join("worktrees");
        let target = wt_root.join("myproject--feature-x");
        fs::create_dir_all(&target).unwrap();
        let config = test_config(wt_root);

        let result = create("feature-x", None, &config, &repo);
        assert!(result.is_err());
        assert!(
            result.unwrap_err().to_string().contains("already exists"),
        );
    }

    #[test]
    fn test_create_copies_devwork_files() {
        let tmp = TempDir::new().unwrap();
        let repo = tmp.path().join("myproject");
        init_repo(&repo);

        fs::write(repo.join(".devwork"), ".env\n").unwrap();
        fs::write(repo.join(".env"), "DB_HOST=localhost").unwrap();

        let wt_root = tmp.path().join("worktrees");
        fs::create_dir_all(&wt_root).unwrap();
        let config = test_config(wt_root);

        let result = create("feature-x", None, &config, &repo).unwrap();
        assert_eq!(
            fs::read_to_string(result.join(".env")).unwrap(),
            "DB_HOST=localhost"
        );
    }

    // --- delete tests ---

    #[test]
    fn test_delete_worktree() {
        let tmp = TempDir::new().unwrap();
        let repo = tmp.path().join("myproject");
        init_repo(&repo);

        let wt_root = tmp.path().join("worktrees");
        fs::create_dir_all(&wt_root).unwrap();
        let config = test_config(wt_root.clone());

        // Create a worktree first
        let wt_path = create("feature-x", None, &config, &repo).unwrap();
        assert!(wt_path.exists());

        // Delete it
        delete("feature-x", &config, &repo).unwrap();
        assert!(!wt_path.exists());
    }

    #[test]
    fn test_delete_nonexistent_worktree() {
        let tmp = TempDir::new().unwrap();
        let repo = tmp.path().join("myproject");
        init_repo(&repo);

        let wt_root = tmp.path().join("worktrees");
        fs::create_dir_all(&wt_root).unwrap();
        let config = test_config(wt_root);

        // Should not error when worktree doesn't exist
        delete("nonexistent", &config, &repo).unwrap();
    }
}
