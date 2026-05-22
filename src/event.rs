use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use ulid::Ulid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Trace,
    Debug,
    #[default]
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub v: u8,
    pub id: String,
    pub seq: u64,
    pub ts: String,
    pub event: String,
    #[serde(default)]
    pub level: Level,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub src: Option<String>,
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub attrs: Map<String, Value>,
    #[serde(default = "default_body")]
    pub body: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_span_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

fn default_body() -> Value {
    Value::Object(Map::new())
}

#[derive(Debug, Clone)]
pub struct NewEvent {
    pub seq: u64,
    pub event: String,
    pub level: Level,
    pub src: Option<String>,
    pub attrs: Map<String, Value>,
    pub body: Value,
    pub trace_id: Option<String>,
    pub span_id: Option<String>,
    pub parent_span_id: Option<String>,
    pub duration_ms: Option<u64>,
}

impl Event {
    pub fn new(input: NewEvent) -> Self {
        Self {
            v: 1,
            id: Ulid::new().to_string(),
            seq: input.seq,
            ts: OffsetDateTime::now_utc()
                .format(&Rfc3339)
                .expect("RFC3339 formatting never fails"),
            event: input.event,
            level: input.level,
            src: input.src,
            attrs: input.attrs,
            body: input.body,
            trace_id: input.trace_id,
            span_id: input.span_id,
            parent_span_id: input.parent_span_id,
            duration_ms: input.duration_ms,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.v != 1 {
            return Err(format!("unsupported schema version {}", self.v));
        }
        if self.id.trim().is_empty() {
            return Err("id is required".to_string());
        }
        if self.seq == 0 {
            return Err("seq must be a positive integer".to_string());
        }
        if self.event.trim().is_empty() {
            return Err("event is required".to_string());
        }
        OffsetDateTime::parse(&self.ts, &Rfc3339).map_err(|err| format!("invalid ts: {err}"))?;
        if let Some(trace_id) = &self.trace_id {
            validate_hex(trace_id, 32, "trace_id")?;
        }
        if let Some(span_id) = &self.span_id {
            validate_hex(span_id, 16, "span_id")?;
        }
        if let Some(parent_span_id) = &self.parent_span_id {
            validate_hex(parent_span_id, 16, "parent_span_id")?;
        }
        Ok(())
    }
}

fn validate_hex(value: &str, len: usize, field: &str) -> Result<(), String> {
    if value.len() != len
        || !value
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
    {
        return Err(format!("{field} must be {len} lowercase hex characters"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new_event(seq: u64, event: &str) -> Event {
        Event::new(NewEvent {
            seq,
            event: event.to_string(),
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
    fn valid_minimal_event_passes_validation() {
        let event = new_event(1, "agent.note");
        assert!(event.validate().is_ok());
    }

    #[test]
    fn empty_event_name_fails_validation() {
        let event = new_event(1, " ");
        assert_eq!(event.validate().unwrap_err(), "event is required");
    }

    #[test]
    fn zero_sequence_fails_validation() {
        let event = new_event(0, "agent.note");
        assert_eq!(
            event.validate().unwrap_err(),
            "seq must be a positive integer"
        );
    }

    #[test]
    fn invalid_trace_id_fails_validation() {
        let mut event = new_event(1, "agent.note");
        event.trace_id = Some("ABC".to_string());
        assert!(event.validate().unwrap_err().contains("trace_id"));
    }

    #[test]
    fn invalid_span_id_fails_validation() {
        let mut event = new_event(1, "agent.note");
        event.span_id = Some("123".to_string());
        assert!(event.validate().unwrap_err().contains("span_id"));
    }

    #[test]
    fn invalid_level_string_fails_deserialization() {
        let json =
            r#"{"v":1,"id":"evt","seq":1,"ts":"2026-05-22T12:00:00Z","event":"x","level":"loud"}"#;
        let parsed: Result<Event, _> = serde_json::from_str(json);
        assert!(parsed.is_err());
    }
}
