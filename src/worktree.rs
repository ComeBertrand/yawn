use anyhow::{bail, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::git;

/// Parse a `.yawninclude` file and return the list of file paths to copy.
/// Blank lines and lines starting with `#` are ignored.
pub fn parse_devwork(content: &str) -> Vec<String> {
    content
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| line.to_string())
        .collect()
}

/// Expand a pattern relative to a directory, supporting globs.
/// If the pattern contains glob characters, expand it; otherwise treat it as a literal path.
fn expand_pattern(base: &Path, pattern: &str) -> Vec<PathBuf> {
    if pattern.contains('*') || pattern.contains('?') || pattern.contains('[') {
        let glob = match globset::Glob::new(pattern) {
            Ok(g) => g.compile_matcher(),
            Err(_) => return Vec::new(),
        };
        collect_files(base, base, &glob).unwrap_or_default()
    } else {
        let path = base.join(pattern);
        if path.exists() {
            vec![PathBuf::from(pattern)]
        } else {
            Vec::new()
        }
    }
}

/// Recursively collect files under `dir` that match `glob`, returning paths relative to `base`.
fn collect_files(base: &Path, dir: &Path, glob: &globset::GlobMatcher) -> Result<Vec<PathBuf>> {
    let mut results = Vec::new();
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return Ok(results),
    };
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let rel = path.strip_prefix(base).unwrap_or(&path);
        if path.is_dir() {
            results.extend(collect_files(base, &path, glob)?);
        } else if glob.is_match(rel) {
            results.push(rel.to_path_buf());
        }
    }
    Ok(results)
}

/// Copy files listed in `.yawninclude` from the main repo to the worktree.
/// Patterns support globs (e.g. `data_file*.csv`, `config/*.toml`).
pub fn copy_devwork_files(main_repo: &Path, worktree: &Path) -> Result<()> {
    let devwork_path = main_repo.join(".yawninclude");
    if !devwork_path.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(&devwork_path)?;
    let patterns = parse_devwork(&content);

    for pattern in &patterns {
        let files = expand_pattern(main_repo, pattern);
        for file in &files {
            let src = main_repo.join(file);
            let dst = worktree.join(file);
            if let Some(parent) = dst.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&src, &dst)
                .with_context(|| format!("failed to copy {} to worktree", file.display()))?;
        }
    }

    Ok(())
}

/// Create a worktree for the current project.
///
/// Returns the path to the created worktree.
pub fn create(name: &str, source: Option<&str>, config: &Config, cwd: &Path) -> Result<PathBuf> {
    let main_root = git::repo_root(cwd)
        .context("not inside a git repository — run this from within a project")?;
    let project_name = main_root
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let target = config
        .worktree_root
        .join(format!("{}--{}", project_name, name));

    if target.exists() {
        bail!("worktree directory already exists: {}", target.display());
    }

    // Auto-create worktree root if it doesn't exist
    if !config.worktree_root.exists() {
        fs::create_dir_all(&config.worktree_root).with_context(|| {
            format!(
                "failed to create worktree root: {}",
                config.worktree_root.display()
            )
        })?;
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
    let main_root = git::repo_root(cwd)
        .context("not inside a git repository — run this from within a project")?;
    let project_name = main_root
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let target = config
        .worktree_root
        .join(format!("{}--{}", project_name, name));

    // Try git worktree remove first
    if git::worktree_remove(cwd, &target).is_err() {
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
            opener: None,
            finder: None,
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

        // Should succeed silently when .yawninclude doesn't exist
        copy_devwork_files(&main_repo, &worktree).unwrap();
    }

    #[test]
    fn test_copy_devwork_files_copies_listed_files() {
        let tmp = TempDir::new().unwrap();
        let main_repo = tmp.path().join("main");
        let worktree = tmp.path().join("wt");
        fs::create_dir_all(&main_repo).unwrap();
        fs::create_dir_all(&worktree).unwrap();

        // Create .yawninclude and the files it references
        fs::write(main_repo.join(".yawninclude"), ".env\nconfig/local.toml\n").unwrap();
        fs::write(main_repo.join(".env"), "SECRET=123").unwrap();
        fs::create_dir_all(main_repo.join("config")).unwrap();
        fs::write(main_repo.join("config/local.toml"), "[db]\nhost=localhost").unwrap();

        copy_devwork_files(&main_repo, &worktree).unwrap();

        assert_eq!(
            fs::read_to_string(worktree.join(".env")).unwrap(),
            "SECRET=123"
        );
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

        fs::write(main_repo.join(".yawninclude"), ".env\nmissing-file\n").unwrap();
        fs::write(main_repo.join(".env"), "SECRET=123").unwrap();

        // Should not error on missing source file
        copy_devwork_files(&main_repo, &worktree).unwrap();
        assert!(worktree.join(".env").exists());
        assert!(!worktree.join("missing-file").exists());
    }

    #[test]
    fn test_copy_devwork_files_glob_pattern() {
        let tmp = TempDir::new().unwrap();
        let main_repo = tmp.path().join("main");
        let worktree = tmp.path().join("wt");
        fs::create_dir_all(&main_repo).unwrap();
        fs::create_dir_all(&worktree).unwrap();

        fs::write(main_repo.join(".yawninclude"), "data_*.csv\n").unwrap();
        fs::write(main_repo.join("data_users.csv"), "id,name").unwrap();
        fs::write(main_repo.join("data_orders.csv"), "id,total").unwrap();
        fs::write(main_repo.join("other.csv"), "should not be copied").unwrap();

        copy_devwork_files(&main_repo, &worktree).unwrap();
        assert!(worktree.join("data_users.csv").exists());
        assert!(worktree.join("data_orders.csv").exists());
        assert!(!worktree.join("other.csv").exists());
    }

    #[test]
    fn test_copy_devwork_files_glob_in_subdir() {
        let tmp = TempDir::new().unwrap();
        let main_repo = tmp.path().join("main");
        let worktree = tmp.path().join("wt");
        fs::create_dir_all(&main_repo).unwrap();
        fs::create_dir_all(&worktree).unwrap();

        fs::write(main_repo.join(".yawninclude"), "config/*.toml\n").unwrap();
        fs::create_dir_all(main_repo.join("config")).unwrap();
        fs::write(main_repo.join("config/dev.toml"), "dev").unwrap();
        fs::write(main_repo.join("config/test.toml"), "test").unwrap();
        fs::write(main_repo.join("config/keep.json"), "not matched").unwrap();

        copy_devwork_files(&main_repo, &worktree).unwrap();
        assert!(worktree.join("config/dev.toml").exists());
        assert!(worktree.join("config/test.toml").exists());
        assert!(!worktree.join("config/keep.json").exists());
    }

    #[test]
    fn test_copy_devwork_files_glob_no_matches() {
        let tmp = TempDir::new().unwrap();
        let main_repo = tmp.path().join("main");
        let worktree = tmp.path().join("wt");
        fs::create_dir_all(&main_repo).unwrap();
        fs::create_dir_all(&worktree).unwrap();

        fs::write(main_repo.join(".yawninclude"), "*.xyz\n").unwrap();

        // No matches should not error
        copy_devwork_files(&main_repo, &worktree).unwrap();
    }

    #[test]
    fn test_copy_devwork_files_mixed_literal_and_glob() {
        let tmp = TempDir::new().unwrap();
        let main_repo = tmp.path().join("main");
        let worktree = tmp.path().join("wt");
        fs::create_dir_all(&main_repo).unwrap();
        fs::create_dir_all(&worktree).unwrap();

        fs::write(main_repo.join(".yawninclude"), ".env\ndata_*.csv\n").unwrap();
        fs::write(main_repo.join(".env"), "SECRET=123").unwrap();
        fs::write(main_repo.join("data_a.csv"), "a").unwrap();
        fs::write(main_repo.join("data_b.csv"), "b").unwrap();

        copy_devwork_files(&main_repo, &worktree).unwrap();
        assert!(worktree.join(".env").exists());
        assert!(worktree.join("data_a.csv").exists());
        assert!(worktree.join("data_b.csv").exists());
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
        assert!(result.unwrap_err().to_string().contains("already exists"),);
    }

    #[test]
    fn test_create_copies_devwork_files() {
        let tmp = TempDir::new().unwrap();
        let repo = tmp.path().join("myproject");
        init_repo(&repo);

        fs::write(repo.join(".yawninclude"), ".env\n").unwrap();
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
