use crate::redaction;
use serde_json::{Value, json};
use std::io::{self, Read};
use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};
use std::thread;
use std::time::Instant;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

#[derive(Debug, Clone)]
pub struct CommandRunResult {
    pub end_body: Value,
    pub exit_code: i32,
}

#[derive(Debug, Clone)]
struct CappedOutput {
    bytes: Vec<u8>,
    truncated: bool,
}

pub fn start_body(cwd: &Path, command: &[String]) -> anyhow::Result<Value> {
    anyhow::ensure!(!command.is_empty(), "command must not be empty");
    let started_at = OffsetDateTime::now_utc().format(&Rfc3339)?;
    Ok(json!({
        "cmd": redaction::redact_argv(command),
        "cwd": redaction::redact_secrets(&cwd.display().to_string()),
        "started_at": started_at,
    }))
}

pub fn run_command(
    cwd: &Path,
    command: &[String],
    preview_bytes: usize,
) -> anyhow::Result<CommandRunResult> {
    anyhow::ensure!(!command.is_empty(), "command must not be empty");
    let timer = Instant::now();
    let mut child = match Command::new(&command[0])
        .args(&command[1..])
        .current_dir(cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(err) => return Ok(spawn_error_result(cwd, command, preview_bytes, timer, err)),
    };

    let stdout = child
        .stdout
        .take()
        .expect("stdout was piped when command was spawned");
    let stderr = child
        .stderr
        .take()
        .expect("stderr was piped when command was spawned");
    let stdout_thread = thread::spawn(move || read_capped(stdout, preview_bytes));
    let stderr_thread = thread::spawn(move || read_capped(stderr, preview_bytes));

    let status = child.wait()?;
    let stdout = join_capture(stdout_thread)?;
    let stderr = join_capture(stderr_thread)?;
    let duration_ms = timer.elapsed().as_millis() as u64;
    let exit_code = exit_code(status);
    let stdout_preview = redaction::preview_from_capped(&stdout.bytes, stdout.truncated);
    let stderr_preview = redaction::preview_from_capped(&stderr.bytes, stderr.truncated);
    let end_body = json!({
        "cmd": redaction::redact_argv(command),
        "cwd": redaction::redact_secrets(&cwd.display().to_string()),
        "exit_code": exit_code,
        "success": status.success(),
        "duration_ms": duration_ms,
        "stdout_preview": stdout_preview.text,
        "stderr_preview": stderr_preview.text,
        "stdout_truncated": stdout_preview.truncated,
        "stderr_truncated": stderr_preview.truncated,
        "stdout_redacted": stdout_preview.redacted,
        "stderr_redacted": stderr_preview.redacted,
        "capture": {
            "stdout": stdout_preview.metadata(),
            "stderr": stderr_preview.metadata(),
        }
    });
    Ok(CommandRunResult {
        end_body,
        exit_code,
    })
}

fn spawn_error_result(
    cwd: &Path,
    command: &[String],
    preview_bytes: usize,
    timer: Instant,
    err: io::Error,
) -> CommandRunResult {
    let duration_ms = timer.elapsed().as_millis() as u64;
    let message = redaction::redact_secrets(&err.to_string());
    let stderr_preview = redaction::preview(message.as_bytes(), preview_bytes);
    let end_body = json!({
        "cmd": redaction::redact_argv(command),
        "cwd": redaction::redact_secrets(&cwd.display().to_string()),
        "exit_code": 127,
        "success": false,
        "duration_ms": duration_ms,
        "spawn_error": message,
        "stdout_preview": "",
        "stderr_preview": stderr_preview.text,
        "stdout_truncated": false,
        "stderr_truncated": stderr_preview.truncated,
        "stdout_redacted": false,
        "stderr_redacted": stderr_preview.redacted,
        "capture": {
            "stdout": {
                "truncated": false,
                "redacted": false,
            },
            "stderr": stderr_preview.metadata(),
        }
    });
    CommandRunResult {
        end_body,
        exit_code: 127,
    }
}

fn read_capped<R: Read>(mut reader: R, limit: usize) -> io::Result<CappedOutput> {
    let mut bytes = Vec::with_capacity(limit.min(8192));
    let mut truncated = false;
    let mut chunk = [0_u8; 8192];
    loop {
        let read = reader.read(&mut chunk)?;
        if read == 0 {
            break;
        }
        let remaining = limit.saturating_sub(bytes.len());
        if remaining > 0 {
            let keep = remaining.min(read);
            bytes.extend_from_slice(&chunk[..keep]);
        }
        if read > remaining {
            truncated = true;
        }
    }
    Ok(CappedOutput { bytes, truncated })
}

fn join_capture(
    handle: thread::JoinHandle<io::Result<CappedOutput>>,
) -> anyhow::Result<CappedOutput> {
    handle
        .join()
        .map_err(|_| anyhow::anyhow!("command output reader thread panicked"))?
        .map_err(Into::into)
}

#[cfg(unix)]
fn exit_code(status: ExitStatus) -> i32 {
    use std::os::unix::process::ExitStatusExt;
    status
        .code()
        .or_else(|| status.signal().map(|signal| 128 + signal))
        .unwrap_or(128)
}

#[cfg(not(unix))]
fn exit_code(status: ExitStatus) -> i32 {
    status.code().unwrap_or(128)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn start_body_contains_redacted_command_and_start_time() {
        let dir = tempdir().unwrap();
        let command = vec![
            "tool".to_string(),
            "--token".to_string(),
            "SECRET123".to_string(),
        ];
        let body = start_body(dir.path(), &command).unwrap();
        assert_eq!(body["cmd"][1], "--token");
        assert_eq!(body["cmd"][2], redaction::REDACTION);
        assert!(body["started_at"].as_str().is_some());
    }

    #[test]
    fn run_command_captures_exit_code_and_output() {
        let dir = tempdir().unwrap();
        let command = vec![
            "sh".to_string(),
            "-c".to_string(),
            "printf hello".to_string(),
        ];
        let result = run_command(dir.path(), &command, 100).unwrap();
        assert_eq!(result.exit_code, 0);
        assert_eq!(result.end_body["stdout_preview"], "hello");
        assert_eq!(result.end_body["success"], true);
    }

    #[test]
    fn run_command_truncates_preview_without_buffering_full_output() {
        let dir = tempdir().unwrap();
        let command = vec![
            "sh".to_string(),
            "-c".to_string(),
            "printf 123456".to_string(),
        ];
        let result = run_command(dir.path(), &command, 3).unwrap();
        assert_eq!(result.end_body["stdout_preview"], "123");
        assert_eq!(result.end_body["stdout_truncated"], true);
        assert_eq!(result.end_body["capture"]["stdout"]["truncated"], true);
    }

    #[test]
    fn run_command_redacts_secret_looking_output() {
        let dir = tempdir().unwrap();
        let command = vec![
            "sh".to_string(),
            "-c".to_string(),
            "printf 'token=abc123 password:hunter2'".to_string(),
        ];
        let result = run_command(dir.path(), &command, 100).unwrap();
        assert_eq!(
            result.end_body["stdout_preview"],
            "token=[REDACTED] password:[REDACTED]"
        );
        assert_eq!(result.end_body["stdout_redacted"], true);
        assert_eq!(result.end_body["capture"]["stdout"]["redacted"], true);
    }

    #[test]
    fn run_command_redacts_secret_looking_argv() {
        let dir = tempdir().unwrap();
        let command = vec![
            "sh".to_string(),
            "-c".to_string(),
            "printf ok".to_string(),
            "--token".to_string(),
            "SECRET123".to_string(),
        ];
        let result = run_command(dir.path(), &command, 100).unwrap();
        assert_eq!(result.end_body["cmd"][3], "--token");
        assert_eq!(result.end_body["cmd"][4], redaction::REDACTION);
    }

    #[test]
    fn run_command_returns_spawn_failure_as_logged_result() {
        let dir = tempdir().unwrap();
        let command = vec!["runtrail-definitely-missing-command".to_string()];
        let result = run_command(dir.path(), &command, 100).unwrap();
        assert_eq!(result.exit_code, 127);
        assert_eq!(result.end_body["success"], false);
        assert!(result.end_body["spawn_error"].as_str().is_some());
    }
}
