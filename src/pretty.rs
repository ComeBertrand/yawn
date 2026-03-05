use anyhow::{bail, Result};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// A discovered project with its computed pretty name.
#[derive(Debug, Clone)]
pub struct PrettyEntry {
    pub path: PathBuf,
    pub display_name: String,
}

/// Detect whether a path is a git worktree (`.git` is a file, not a directory).
pub fn is_worktree(path: &Path) -> bool {
    let git_entry = path.join(".git");
    git_entry.is_file()
}

/// For a worktree, parse the `.git` file to find the main repository name.
/// The `.git` file contains a line like `gitdir: /path/to/main/.git/worktrees/<name>`.
/// We extract the main repo's basename from this.
pub fn worktree_main_repo_name(path: &Path) -> Result<String> {
    let git_file = path.join(".git");
    let content = fs::read_to_string(&git_file)?;
    let gitdir = content
        .strip_prefix("gitdir: ")
        .unwrap_or(&content)
        .trim();
    // gitdir is like: /path/to/main-repo/.git/worktrees/<name>
    // We want the main-repo basename.
    let gitdir_path = PathBuf::from(gitdir);
    // Walk up from worktrees/<name> -> .git -> main-repo
    let main_git_dir = gitdir_path
        .parent() // worktrees
        .and_then(|p| p.parent()) // .git
        .and_then(|p| p.parent()) // main-repo
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_string());
    match main_git_dir {
        Some(name) => Ok(name),
        None => bail!("could not determine main repo from worktree at {}", path.display()),
    }
}

/// Build pretty names for a list of discovered project paths.
///
/// Rules:
/// 1. Base display name is the directory basename.
/// 2. If worktree: strip `<project>--` prefix, annotate with `[worktree of <project>]`.
/// 3. Disambiguate collisions with shortest unique parent path suffix.
pub fn build_pretty_names(paths: &[PathBuf]) -> Vec<PrettyEntry> {
    // Step 1: compute base names
    let mut entries: Vec<(PathBuf, String, Option<String>)> = Vec::new(); // (path, base_name, worktree_annotation)

    for path in paths {
        let basename = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        if is_worktree(path) {
            if let Ok(main_name) = worktree_main_repo_name(path) {
                let prefix = format!("{}--", main_name);
                let short_name = if basename.starts_with(&prefix) {
                    basename[prefix.len()..].to_string()
                } else {
                    basename.clone()
                };
                entries.push((
                    path.clone(),
                    short_name,
                    Some(format!("[worktree of {}]", main_name)),
                ));
                continue;
            }
        }

        entries.push((path.clone(), basename, None));
    }

    // Step 2: find collisions on base name and disambiguate
    let mut name_counts: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, (_, name, _)) in entries.iter().enumerate() {
        name_counts.entry(name.clone()).or_default().push(i);
    }

    let mut result: Vec<PrettyEntry> = entries
        .iter()
        .map(|(path, name, annotation)| {
            let display = match annotation {
                Some(ann) => format!("{} {}", name, ann),
                None => name.clone(),
            };
            PrettyEntry {
                path: path.clone(),
                display_name: display,
            }
        })
        .collect();

    // Disambiguate collisions
    for (_name, indices) in &name_counts {
        if indices.len() <= 1 {
            continue;
        }

        // Find the shortest unique parent path suffix for each colliding entry
        let paths_for_collision: Vec<&Path> = indices.iter().map(|&i| entries[i].0.as_path()).collect();
        let suffixes = shortest_unique_suffixes(&paths_for_collision);

        for (j, &idx) in indices.iter().enumerate() {
            let (_, ref base_name, ref annotation) = entries[idx];
            let display = match annotation {
                Some(ann) => format!("{} ({}) {}", base_name, suffixes[j], ann),
                None => format!("{} ({})", base_name, suffixes[j]),
            };
            result[idx].display_name = display;
        }
    }

    result
}

/// Find the shortest unique parent path suffix to disambiguate a set of paths.
///
/// For example, given:
///   /home/user/Workspace/mnemo
///   /home/user/Documents/projects/mnemo
///
/// Returns: ["Workspace", "Documents/projects"]
/// (the shortest suffix of the parent path that makes each unique)
fn shortest_unique_suffixes(paths: &[&Path]) -> Vec<String> {
    let parents: Vec<Vec<String>> = paths
        .iter()
        .map(|p| {
            let parent = p.parent().unwrap_or(Path::new(""));
            parent
                .components()
                .map(|c| c.as_os_str().to_string_lossy().to_string())
                .collect::<Vec<_>>()
        })
        .collect();

    let n = paths.len();
    let mut suffixes = vec![String::new(); n];

    // Start with 1 component from the end, increase until all are unique
    let max_components = parents.iter().map(|p| p.len()).max().unwrap_or(0);

    for depth in 1..=max_components {
        let tails: Vec<String> = parents
            .iter()
            .map(|components| {
                let start = components.len().saturating_sub(depth);
                components[start..].join("/")
            })
            .collect();

        // Check which are now unique
        let mut seen: HashMap<&str, Vec<usize>> = HashMap::new();
        for (i, tail) in tails.iter().enumerate() {
            seen.entry(tail.as_str()).or_default().push(i);
        }

        for (i, tail) in tails.iter().enumerate() {
            if suffixes[i].is_empty() && seen[tail.as_str()].len() == 1 {
                suffixes[i] = tail.clone();
            }
        }

        if suffixes.iter().all(|s| !s.is_empty()) {
            break;
        }
    }

    suffixes
}

/// Resolve a pretty name to an absolute path.
///
/// Builds the same pretty names and finds the matching entry.
/// Errors if no match or ambiguous.
pub fn resolve(pretty_name: &str, paths: &[PathBuf]) -> Result<PathBuf> {
    let entries = build_pretty_names(paths);
    let matches: Vec<&PrettyEntry> = entries
        .iter()
        .filter(|e| e.display_name == pretty_name)
        .collect();

    match matches.len() {
        0 => bail!("no project matches '{}'", pretty_name),
        1 => Ok(matches[0].path.clone()),
        _ => bail!("ambiguous name '{}' matches {} projects", pretty_name, matches.len()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn make_git_repo(path: &Path) {
        fs::create_dir_all(path).unwrap();
        fs::create_dir(path.join(".git")).unwrap();
    }

    fn make_git_worktree(path: &Path, main_repo: &Path) {
        fs::create_dir_all(path).unwrap();
        let worktree_name = path.file_name().unwrap().to_string_lossy();
        let gitdir = main_repo
            .join(".git")
            .join("worktrees")
            .join(worktree_name.as_ref());
        fs::create_dir_all(&gitdir).unwrap();
        fs::write(
            path.join(".git"),
            format!("gitdir: {}", gitdir.display()),
        )
        .unwrap();
    }

    #[test]
    fn test_is_worktree_regular_repo() {
        let tmp = TempDir::new().unwrap();
        let repo = tmp.path().join("repo");
        make_git_repo(&repo);
        assert!(!is_worktree(&repo));
    }

    #[test]
    fn test_is_worktree_actual_worktree() {
        let tmp = TempDir::new().unwrap();
        let main_repo = tmp.path().join("main");
        make_git_repo(&main_repo);
        let wt = tmp.path().join("main--feature");
        make_git_worktree(&wt, &main_repo);
        assert!(is_worktree(&wt));
    }

    #[test]
    fn test_worktree_main_repo_name() {
        let tmp = TempDir::new().unwrap();
        let main_repo = tmp.path().join("cargostack-backend");
        make_git_repo(&main_repo);
        let wt = tmp.path().join("cargostack-backend--fix-branch");
        make_git_worktree(&wt, &main_repo);

        let name = worktree_main_repo_name(&wt).unwrap();
        assert_eq!(name, "cargostack-backend");
    }

    #[test]
    fn test_pretty_simple_repos() {
        let tmp = TempDir::new().unwrap();
        let repo_a = tmp.path().join("alpha");
        let repo_b = tmp.path().join("beta");
        make_git_repo(&repo_a);
        make_git_repo(&repo_b);

        let paths = vec![repo_a, repo_b];
        let entries = build_pretty_names(&paths);
        assert_eq!(entries[0].display_name, "alpha");
        assert_eq!(entries[1].display_name, "beta");
    }

    #[test]
    fn test_pretty_worktree_annotation() {
        let tmp = TempDir::new().unwrap();
        let main_repo = tmp.path().join("cargostack-backend");
        make_git_repo(&main_repo);
        let wt = tmp.path().join("cargostack-backend--fix-branch");
        make_git_worktree(&wt, &main_repo);

        let paths = vec![main_repo, wt];
        let entries = build_pretty_names(&paths);
        assert_eq!(entries[0].display_name, "cargostack-backend");
        assert_eq!(
            entries[1].display_name,
            "fix-branch [worktree of cargostack-backend]"
        );
    }

    #[test]
    fn test_pretty_collision_disambiguation() {
        let tmp = TempDir::new().unwrap();
        let mnemo_ws = tmp.path().join("Workspace").join("mnemo");
        let mnemo_docs = tmp.path().join("Documents").join("projects").join("mnemo");
        make_git_repo(&mnemo_ws);
        make_git_repo(&mnemo_docs);

        let paths = vec![mnemo_ws, mnemo_docs];
        let entries = build_pretty_names(&paths);

        // Both are "mnemo" so they need disambiguation
        assert!(
            entries[0].display_name.contains("Workspace"),
            "expected Workspace disambiguation, got: {}",
            entries[0].display_name
        );
        assert!(
            entries[1].display_name.contains("projects"),
            "expected projects disambiguation, got: {}",
            entries[1].display_name
        );
    }

    #[test]
    fn test_pretty_collision_shortest_suffix() {
        let tmp = TempDir::new().unwrap();
        // These share "projects" parent, so we need to go one level up
        let mnemo_a = tmp.path().join("a").join("projects").join("mnemo");
        let mnemo_b = tmp.path().join("b").join("projects").join("mnemo");
        make_git_repo(&mnemo_a);
        make_git_repo(&mnemo_b);

        let paths = vec![mnemo_a, mnemo_b];
        let entries = build_pretty_names(&paths);

        // "projects" alone isn't unique, so should include "a/projects" and "b/projects"
        assert!(
            entries[0].display_name.contains("a/projects")
                || entries[0].display_name.contains("a"),
            "expected 'a' disambiguation, got: {}",
            entries[0].display_name
        );
        assert!(
            entries[1].display_name.contains("b/projects")
                || entries[1].display_name.contains("b"),
            "expected 'b' disambiguation, got: {}",
            entries[1].display_name
        );
    }

    #[test]
    fn test_pretty_no_collision_no_suffix() {
        let tmp = TempDir::new().unwrap();
        let alpha = tmp.path().join("alpha");
        let beta = tmp.path().join("beta");
        make_git_repo(&alpha);
        make_git_repo(&beta);

        let paths = vec![alpha, beta];
        let entries = build_pretty_names(&paths);
        assert!(!entries[0].display_name.contains('('));
        assert!(!entries[1].display_name.contains('('));
    }

    #[test]
    fn test_pretty_worktree_with_collision() {
        // A worktree whose short name collides with another project
        let tmp = TempDir::new().unwrap();

        let feature_repo = tmp.path().join("Workspace").join("feature");
        make_git_repo(&feature_repo);

        let main_repo = tmp.path().join("worktrees").join("myapp");
        make_git_repo(&main_repo);

        let wt = tmp.path().join("worktrees").join("myapp--feature");
        make_git_worktree(&wt, &main_repo);

        let paths = vec![feature_repo, wt];
        let entries = build_pretty_names(&paths);

        // Both have base name "feature", so both should be disambiguated
        // The worktree should also have its annotation
        let wt_entry = &entries[1];
        assert!(
            wt_entry.display_name.contains("[worktree of myapp]"),
            "expected worktree annotation, got: {}",
            wt_entry.display_name
        );
        assert!(
            wt_entry.display_name.contains('('),
            "expected disambiguation, got: {}",
            wt_entry.display_name
        );
    }

    #[test]
    fn test_resolve_exact_match() {
        let tmp = TempDir::new().unwrap();
        let repo_a = tmp.path().join("alpha");
        let repo_b = tmp.path().join("beta");
        make_git_repo(&repo_a);
        make_git_repo(&repo_b);

        let paths = vec![repo_a.clone(), repo_b];
        let result = resolve("alpha", &paths).unwrap();
        assert_eq!(result, repo_a);
    }

    #[test]
    fn test_resolve_worktree() {
        let tmp = TempDir::new().unwrap();
        let main_repo = tmp.path().join("myapp");
        make_git_repo(&main_repo);
        let wt = tmp.path().join("myapp--feature");
        make_git_worktree(&wt, &main_repo);

        let paths = vec![main_repo, wt.clone()];
        let result = resolve("feature [worktree of myapp]", &paths).unwrap();
        assert_eq!(result, wt);
    }

    #[test]
    fn test_resolve_no_match() {
        let tmp = TempDir::new().unwrap();
        let repo = tmp.path().join("alpha");
        make_git_repo(&repo);

        let paths = vec![repo];
        let result = resolve("nonexistent", &paths);
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_disambiguated_name() {
        let tmp = TempDir::new().unwrap();
        let mnemo_ws = tmp.path().join("Workspace").join("mnemo");
        let mnemo_docs = tmp.path().join("Documents").join("mnemo");
        make_git_repo(&mnemo_ws);
        make_git_repo(&mnemo_docs);

        let paths = vec![mnemo_ws.clone(), mnemo_docs.clone()];
        let entries = build_pretty_names(&paths);

        // Resolve using the disambiguated name
        let result = resolve(&entries[0].display_name, &paths).unwrap();
        assert_eq!(result, mnemo_ws);

        let result = resolve(&entries[1].display_name, &paths).unwrap();
        assert_eq!(result, mnemo_docs);
    }

    #[test]
    fn test_shortest_unique_suffixes_simple() {
        let a = PathBuf::from("/home/user/Workspace/mnemo");
        let b = PathBuf::from("/home/user/Documents/projects/mnemo");
        let paths: Vec<&Path> = vec![a.as_path(), b.as_path()];
        let suffixes = shortest_unique_suffixes(&paths);
        assert_eq!(suffixes[0], "Workspace");
        assert_eq!(suffixes[1], "projects");
    }

    #[test]
    fn test_shortest_unique_suffixes_deeper() {
        let a = PathBuf::from("/x/a/shared/mnemo");
        let b = PathBuf::from("/x/b/shared/mnemo");
        let paths: Vec<&Path> = vec![a.as_path(), b.as_path()];
        let suffixes = shortest_unique_suffixes(&paths);
        // "shared" is the same for both, so need "a/shared" and "b/shared"
        assert_eq!(suffixes[0], "a/shared");
        assert_eq!(suffixes[1], "b/shared");
    }

    #[test]
    fn test_three_way_collision() {
        let tmp = TempDir::new().unwrap();
        let a = tmp.path().join("x").join("mnemo");
        let b = tmp.path().join("y").join("mnemo");
        let c = tmp.path().join("z").join("mnemo");
        make_git_repo(&a);
        make_git_repo(&b);
        make_git_repo(&c);

        let paths = vec![a, b, c];
        let entries = build_pretty_names(&paths);

        // All three should be disambiguated and unique
        let names: Vec<&str> = entries.iter().map(|e| e.display_name.as_str()).collect();
        assert_eq!(names.len(), 3);
        assert_ne!(names[0], names[1]);
        assert_ne!(names[1], names[2]);
        assert_ne!(names[0], names[2]);
    }
}
