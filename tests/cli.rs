use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use tempfile::tempdir;

#[test]
fn log_message_appends_event_and_prints_json() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("events.jsonl");

    let output = Command::cargo_bin("runtrail")
        .unwrap()
        .args([
            "log",
            "--file",
            file.to_str().unwrap(),
            "--event",
            "agent.note",
            "--message",
            "hello",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let printed: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(printed["event"], "agent.note");
    assert_eq!(printed["body"]["message"], "hello");

    let raw = std::fs::read_to_string(file).unwrap();
    let stored: Value = serde_json::from_str(raw.trim()).unwrap();
    assert_eq!(stored["seq"], 1);
}

#[test]
fn log_parses_attrs_and_body_json() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("nested/events.jsonl");

    Command::cargo_bin("runtrail")
        .unwrap()
        .args([
            "log",
            "--file",
            file.to_str().unwrap(),
            "--event",
            "command.run",
            "--attr",
            "exit_code=0",
            "--attr",
            "tool.name=terminal",
            "--body",
            r#"{"cmd":"cargo test"}"#,
        ])
        .assert()
        .success();

    let raw = std::fs::read_to_string(file).unwrap();
    let stored: Value = serde_json::from_str(raw.trim()).unwrap();
    assert_eq!(stored["attrs"]["exit_code"], 0);
    assert_eq!(stored["attrs"]["tool.name"], "terminal");
    assert_eq!(stored["body"]["cmd"], "cargo test");
}

#[test]
fn log_requires_event_name() {
    Command::cargo_bin("runtrail")
        .unwrap()
        .arg("log")
        .assert()
        .failure()
        .stderr(predicate::str::contains("event"));
}

#[test]
fn tail_shows_recent_events_as_json() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("events.jsonl");
    for idx in 0..3 {
        Command::cargo_bin("runtrail")
            .unwrap()
            .args([
                "log",
                "--file",
                file.to_str().unwrap(),
                "--event",
                "agent.note",
                "--message",
                &format!("msg-{idx}"),
            ])
            .assert()
            .success();
    }

    let output = Command::cargo_bin("runtrail")
        .unwrap()
        .args([
            "tail",
            "--file",
            file.to_str().unwrap(),
            "--lines",
            "2",
            "--json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let lines: Vec<_> = String::from_utf8(output)
        .unwrap()
        .lines()
        .map(str::to_string)
        .collect();
    assert_eq!(lines.len(), 2);
    let first: Value = serde_json::from_str(&lines[0]).unwrap();
    assert_eq!(first["seq"], 2);
}

#[test]
fn validate_reports_valid_and_invalid_logs() {
    let dir = tempdir().unwrap();
    let good = dir.path().join("good.jsonl");
    Command::cargo_bin("runtrail")
        .unwrap()
        .args([
            "log",
            "--file",
            good.to_str().unwrap(),
            "--event",
            "agent.note",
        ])
        .assert()
        .success();
    Command::cargo_bin("runtrail")
        .unwrap()
        .args(["validate", "--file", good.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("valid"));

    let bad = dir.path().join("bad.jsonl");
    std::fs::write(&bad, "{nope}\n").unwrap();
    Command::cargo_bin("runtrail")
        .unwrap()
        .args(["validate", "--file", bad.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("line 1"));
}

#[test]
fn validate_strict_rejects_seq_that_does_not_match_line_number() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("events.jsonl");
    Command::cargo_bin("runtrail")
        .unwrap()
        .args([
            "log",
            "--file",
            file.to_str().unwrap(),
            "--event",
            "agent.note",
        ])
        .assert()
        .success();

    let mut event: Value = serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    event["seq"] = Value::from(7);
    std::fs::write(
        &file,
        format!("{}\n", serde_json::to_string(&event).unwrap()),
    )
    .unwrap();

    Command::cargo_bin("runtrail")
        .unwrap()
        .args(["validate", "--file", file.to_str().unwrap()])
        .assert()
        .success();
    Command::cargo_bin("runtrail")
        .unwrap()
        .args(["validate", "--file", file.to_str().unwrap(), "--strict"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "strict mode: seq must match line number 1",
        ));
}

#[test]
fn validate_strict_accepts_seq_that_matches_line_number() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("events.jsonl");
    for idx in 0..2 {
        Command::cargo_bin("runtrail")
            .unwrap()
            .args([
                "log",
                "--file",
                file.to_str().unwrap(),
                "--event",
                "agent.note",
                "--message",
                &format!("msg-{idx}"),
            ])
            .assert()
            .success();
    }

    Command::cargo_bin("runtrail")
        .unwrap()
        .args(["validate", "--file", file.to_str().unwrap(), "--strict"])
        .assert()
        .success()
        .stdout(predicate::str::contains("valid"));
}

#[test]
fn golden_example_fixtures_are_valid_and_strictly_sequenced() {
    for fixture in [
        "examples/agent-session.jsonl",
        "examples/browser-qa.jsonl",
        "examples/ci-failure.jsonl",
        "examples/command-failure.jsonl",
        "examples/repair-handoff.jsonl",
    ] {
        Command::cargo_bin("runtrail")
            .unwrap()
            .args(["validate", "--file", fixture, "--strict"])
            .assert()
            .success()
            .stdout(predicate::str::contains("valid"));

        let raw = std::fs::read_to_string(fixture).unwrap();
        assert!(
            raw.lines()
                .all(|line| line.contains(r#""schema":"runtrail.v1""#))
        );
    }
}

#[test]
fn summarise_outputs_counts_warnings_and_recent_events() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("events.jsonl");
    Command::cargo_bin("runtrail")
        .unwrap()
        .args([
            "log",
            "--file",
            file.to_str().unwrap(),
            "--event",
            "agent.note",
            "--message",
            "hello",
        ])
        .assert()
        .success();
    Command::cargo_bin("runtrail")
        .unwrap()
        .args([
            "log",
            "--file",
            file.to_str().unwrap(),
            "--event",
            "error",
            "--level",
            "error",
            "--body",
            r#"{"error":"boom"}"#,
        ])
        .assert()
        .success();

    Command::cargo_bin("runtrail")
        .unwrap()
        .args(["summarise", "--file", file.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Total events: 2"))
        .stdout(predicate::str::contains("`error`: 1"))
        .stdout(predicate::str::contains("boom"));
}

#[test]
fn diff_outputs_added_removed_and_new_errors() {
    let dir = tempdir().unwrap();
    let before = dir.path().join("before.jsonl");
    let after = dir.path().join("after.jsonl");
    Command::cargo_bin("runtrail")
        .unwrap()
        .args([
            "log",
            "--file",
            before.to_str().unwrap(),
            "--event",
            "agent.note",
        ])
        .assert()
        .success();
    std::fs::copy(&before, &after).unwrap();
    Command::cargo_bin("runtrail")
        .unwrap()
        .args([
            "log",
            "--file",
            after.to_str().unwrap(),
            "--event",
            "error",
            "--level",
            "error",
            "--message",
            "boom",
        ])
        .assert()
        .success();

    Command::cargo_bin("runtrail")
        .unwrap()
        .args(["diff", before.to_str().unwrap(), after.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Delta: 1"))
        .stdout(predicate::str::contains("New warnings and errors"))
        .stdout(predicate::str::contains("boom"));
}

#[test]
fn ci_github_context_logs_allowlisted_environment() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("events.jsonl");
    let mut cmd = Command::cargo_bin("runtrail").unwrap();
    cmd.args(["ci", "github-context", "--file", file.to_str().unwrap()])
        .env("GITHUB_RUN_ID", "123")
        .env("GITHUB_RUN_ATTEMPT", "2")
        .env("GITHUB_WORKFLOW", "CI")
        .env("GITHUB_SHA", "abc123")
        .env("GITHUB_REPOSITORY", "owner/repo")
        .env("SECRET_TOKEN", "do-not-log")
        .assert()
        .success();

    let raw = std::fs::read_to_string(file).unwrap();
    assert!(raw.contains("ci.github.context"));
    assert!(raw.contains("GITHUB_RUN_ID"));
    assert!(!raw.contains("do-not-log"));
}

fn init_git_repo() -> tempfile::TempDir {
    let dir = tempdir().unwrap();
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(dir.path())
        .status()
        .unwrap();
    std::process::Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(dir.path())
        .status()
        .unwrap();
    std::process::Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(dir.path())
        .status()
        .unwrap();
    std::fs::write(dir.path().join("README.md"), "hello").unwrap();
    std::process::Command::new("git")
        .args(["add", "README.md"])
        .current_dir(dir.path())
        .status()
        .unwrap();
    std::process::Command::new("git")
        .args(["commit", "-m", "initial"])
        .current_dir(dir.path())
        .status()
        .unwrap();
    dir
}

#[test]
fn repo_snapshot_logs_git_status() {
    let repo = init_git_repo();
    let file = repo.path().join("events.jsonl");
    std::fs::write(repo.path().join("README.md"), "hello world").unwrap();
    Command::cargo_bin("runtrail")
        .unwrap()
        .args([
            "repo",
            "snapshot",
            "--cwd",
            repo.path().to_str().unwrap(),
            "--file",
            file.to_str().unwrap(),
        ])
        .assert()
        .success();

    let raw = std::fs::read_to_string(file).unwrap();
    let stored: Value = serde_json::from_str(raw.trim()).unwrap();
    assert_eq!(stored["event"], "repo.snapshot");
    assert_eq!(stored["body"]["dirty"], true);
    assert_eq!(stored["body"]["files"][0]["path"], "README.md");
}

#[test]
fn repo_diff_logs_stat_and_patch() {
    let repo = init_git_repo();
    let file = repo.path().join("events.jsonl");
    std::fs::write(repo.path().join("README.md"), "hello world").unwrap();
    Command::cargo_bin("runtrail")
        .unwrap()
        .args([
            "repo",
            "diff",
            "--cwd",
            repo.path().to_str().unwrap(),
            "--file",
            file.to_str().unwrap(),
        ])
        .assert()
        .success();

    let raw = std::fs::read_to_string(file).unwrap();
    let stored: Value = serde_json::from_str(raw.trim()).unwrap();
    assert_eq!(stored["event"], "repo.diff");
    assert!(
        stored["body"]["stat"]
            .as_str()
            .unwrap()
            .contains("README.md")
    );
    assert!(
        stored["body"]["patch"]
            .as_str()
            .unwrap()
            .contains("hello world")
    );
}

#[test]
fn run_command_logs_start_and_end_events() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("events.jsonl");
    Command::cargo_bin("runtrail")
        .unwrap()
        .args([
            "run",
            "--file",
            file.to_str().unwrap(),
            "--cwd",
            dir.path().to_str().unwrap(),
            "--",
            "sh",
            "-c",
            "printf hello",
        ])
        .assert()
        .success();

    let raw = std::fs::read_to_string(file).unwrap();
    let events: Vec<Value> = raw
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0]["event"], "command.start");
    assert_eq!(events[1]["event"], "command.end");
    assert_eq!(events[1]["body"]["stdout_preview"], "hello");
}

#[test]
fn run_command_logs_start_before_child_exits() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("events.jsonl");
    let marker = dir.path().join("child-started");
    let script = format!("printf started > '{}'; sleep 1", marker.display());
    let mut child = std::process::Command::new(assert_cmd::cargo::cargo_bin("runtrail"))
        .args([
            "run",
            "--file",
            file.to_str().unwrap(),
            "--cwd",
            dir.path().to_str().unwrap(),
            "--",
            "sh",
            "-c",
            &script,
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();

    for _ in 0..50 {
        if marker.exists() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert!(marker.exists(), "child command did not start");
    let raw = std::fs::read_to_string(&file).unwrap();
    assert!(raw.contains("command.start"));
    assert!(!raw.contains("command.end"));

    let status = child.wait().unwrap();
    assert!(status.success());
}

#[test]
fn run_command_logs_spawn_failure_after_start() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("events.jsonl");
    Command::cargo_bin("runtrail")
        .unwrap()
        .args([
            "run",
            "--file",
            file.to_str().unwrap(),
            "--cwd",
            dir.path().to_str().unwrap(),
            "--",
            "runtrail-definitely-missing-command",
        ])
        .assert()
        .code(127);

    let raw = std::fs::read_to_string(file).unwrap();
    let events: Vec<Value> = raw
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0]["event"], "command.start");
    assert_eq!(events[1]["event"], "command.end");
    assert_eq!(events[1]["body"]["success"], false);
    assert_eq!(events[1]["body"]["exit_code"], 127);
    assert!(events[1]["body"].get("spawn_error").is_some());
}

#[test]
fn run_command_redacts_secret_argv_in_events_and_attrs() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("events.jsonl");
    Command::cargo_bin("runtrail")
        .unwrap()
        .args([
            "run",
            "--file",
            file.to_str().unwrap(),
            "--cwd",
            dir.path().to_str().unwrap(),
            "--",
            "sh",
            "-c",
            "printf ok",
            "--token",
            "SECRET123",
        ])
        .assert()
        .success();

    let raw = std::fs::read_to_string(file).unwrap();
    assert!(!raw.contains("SECRET123"));
    assert!(raw.contains("[REDACTED]"));
}

#[test]
fn run_command_returns_child_exit_code_and_logs_error() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("events.jsonl");
    Command::cargo_bin("runtrail")
        .unwrap()
        .args([
            "run",
            "--file",
            file.to_str().unwrap(),
            "--cwd",
            dir.path().to_str().unwrap(),
            "--",
            "sh",
            "-c",
            "exit 7",
        ])
        .assert()
        .code(7);

    let raw = std::fs::read_to_string(file).unwrap();
    assert!(raw.contains("command.end"));
    assert!(raw.contains("\"level\":\"error\""));
    assert!(raw.contains("\"exit_code\":7"));
}

#[test]
fn repair_prompt_outputs_agent_ready_markdown() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("events.jsonl");
    Command::cargo_bin("runtrail")
        .unwrap()
        .args([
            "run",
            "--file",
            file.to_str().unwrap(),
            "--cwd",
            dir.path().to_str().unwrap(),
            "--",
            "sh",
            "-c",
            "echo boom >&2; exit 2",
        ])
        .assert()
        .code(2);

    Command::cargo_bin("runtrail")
        .unwrap()
        .args(["repair-prompt", "--file", file.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Agent Repair Prompt"))
        .stdout(predicate::str::contains("Failure Evidence"))
        .stdout(predicate::str::contains("Safe Commands To Try"))
        .stdout(predicate::str::contains("boom"));
}

#[test]
fn index_and_inspect_make_trails_easier_to_query() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("events.jsonl");
    Command::cargo_bin("runtrail")
        .unwrap()
        .args([
            "log",
            "--file",
            file.to_str().unwrap(),
            "--event",
            "agent.note",
            "--message",
            "hello",
        ])
        .assert()
        .success();

    Command::cargo_bin("runtrail")
        .unwrap()
        .args(["index", "--file", file.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("agent.note"))
        .stdout(predicate::str::contains("seq"));

    Command::cargo_bin("runtrail")
        .unwrap()
        .args([
            "inspect",
            "--file",
            file.to_str().unwrap(),
            "--event",
            "agent.note",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("agent.note"))
        .stdout(predicate::str::contains("hello"));
}

#[test]
fn replay_outputs_conservative_command_hints() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("events.jsonl");
    Command::cargo_bin("runtrail")
        .unwrap()
        .args([
            "run",
            "--file",
            file.to_str().unwrap(),
            "--",
            "sh",
            "-c",
            "printf ok",
        ])
        .assert()
        .success();

    Command::cargo_bin("runtrail")
        .unwrap()
        .args(["replay", "--file", file.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("runtrail Replay Hints"))
        .stdout(predicate::str::contains("sh -c 'printf ok'"));
}

#[test]
fn ci_capture_logs_fixture_context_and_artifacts() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("events.jsonl");
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(dir.path())
        .status()
        .unwrap();
    std::process::Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(dir.path())
        .status()
        .unwrap();
    std::process::Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(dir.path())
        .status()
        .unwrap();
    std::fs::write(dir.path().join("Cargo.toml"), "[package]\nname='x'\n").unwrap();
    std::process::Command::new("git")
        .args(["add", "Cargo.toml"])
        .current_dir(dir.path())
        .status()
        .unwrap();
    std::process::Command::new("git")
        .args(["commit", "-m", "initial"])
        .current_dir(dir.path())
        .status()
        .unwrap();

    Command::cargo_bin("runtrail")
        .unwrap()
        .args([
            "ci",
            "capture",
            "--file",
            file.to_str().unwrap(),
            "--cwd",
            dir.path().to_str().unwrap(),
        ])
        .assert()
        .success();

    let raw = std::fs::read_to_string(file).unwrap();
    let stored: Value = serde_json::from_str(raw.trim()).unwrap();
    assert_eq!(stored["event"], "ci.capture");
    assert_eq!(stored["body"]["dependencies"]["rust"]["cargo_toml"], true);
    assert_eq!(stored["body"]["artifacts"]["dir"], ".runtrail/artifacts");
    assert!(dir.path().join(".runtrail/artifacts").exists());
}

#[test]
fn completions_generates_shell_script() {
    Command::cargo_bin("runtrail")
        .unwrap()
        .args(["completions", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::contains("_runtrail"));
}
