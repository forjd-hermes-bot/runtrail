use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use tempfile::tempdir;

#[test]
fn log_message_appends_event_and_prints_json() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("events.jsonl");

    let output = Command::cargo_bin("cel")
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

    Command::cargo_bin("cel")
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
    Command::cargo_bin("cel")
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
        Command::cargo_bin("cel")
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

    let output = Command::cargo_bin("cel")
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
    Command::cargo_bin("cel")
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
    Command::cargo_bin("cel")
        .unwrap()
        .args(["validate", "--file", good.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("valid"));

    let bad = dir.path().join("bad.jsonl");
    std::fs::write(&bad, "{nope}\n").unwrap();
    Command::cargo_bin("cel")
        .unwrap()
        .args(["validate", "--file", bad.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("line 1"));
}

#[test]
fn summarise_outputs_counts_warnings_and_recent_events() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("events.jsonl");
    Command::cargo_bin("cel")
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
    Command::cargo_bin("cel")
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

    Command::cargo_bin("cel")
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
    Command::cargo_bin("cel")
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
    Command::cargo_bin("cel")
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

    Command::cargo_bin("cel")
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
    let mut cmd = Command::cargo_bin("cel").unwrap();
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
