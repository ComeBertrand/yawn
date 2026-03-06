mod cli;
mod config;
mod discovery;
mod git;
mod pretty;
mod session;
mod worktree;

use anyhow::{bail, Context, Result};
use clap::Parser;
use std::env;
use std::io::Write;
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
        Command::List { path, pretty } => cmd_list(path, pretty, &config),
        Command::Resolve { name, path } => cmd_resolve(&name, path, &config),
        Command::Pick { path, finder } => cmd_pick(path, &finder, &config),
        Command::Open { path } => cmd_open(&path, &config),
        Command::Create { name, source, open } => {
            cmd_create(&name, source.as_deref(), open, &config)
        }
        Command::Delete { name } => cmd_delete(&name, &config),
        Command::Complete { subcommand } => cmd_complete(&subcommand),
    }
}

fn cmd_list(path: Option<PathBuf>, pretty: bool, config: &config::Config) -> Result<()> {
    let paths = if let Some(root) = path {
        // Explicit path: always discover
        let ignore_set = discovery::build_ignore_set(&config.ignore)?;
        discovery::discover(&root, &ignore_set, config.max_depth)?
    } else {
        let cwd = env::current_dir()?;
        if git::repo_root(&cwd).is_ok() {
            // Inside a git repo: list its worktrees
            git::worktree_list(&cwd)?
        } else {
            // Not in a git repo: discover projects under cwd
            let ignore_set = discovery::build_ignore_set(&config.ignore)?;
            discovery::discover(&cwd, &ignore_set, config.max_depth)?
        }
    };

    if pretty {
        let entries = pretty::build_pretty_names(&paths);
        for entry in &entries {
            println!("{}", entry.display_name);
        }
    } else {
        for path in &paths {
            println!("{}", path.display());
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

fn cmd_pick(path: Option<PathBuf>, finder: &str, config: &config::Config) -> Result<()> {
    let root = path.unwrap_or(env::current_dir()?);
    let ignore_set = discovery::build_ignore_set(&config.ignore)?;
    let paths = discovery::discover(&root, &ignore_set, config.max_depth)?;
    let entries = pretty::build_pretty_names(&paths);

    if entries.is_empty() {
        bail!("no projects found under {}", root.display());
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
    session::open(&resolved, config.open_command.as_deref())
}

fn cmd_open(path: &std::path::Path, config: &config::Config) -> Result<()> {
    if !path.is_absolute() {
        bail!("path must be absolute: {}", path.display());
    }
    if !path.is_dir() {
        bail!("path is not a directory: {}", path.display());
    }
    session::open(path, config.open_command.as_deref())
}

fn cmd_create(name: &str, source: Option<&str>, open: bool, config: &config::Config) -> Result<()> {
    let cwd = env::current_dir()?;
    let wt_path = worktree::create(name, source, config, &cwd)?;
    println!("created worktree at {}", wt_path.display());

    if open {
        session::open(&wt_path, config.open_command.as_deref())?;
    }

    Ok(())
}

fn cmd_delete(name: &str, config: &config::Config) -> Result<()> {
    let cwd = env::current_dir()?;
    worktree::delete(name, config, &cwd)?;
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
