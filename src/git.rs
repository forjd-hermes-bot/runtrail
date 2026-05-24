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
        "clean": !context.dirty,
        "upstream": upstream_branch(cwd).ok(),
        "remote_url": normalized_remote_url(cwd).ok(),
        "files": files.into_iter().map(|file| json!({
            "path": file.path,
            "status": file.status,
        })).collect::<Vec<_>>()
    }))
}

pub fn diff_body(cwd: &Path, stat_only: bool) -> Result<Value> {
    let context = git_context(cwd)?;
    let unstaged_stat = git_output(cwd, ["diff", "--stat"])?;
    let staged_stat = git_output(cwd, ["diff", "--cached", "--stat"])?;
    let stat = combine_git_outputs(&[
        ("unstaged", unstaged_stat.as_str()),
        ("staged", staged_stat.as_str()),
    ]);
    let (unstaged_patch, staged_patch, patch) = if stat_only {
        (None, None, None)
    } else {
        let unstaged_patch = git_output(cwd, ["diff", "--patch"])?;
        let staged_patch = git_output(cwd, ["diff", "--cached", "--patch"])?;
        let patch = combine_git_outputs(&[
            ("unstaged", unstaged_patch.as_str()),
            ("staged", staged_patch.as_str()),
        ]);
        (Some(unstaged_patch), Some(staged_patch), Some(patch))
    };
    Ok(json!({
        "repo_root": context.root,
        "branch": context.branch,
        "head": context.head,
        "dirty": context.dirty,
        "stat": stat,
        "patch": patch,
        "unstaged": {
            "stat": unstaged_stat,
            "patch": unstaged_patch,
        },
        "staged": {
            "stat": staged_stat,
            "patch": staged_patch,
        }
    }))
}

fn combine_git_outputs(sections: &[(&str, &str)]) -> String {
    let non_empty = sections
        .iter()
        .filter(|(_, body)| !body.trim().is_empty())
        .collect::<Vec<_>>();
    if non_empty.is_empty() {
        return String::new();
    }
    if non_empty.len() == 1 {
        return non_empty[0].1.to_string();
    }
    non_empty
        .into_iter()
        .map(|(name, body)| format!("# {name}\n{body}"))
        .collect::<Vec<_>>()
        .join("\n\n")
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

fn upstream_branch(cwd: &Path) -> Result<String> {
    git_output(
        cwd,
        ["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"],
    )
}

fn normalized_remote_url(cwd: &Path) -> Result<String> {
    let url = git_output(cwd, ["remote", "get-url", "origin"])?;
    Ok(normalize_remote_url(&url))
}

fn normalize_remote_url(url: &str) -> String {
    let trimmed = url.trim();
    let without_query = trimmed
        .split(['?', '#'])
        .next()
        .unwrap_or(trimmed)
        .to_string();
    let without_credentials = if let Some((scheme, rest)) = without_query.split_once("://") {
        if let Some(at) = rest.rfind('@') {
            format!("{scheme}://{}", &rest[at + 1..])
        } else {
            without_query
        }
    } else if let Some(at) = without_query.rfind('@') {
        let prefix = &without_query[..at];
        if prefix.contains(':') && !prefix.eq_ignore_ascii_case("git") {
            without_query[at + 1..].to_string()
        } else {
            without_query
        }
    } else {
        without_query
    };
    without_credentials.trim_end_matches(".git").to_string()
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

    fn init_repo() -> tempfile::TempDir {
        let dir = tempdir().unwrap();
        git(dir.path(), &["init"]);
        git(dir.path(), &["config", "user.email", "test@example.com"]);
        git(dir.path(), &["config", "user.name", "Test"]);
        fs::write(dir.path().join("README.md"), "hello").unwrap();
        git(dir.path(), &["add", "README.md"]);
        git(dir.path(), &["commit", "-m", "initial"]);
        dir
    }

    #[test]
    fn snapshot_body_reports_dirty_files() {
        let dir = init_repo();
        fs::write(dir.path().join("README.md"), "hello world").unwrap();

        let body = snapshot_body(dir.path()).unwrap();
        assert_eq!(body["dirty"], true);
        assert_eq!(body["clean"], false);
        assert_eq!(body["files"][0]["path"], "README.md");
    }

    #[test]
    fn diff_body_reports_staged_only_changes() {
        let dir = init_repo();
        fs::write(dir.path().join("README.md"), "hello staged").unwrap();
        git(dir.path(), &["add", "README.md"]);

        let body = diff_body(dir.path(), false).unwrap();
        assert_eq!(body["dirty"], true);
        assert!(body["stat"].as_str().unwrap().contains("README.md"));
        assert!(
            body["staged"]["stat"]
                .as_str()
                .unwrap()
                .contains("README.md")
        );
        assert_eq!(body["unstaged"]["stat"], "");
        assert!(body["patch"].as_str().unwrap().contains("hello staged"));
    }

    #[test]
    fn diff_body_can_omit_patches() {
        let dir = init_repo();
        fs::write(dir.path().join("README.md"), "hello world").unwrap();

        let body = diff_body(dir.path(), true).unwrap();
        assert!(body["stat"].as_str().unwrap().contains("README.md"));
        assert!(body["patch"].is_null());
        assert!(body["unstaged"]["patch"].is_null());
    }

    #[test]
    fn normalize_remote_url_strips_credentials_query_fragment_and_git_suffix() {
        assert_eq!(
            normalize_remote_url(
                "https://user:secret@github.com/forjd/runtrail.git?token=abc#frag"
            ),
            "https://github.com/forjd/runtrail"
        );
        assert_eq!(
            normalize_remote_url("user:secret@github.com/forjd/runtrail.git"),
            "github.com/forjd/runtrail"
        );
        assert_eq!(
            normalize_remote_url("git@github.com:forjd/runtrail.git"),
            "git@github.com:forjd/runtrail"
        );
    }
}
