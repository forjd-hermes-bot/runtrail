use crate::event::Event;
use anyhow::{Context, Result, anyhow};
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationIssue {
    pub line: usize,
    pub message: String,
}

pub fn append_event(path: &Path, event: &Event) -> Result<()> {
    event.validate().map_err(|message| anyhow!(message))?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create log directory {}", parent.display()))?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("failed to open {} for append", path.display()))?;
    serde_json::to_writer(&mut file, event).context("failed to serialize event")?;
    file.write_all(b"\n").context("failed to write newline")?;
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
    let max_seq = read_events(path)?
        .into_iter()
        .map(|event| event.seq)
        .max()
        .unwrap_or(0);
    Ok(max_seq + 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{Level, NewEvent};
    use serde_json::{Map, Value};
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
