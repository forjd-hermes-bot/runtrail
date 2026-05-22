use crate::event::{Event, Level};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default)]
pub struct Summary {
    pub total: usize,
    pub first_ts: Option<String>,
    pub last_ts: Option<String>,
    pub by_event: BTreeMap<String, usize>,
    pub by_level: BTreeMap<String, usize>,
    pub warnings_and_errors: Vec<Event>,
    pub recent: Vec<Event>,
}

impl Summary {
    pub fn from_events(events: &[Event], recent_limit: usize) -> Self {
        let mut summary = Self {
            total: events.len(),
            first_ts: events.first().map(|event| event.ts.clone()),
            last_ts: events.last().map(|event| event.ts.clone()),
            ..Self::default()
        };
        for event in events {
            *summary.by_event.entry(event.event.clone()).or_insert(0) += 1;
            *summary
                .by_level
                .entry(format_level(&event.level).to_string())
                .or_insert(0) += 1;
            if matches!(event.level, Level::Warn | Level::Error) {
                summary.warnings_and_errors.push(event.clone());
            }
        }
        let start = events.len().saturating_sub(recent_limit);
        summary.recent = events[start..].to_vec();
        summary
    }

    pub fn to_markdown(&self) -> String {
        let mut out = String::new();
        out.push_str("# compact-event-log summary\n\n");
        out.push_str(&format!("- Total events: {}\n", self.total));
        out.push_str(&format!(
            "- First timestamp: {}\n",
            self.first_ts.as_deref().unwrap_or("n/a")
        ));
        out.push_str(&format!(
            "- Last timestamp: {}\n\n",
            self.last_ts.as_deref().unwrap_or("n/a")
        ));

        out.push_str("## Counts by event\n\n");
        for (event, count) in &self.by_event {
            out.push_str(&format!("- `{event}`: {count}\n"));
        }
        if self.by_event.is_empty() {
            out.push_str("- none\n");
        }

        out.push_str("\n## Counts by level\n\n");
        for (level, count) in &self.by_level {
            out.push_str(&format!("- `{level}`: {count}\n"));
        }
        if self.by_level.is_empty() {
            out.push_str("- none\n");
        }

        out.push_str("\n## Warnings and errors\n\n");
        if self.warnings_and_errors.is_empty() {
            out.push_str("- none\n");
        } else {
            for event in &self.warnings_and_errors {
                out.push_str(&format!(
                    "- #{} `{}` {} — {}\n",
                    event.seq,
                    format_level(&event.level),
                    event.event,
                    preview(&event.body)
                ));
            }
        }

        out.push_str("\n## Recent events\n\n");
        if self.recent.is_empty() {
            out.push_str("- none\n");
        } else {
            for event in &self.recent {
                out.push_str(&format!(
                    "- #{} `{}` {} — {}\n",
                    event.seq,
                    format_level(&event.level),
                    event.event,
                    preview(&event.body)
                ));
            }
        }
        out
    }
}

pub fn format_level(level: &Level) -> &'static str {
    match level {
        Level::Trace => "trace",
        Level::Debug => "debug",
        Level::Info => "info",
        Level::Warn => "warn",
        Level::Error => "error",
    }
}

pub fn preview(body: &Value) -> String {
    if let Some(message) = body.get("message").and_then(Value::as_str) {
        return truncate(message);
    }
    if let Some(error) = body.get("error").and_then(Value::as_str) {
        return truncate(error);
    }
    truncate(&serde_json::to_string(body).unwrap_or_else(|_| "<unrenderable>".to_string()))
}

fn truncate(value: &str) -> String {
    const LIMIT: usize = 120;
    if value.chars().count() <= LIMIT {
        value.to_string()
    } else {
        format!("{}…", value.chars().take(LIMIT).collect::<String>())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{Event, NewEvent};
    use serde_json::{Map, json};

    fn event(seq: u64, name: &str, level: Level, body: Value) -> Event {
        Event::new(NewEvent {
            seq,
            event: name.to_string(),
            level,
            src: Some("test".to_string()),
            attrs: Map::new(),
            body,
            trace_id: None,
            span_id: None,
            parent_span_id: None,
            duration_ms: None,
        })
    }

    #[test]
    fn summary_counts_events_levels_and_warnings() {
        let events = vec![
            event(1, "agent.note", Level::Info, json!({"message":"ok"})),
            event(2, "error", Level::Error, json!({"error":"boom"})),
        ];
        let summary = Summary::from_events(&events, 1);
        assert_eq!(summary.total, 2);
        assert_eq!(summary.by_event["agent.note"], 1);
        assert_eq!(summary.by_level["error"], 1);
        assert_eq!(summary.warnings_and_errors.len(), 1);
        assert_eq!(summary.recent.len(), 1);
        let markdown = summary.to_markdown();
        assert!(markdown.contains("Total events: 2"));
        assert!(markdown.contains("boom"));
    }
}
