use anyhow::{Context, Result};
use serde_json::{Value, json};
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct GitContext {
    pub root: String,
    pub branch: String,
    pub head: String,
    pub dirty: bool,
}

#[derive(Debug, Clone)]
pub struct GitFileChange {
    pub path: String,
    pub status: String,
}

pub fn snapshot_body(cwd: &Path) -> Result<Value> {
    let context = git_context(cwd)?;
    let files = status_files(cwd)?;
    Ok(json!({
        "repo_root": context.root,
        "branch": context.branch,
        "head": context.head,
        "dirty": context.dirty,
        "files": files.into_iter().map(|file| json!({
            "path": file.path,
            "status": file.status,
        })).collect::<Vec<_>>()
    }))
}

pub fn diff_body(cwd: &Path, stat_only: bool) -> Result<Value> {
    let context = git_context(cwd)?;
    let stat = git_output(cwd, ["diff", "--stat"])?;
    let diff = if stat_only {
        None
    } else {
        Some(git_output(cwd, ["diff", "--patch"])?)
    };
    Ok(json!({
        "repo_root": context.root,
        "branch": context.branch,
        "head": context.head,
        "dirty": context.dirty,
        "stat": stat,
        "patch": diff,
    }))
}

fn git_context(cwd: &Path) -> Result<GitContext> {
    let root = git_output(cwd, ["rev-parse", "--show-toplevel"])?;
    let branch = git_output(cwd, ["branch", "--show-current"])?;
    let head = git_output(cwd, ["rev-parse", "HEAD"])?;
    let dirty = !git_output(cwd, ["status", "--porcelain"])?
        .trim()
        .is_empty();
    Ok(GitContext {
        root,
        branch,
        head,
        dirty,
    })
}

fn status_files(cwd: &Path) -> Result<Vec<GitFileChange>> {
    let output = git_output(cwd, ["status", "--porcelain"])?;
    Ok(output
        .lines()
        .filter_map(|line| {
            if line.len() < 4 {
                return None;
            }
            let status = line[..2].trim().to_string();
            let path = line[3..].to_string();
            Some(GitFileChange { path, status })
        })
        .collect())
}

fn git_output<const N: usize>(cwd: &Path, args: [&str; N]) -> Result<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .with_context(|| "failed to run git")?;
    if !output.status.success() {
        anyhow::bail!(
            "git failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .trim_end()
        .to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn git(dir: &Path, args: &[&str]) {
        let status = Command::new("git")
            .args(args)
            .current_dir(dir)
            .status()
            .unwrap();
        assert!(status.success());
    }

    #[test]
    fn snapshot_body_reports_dirty_files() {
        let dir = tempdir().unwrap();
        git(dir.path(), &["init"]);
        git(dir.path(), &["config", "user.email", "test@example.com"]);
        git(dir.path(), &["config", "user.name", "Test"]);
        fs::write(dir.path().join("README.md"), "hello").unwrap();
        git(dir.path(), &["add", "README.md"]);
        git(dir.path(), &["commit", "-m", "initial"]);
        fs::write(dir.path().join("README.md"), "hello world").unwrap();

        let body = snapshot_body(dir.path()).unwrap();
        assert_eq!(body["dirty"], true);
        assert_eq!(body["files"][0]["path"], "README.md");
    }
}
