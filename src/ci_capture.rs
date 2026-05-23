use crate::git;
use serde_json::{Value, json};
use std::fs;
use std::path::Path;

pub fn capture_body(cwd: &Path) -> anyhow::Result<Value> {
    let artifacts_dir = cwd.join(".runtrail/artifacts");
    fs::create_dir_all(&artifacts_dir)?;
    Ok(json!({
        "github": github_context(),
        "repo": {
            "snapshot": git::snapshot_body(cwd)?,
            "diff": git::diff_body(cwd, true)?,
        },
        "dependencies": dependency_metadata(cwd),
        "artifacts": {
            "dir": ".runtrail/artifacts"
        },
        "unsupported": [
            {
                "feature": "services",
                "reason": "service containers and network topology are not captured"
            },
            {
                "feature": "secrets",
                "reason": "secret values are intentionally omitted"
            },
            {
                "feature": "runner",
                "reason": "hosted runner image, permissions, and matrix differences may not replay locally"
            }
        ]
    }))
}

fn github_context() -> Value {
    let keys = [
        "GITHUB_WORKFLOW",
        "GITHUB_RUN_ID",
        "GITHUB_RUN_ATTEMPT",
        "GITHUB_JOB",
        "GITHUB_SHA",
        "GITHUB_REPOSITORY",
        "RUNNER_OS",
        "RUNNER_ARCH",
    ];
    let mut map = serde_json::Map::new();
    for key in keys {
        if let Ok(value) = std::env::var(key) {
            map.insert(key.to_string(), Value::String(value));
        }
    }
    Value::Object(map)
}

fn dependency_metadata(cwd: &Path) -> Value {
    json!({
        "rust": {
            "cargo_toml": cwd.join("Cargo.toml").exists(),
            "cargo_lock": cwd.join("Cargo.lock").exists(),
            "rust_toolchain": cwd.join("rust-toolchain.toml").exists() || cwd.join("rust-toolchain").exists(),
        },
        "node": {
            "package_json": cwd.join("package.json").exists(),
            "lockfile": first_existing(cwd, &["pnpm-lock.yaml", "yarn.lock", "package-lock.json"]),
        },
        "python": {
            "pyproject_toml": cwd.join("pyproject.toml").exists(),
            "lockfile": first_existing(cwd, &["uv.lock", "poetry.lock", "Pipfile.lock", "requirements.txt"]),
        }
    })
}

fn first_existing(cwd: &Path, candidates: &[&str]) -> Option<String> {
    candidates
        .iter()
        .find(|candidate| cwd.join(candidate).exists())
        .map(|candidate| (*candidate).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
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
    fn capture_body_creates_artifact_dir_and_reports_dependency_metadata() {
        let dir = tempdir().unwrap();
        git(dir.path(), &["init"]);
        git(dir.path(), &["config", "user.email", "test@example.com"]);
        git(dir.path(), &["config", "user.name", "Test"]);
        fs::write(dir.path().join("Cargo.toml"), "[package]\nname='x'\n").unwrap();
        git(dir.path(), &["add", "Cargo.toml"]);
        git(dir.path(), &["commit", "-m", "initial"]);

        let body = capture_body(dir.path()).unwrap();

        assert!(dir.path().join(".runtrail/artifacts").exists());
        assert_eq!(body["dependencies"]["rust"]["cargo_toml"], true);
        assert_eq!(body["artifacts"]["dir"], ".runtrail/artifacts");
        assert!(body["unsupported"].as_array().unwrap().len() >= 3);
    }
}
