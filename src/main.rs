mod cli;
mod config;
mod discovery;
mod git;
mod init;
mod pretty;
mod session;
mod worktree;

use anyhow::{bail, Context, Result};
use clap::Parser;
use std::env;
use std::io::{IsTerminal, Write};
use std::path::PathBuf;
use std::process::{Command as ProcessCommand, Stdio};

use cli::{Cli, Command};

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    git::check_git_available()?;
    let config = config::load_config()?;

    match cli.command {
        Command::List {
            path,
            json,
            raw,
            porcelain,
        } => cmd_list(path, json, raw, porcelain, &config),
        Command::Resolve { name, path } => cmd_resolve(&name, path, &config),
        Command::Prettify { dir, path } => cmd_prettify(&dir, path, &config),
        Command::Pick { path, finder } => cmd_pick(path, finder.as_deref(), &config),
        Command::Open { path, command } => cmd_open(&path, command.as_deref(), &config),
        Command::Create {
            name,
            source,
            open,
            init,
        } => cmd_create(&name, source.as_deref(), open, init, &config),
        Command::Init {} => cmd_init(),
        Command::Delete {
            name,
            branch,
            force,
        } => cmd_delete(&name, branch, force, &config),
        Command::Complete { subcommand } => cmd_complete(&subcommand),
    }
}

/// Resolve project paths from an optional path argument.
///
/// With an explicit path, discovers projects under it. Without one, if inside
/// a git repo, lists its worktrees; otherwise discovers projects under cwd.
fn resolve_paths(path: Option<PathBuf>, config: &config::Config) -> Result<Vec<PathBuf>> {
    if let Some(root) = path {
        let ignore_set = discovery::build_ignore_set(&config.ignore)?;
        discovery::discover(&root, &ignore_set, config.max_depth)
    } else {
        let cwd = env::current_dir()?;
        if git::repo_root(&cwd).is_ok() {
            git::worktree_list(&cwd)
        } else {
            let ignore_set = discovery::build_ignore_set(&config.ignore)?;
            discovery::discover(&cwd, &ignore_set, config.max_depth)
        }
    }
}

fn cmd_list(
    path: Option<PathBuf>,
    json: bool,
    raw: bool,
    porcelain: bool,
    config: &config::Config,
) -> Result<()> {
    let paths = resolve_paths(path, config)?;

    if json {
        let entries = pretty::build_pretty_names(&paths);
        let json_entries: Vec<serde_json::Value> = entries
            .iter()
            .map(|e| {
                serde_json::json!({
                    "path": e.path.to_string_lossy(),
                    "name": e.display_name,
                    "is_worktree": e.worktree_of.is_some(),
                    "worktree_of": e.worktree_of,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&json_entries)?);
    } else if raw {
        for path in &paths {
            println!("{}", path.display());
        }
    } else {
        let entries = pretty::build_pretty_names(&paths);
        let use_tree = !porcelain && std::io::stdout().is_terminal();
        if use_tree {
            for line in pretty::build_tree_output(&entries) {
                println!("{}", line);
            }
        } else {
            for entry in &entries {
                println!("{}", entry.display_name);
            }
        }
    }

    Ok(())
}

fn cmd_resolve(name: &str, path: Option<PathBuf>, config: &config::Config) -> Result<()> {
    let root = path.unwrap_or(env::current_dir()?);
    let ignore_set = discovery::build_ignore_set(&config.ignore)?;
    let paths = discovery::discover(&root, &ignore_set, config.max_depth)?;
    let resolved = pretty::resolve(name, &paths)?;
    println!("{}", resolved.display());
    Ok(())
}

fn cmd_prettify(
    dir: &std::path::Path,
    path: Option<PathBuf>,
    config: &config::Config,
) -> Result<()> {
    let root = path.unwrap_or(env::current_dir()?);
    let ignore_set = discovery::build_ignore_set(&config.ignore)?;
    let paths = discovery::discover(&root, &ignore_set, config.max_depth)?;
    let name = pretty::prettify(dir, &paths)?;
    println!("{}", name);
    Ok(())
}

fn cmd_pick(path: Option<PathBuf>, finder: Option<&str>, config: &config::Config) -> Result<()> {
    let finder = finder.or(config.finder.as_deref()).ok_or_else(|| {
        anyhow::anyhow!("no finder configured: use -F or set session.finder in config")
    })?;
    let paths = resolve_paths(path, config)?;
    let entries = pretty::build_pretty_names(&paths);

    if entries.is_empty() {
        bail!("no projects found");
    }

    let input: String = entries
        .iter()
        .map(|e| e.display_name.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    let mut child = ProcessCommand::new("sh")
        .arg("-c")
        .arg(finder)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .with_context(|| format!("failed to run finder: {}", finder))?;

    child.stdin.take().unwrap().write_all(input.as_bytes())?;

    let output = child.wait_with_output()?;
    if !output.status.success() {
        bail!("finder exited with non-zero status");
    }

    let selection = String::from_utf8(output.stdout)?;
    let selection = selection.trim();
    if selection.is_empty() {
        bail!("no project selected");
    }

    let resolved = pretty::resolve(selection, &paths)?;
    session::open(&resolved, config.opener.as_deref())
}

fn cmd_open(path: &std::path::Path, command: Option<&str>, config: &config::Config) -> Result<()> {
    if !path.is_absolute() {
        bail!("path must be absolute: {}", path.display());
    }
    if !path.is_dir() {
        bail!("path is not a directory: {}", path.display());
    }
    let opener = command.or(config.opener.as_deref());
    session::open(path, opener)
}

fn cmd_create(
    name: &str,
    source: Option<&str>,
    open: bool,
    run_init: bool,
    config: &config::Config,
) -> Result<()> {
    let cwd = env::current_dir()?;
    let wt_path = worktree::create(name, source, config, &cwd)?;
    eprintln!("created worktree at {}", wt_path.display());
    println!("{}", wt_path.display());

    if run_init || config.auto_init {
        init::run(&wt_path)?;
    }

    if open {
        session::open(&wt_path, config.opener.as_deref())?;
    }

    Ok(())
}

fn cmd_init() -> Result<()> {
    let cwd = env::current_dir()?;
    init::run(&cwd)
}

fn cmd_delete(name: &str, delete_branch: bool, force: bool, config: &config::Config) -> Result<()> {
    let cwd = env::current_dir()?;
    worktree::delete(name, delete_branch, force, config, &cwd)?;
    println!("deleted worktree '{}'", name);
    Ok(())
}

fn cmd_complete(subcommand: &str) -> Result<()> {
    let cwd = env::current_dir()?;

    match subcommand {
        "delete" => {
            // List worktree short names for the current project
            let root = git::repo_root(&cwd)?;
            let project_name = root
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            let prefix = format!("{}--", project_name);

            let worktrees = git::worktree_list(&cwd)?;
            for wt in &worktrees {
                if let Some(name) = wt.file_name().and_then(|n| n.to_str()) {
                    if let Some(short) = name.strip_prefix(&prefix) {
                        println!("{}", short);
                    }
                }
            }
        }
        "open" => {
            // Equivalent to `yawn list`
            let config = config::load_config()?;
            let ignore_set = discovery::build_ignore_set(&config.ignore)?;
            let paths = discovery::discover(&cwd, &ignore_set, config.max_depth)?;
            for path in &paths {
                println!("{}", path.display());
            }
        }
        _ => bail!("unknown completion subcommand: {}", subcommand),
    }

    Ok(())
}
