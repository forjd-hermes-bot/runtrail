use crate::event::Event;
use anyhow::{Context, Result, anyhow};
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationIssue {
    pub line: usize,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationMode {
    Default,
    Strict,
}

struct LogLock {
    path: PathBuf,
    _file: File,
}

impl LogLock {
    fn acquire(log_path: &Path) -> Result<Self> {
        let lock_path = lock_path(log_path);
        let started = Instant::now();
        loop {
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&lock_path)
            {
                Ok(file) => {
                    return Ok(Self {
                        path: lock_path,
                        _file: file,
                    });
                }
                Err(err) if err.kind() == ErrorKind::AlreadyExists => {
                    if started.elapsed() > Duration::from_secs(10) {
                        return Err(anyhow!(
                            "timed out waiting for log lock {}",
                            lock_path.display()
                        ));
                    }
                    thread::sleep(Duration::from_millis(10));
                }
                Err(err) => {
                    return Err(err).with_context(|| {
                        format!("failed to create log lock {}", lock_path.display())
                    });
                }
            }
        }
    }
}

impl Drop for LogLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

pub fn append_event(path: &Path, event: &Event) -> Result<()> {
    ensure_parent(path)?;
    let _lock = LogLock::acquire(path)?;
    append_event_locked(path, event)
}

pub fn append_event_with_next_seq(path: &Path, mut event: Event) -> Result<Event> {
    ensure_parent(path)?;
    let _lock = LogLock::acquire(path)?;
    event.seq = max_seq(path)?
        .checked_add(1)
        .ok_or_else(|| anyhow!("seq overflow for {}", path.display()))?;
    append_event_locked(path, &event)?;
    Ok(event)
}

fn append_event_locked(path: &Path, event: &Event) -> Result<()> {
    event.validate().map_err(|message| anyhow!(message))?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("failed to open {} for append", path.display()))?;
    let mut record = serde_json::to_vec(event).context("failed to serialize event")?;
    record.push(b'\n');
    file.write_all(&record).context("failed to write event")?;
    Ok(())
}

pub fn read_events(path: &Path) -> Result<Vec<Event>> {
    let file =
        fs::File::open(path).with_context(|| format!("failed to open {}", path.display()))?;
    let reader = BufReader::new(file);
    let mut events = Vec::new();
    for (idx, line) in reader.lines().enumerate() {
        let line_number = idx + 1;
        let line = line.with_context(|| format!("failed to read line {line_number}"))?;
        if line.trim().is_empty() {
            continue;
        }
        let event: Event = serde_json::from_str(&line)
            .with_context(|| format!("line {line_number}: invalid JSON event"))?;
        event
            .validate()
            .map_err(|message| anyhow!("line {line_number}: {message}"))?;
        events.push(event);
    }
    Ok(events)
}

pub fn validate_file(path: &Path) -> Vec<ValidationIssue> {
    validate_file_with_mode(path, ValidationMode::Default)
}

pub fn validate_file_with_mode(path: &Path, mode: ValidationMode) -> Vec<ValidationIssue> {
    let file = match fs::File::open(path) {
        Ok(file) => file,
        Err(err) => {
            return vec![ValidationIssue {
                line: 0,
                message: format!("failed to open {}: {err}", path.display()),
            }];
        }
    };
    let reader = BufReader::new(file);
    let mut issues = Vec::new();
    for (idx, line) in reader.lines().enumerate() {
        let line_number = idx + 1;
        let line = match line {
            Ok(line) => line,
            Err(err) => {
                issues.push(ValidationIssue {
                    line: line_number,
                    message: format!("failed to read line: {err}"),
                });
                continue;
            }
        };
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<Event>(&line) {
            Ok(event) => {
                if let Err(message) = event.validate() {
                    issues.push(ValidationIssue {
                        line: line_number,
                        message,
                    });
                }
                if mode == ValidationMode::Strict && event.seq != line_number as u64 {
                    issues.push(ValidationIssue {
                        line: line_number,
                        message: format!("strict mode: seq must match line number {line_number}"),
                    });
                }
            }
            Err(err) => issues.push(ValidationIssue {
                line: line_number,
                message: format!("invalid JSON event: {err}"),
            }),
        }
    }
    issues
}

pub fn next_seq(path: &Path) -> Result<u64> {
    if !path.exists() {
        return Ok(1);
    }
    max_seq(path)?
        .checked_add(1)
        .ok_or_else(|| anyhow!("seq overflow for {}", path.display()))
}

fn max_seq(path: &Path) -> Result<u64> {
    if !path.exists() {
        return Ok(0);
    }
    Ok(read_events(path)?
        .into_iter()
        .map(|event| event.seq)
        .max()
        .unwrap_or(0))
}

fn ensure_parent(path: &Path) -> Result<()> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create log directory {}", parent.display()))?;
    }
    Ok(())
}

fn lock_path(path: &Path) -> PathBuf {
    let mut lock = path.as_os_str().to_os_string();
    lock.push(".lock");
    PathBuf::from(lock)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{Level, NewEvent};
    use serde_json::{Map, Value};
    use std::sync::{Arc, Barrier};
    use tempfile::tempdir;

    fn event(seq: u64, name: &str) -> Event {
        Event::new(NewEvent {
            seq,
            event: name.to_string(),
            level: Level::Info,
            src: Some("test".to_string()),
            attrs: Map::new(),
            body: Value::Object(Map::new()),
            trace_id: None,
            span_id: None,
            parent_span_id: None,
            duration_ms: None,
        })
    }

    #[test]
    fn append_creates_parent_directories_and_writes_newline() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("nested/events.jsonl");
        append_event(&path, &event(1, "agent.note")).unwrap();
        let raw = fs::read_to_string(path).unwrap();
        assert!(raw.ends_with('\n'));
        assert_eq!(raw.lines().count(), 1);
    }

    #[test]
    fn append_with_next_seq_assigns_sequence_atomically() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("events.jsonl");
        let first = append_event_with_next_seq(&path, event(1, "agent.note")).unwrap();
        let second = append_event_with_next_seq(&path, event(1, "command.run")).unwrap();
        assert_eq!(first.seq, 1);
        assert_eq!(second.seq, 2);
    }

    #[test]
    fn concurrent_append_with_next_seq_uses_unique_sequences() {
        let dir = tempdir().unwrap();
        let path = Arc::new(dir.path().join("events.jsonl"));
        let writers = 16;
        let barrier = Arc::new(Barrier::new(writers));
        let mut handles = Vec::new();
        for idx in 0..writers {
            let path = Arc::clone(&path);
            let barrier = Arc::clone(&barrier);
            handles.push(std::thread::spawn(move || {
                barrier.wait();
                append_event_with_next_seq(&path, event(1, &format!("agent.note.{idx}"))).unwrap();
            }));
        }
        for handle in handles {
            handle.join().unwrap();
        }
        let mut seqs = read_events(&path)
            .unwrap()
            .into_iter()
            .map(|event| event.seq)
            .collect::<Vec<_>>();
        seqs.sort_unstable();
        assert_eq!(seqs, (1..=writers as u64).collect::<Vec<_>>());
    }

    #[test]
    fn read_events_returns_written_events() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("events.jsonl");
        append_event(&path, &event(1, "agent.note")).unwrap();
        append_event(&path, &event(2, "command.run")).unwrap();
        let events = read_events(&path).unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[1].event, "command.run");
    }

    #[test]
    fn next_seq_is_max_seq_plus_one() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("events.jsonl");
        assert_eq!(next_seq(&path).unwrap(), 1);
        append_event(&path, &event(7, "agent.note")).unwrap();
        assert_eq!(next_seq(&path).unwrap(), 8);
    }

    #[test]
    fn next_seq_reports_overflow() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("events.jsonl");
        append_event(&path, &event(u64::MAX, "agent.note")).unwrap();
        assert!(
            next_seq(&path)
                .unwrap_err()
                .to_string()
                .contains("overflow")
        );
    }

    #[test]
    fn invalid_json_reports_line_number() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("events.jsonl");
        fs::write(&path, "{nope}\n").unwrap();
        let issues = validate_file(&path);
        assert_eq!(issues[0].line, 1);
        assert!(issues[0].message.contains("invalid JSON"));
    }

    #[test]
    fn empty_lines_are_ignored() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("events.jsonl");
        append_event(&path, &event(1, "agent.note")).unwrap();
        fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap()
            .write_all(b"\n")
            .unwrap();
        assert_eq!(read_events(&path).unwrap().len(), 1);
        assert!(validate_file(&path).is_empty());
    }
}
