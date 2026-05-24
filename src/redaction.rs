use serde_json::{Map, Value, json};

pub const REDACTION: &str = "[REDACTED]";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preview {
    pub text: String,
    pub truncated: bool,
    pub redacted: bool,
}

impl Preview {
    pub fn metadata(&self) -> serde_json::Value {
        json!({
            "truncated": self.truncated,
            "redacted": self.redacted,
        })
    }
}

#[derive(Debug, Clone)]
struct Token<'a> {
    start: usize,
    end: usize,
    text: &'a str,
}

pub fn preview(bytes: &[u8], limit: usize) -> Preview {
    let truncated = bytes.len() > limit;
    let slice = if truncated { &bytes[..limit] } else { bytes };
    preview_from_capped(slice, truncated)
}

pub fn preview_from_capped(bytes: &[u8], truncated: bool) -> Preview {
    let raw = String::from_utf8_lossy(bytes).to_string();
    let text = redact_secrets(&raw);
    let redacted = text != raw;
    Preview {
        text,
        truncated,
        redacted,
    }
}

pub fn redact_argv(command: &[String]) -> Vec<String> {
    let mut redact_next = false;
    command
        .iter()
        .map(|arg| {
            if redact_next {
                redact_next = false;
                return REDACTION.to_string();
            }
            if is_sensitive_flag(arg) {
                redact_next = true;
                return arg.clone();
            }
            redact_secrets(arg)
        })
        .collect()
}

pub fn redact_map(map: Map<String, Value>) -> Map<String, Value> {
    map.into_iter()
        .map(|(key, value)| {
            let redacted = if key_is_sensitive(&key) {
                Value::String(REDACTION.to_string())
            } else if is_command_key(&key) {
                redact_command_value(value)
            } else {
                redact_value(value)
            };
            (key, redacted)
        })
        .collect()
}

pub fn redact_value(value: Value) -> Value {
    match value {
        Value::String(value) => Value::String(redact_secrets(&value)),
        Value::Array(values) => Value::Array(values.into_iter().map(redact_value).collect()),
        Value::Object(map) => Value::Object(redact_map(map)),
        other => other,
    }
}

pub fn redact_secrets(input: &str) -> String {
    let tokens = tokenize(input);
    if tokens.is_empty() {
        return input.to_string();
    }

    let mut replacements = Vec::new();
    let mut redact_next = false;
    for token in &tokens {
        if redact_next {
            replacements.push((token.start, token.end, REDACTION.to_string()));
            redact_next = false;
            continue;
        }

        if is_sensitive_flag(token.text) {
            redact_next = true;
            continue;
        }

        if let Some(redacted) = redact_assignment_or_header(token.text) {
            replacements.push((token.start, token.end, redacted));
            continue;
        }

        if is_bearer_marker(token.text) {
            redact_next = true;
            continue;
        }

        if looks_like_standalone_secret(token.text) {
            replacements.push((token.start, token.end, REDACTION.to_string()));
        }
    }

    if replacements.is_empty() {
        return input.to_string();
    }

    let mut output = String::with_capacity(input.len());
    let mut cursor = 0;
    for (start, end, replacement) in replacements {
        if start < cursor {
            continue;
        }
        output.push_str(&input[cursor..start]);
        output.push_str(&replacement);
        cursor = end;
    }
    output.push_str(&input[cursor..]);
    output
}

fn tokenize(input: &str) -> Vec<Token<'_>> {
    let mut tokens = Vec::new();
    let mut start = None;
    for (idx, ch) in input.char_indices() {
        if ch.is_whitespace() {
            if let Some(token_start) = start.take() {
                tokens.push(Token {
                    start: token_start,
                    end: idx,
                    text: &input[token_start..idx],
                });
            }
        } else if start.is_none() {
            start = Some(idx);
        }
    }
    if let Some(token_start) = start {
        tokens.push(Token {
            start: token_start,
            end: input.len(),
            text: &input[token_start..],
        });
    }
    tokens
}

fn redact_assignment_or_header(token: &str) -> Option<String> {
    let (key, separator, value) = split_assignment(token)?;
    if value.is_empty() || !key_is_sensitive(key) {
        return None;
    }
    Some(format!("{key}{separator}{REDACTION}"))
}

fn split_assignment(token: &str) -> Option<(&str, char, &str)> {
    let equals = token.find('=');
    let colon = token.find(':');
    let idx = match (equals, colon) {
        (Some(eq), Some(co)) => eq.min(co),
        (Some(eq), None) => eq,
        (None, Some(co)) => co,
        (None, None) => return None,
    };
    let separator = token[idx..].chars().next()?;
    Some((
        &token[..idx],
        separator,
        &token[idx + separator.len_utf8()..],
    ))
}

fn redact_command_value(value: Value) -> Value {
    match value {
        Value::Array(values) => {
            let mut command = Vec::with_capacity(values.len());
            let mut all_strings = true;
            for value in values {
                if let Value::String(part) = value {
                    command.push(part);
                } else {
                    all_strings = false;
                    break;
                }
            }
            if all_strings {
                Value::Array(
                    redact_argv(&command)
                        .into_iter()
                        .map(Value::String)
                        .collect(),
                )
            } else {
                Value::String(REDACTION.to_string())
            }
        }
        other => redact_value(other),
    }
}

fn is_command_key(key: &str) -> bool {
    matches!(
        key.to_ascii_lowercase().as_str(),
        "cmd" | "command" | "argv"
    )
}

fn key_is_sensitive(key: &str) -> bool {
    let normalized = normalize_key(key);
    const NAMES: &[&str] = &[
        "authorization",
        "token",
        "secret",
        "password",
        "passwd",
        "api_key",
        "apikey",
        "access_key",
        "private_key",
    ];
    NAMES.iter().any(|name| normalized.contains(name))
}

fn normalize_key(value: &str) -> String {
    value
        .trim_matches(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_' && ch != '-')
        .to_ascii_lowercase()
        .replace('-', "_")
}

fn is_sensitive_flag(token: &str) -> bool {
    if token.contains('=') || token.contains(':') {
        return false;
    }
    let normalized = normalize_key(token).trim_start_matches('_').to_string();
    matches!(
        normalized.as_str(),
        "token"
            | "secret"
            | "password"
            | "passwd"
            | "api_key"
            | "apikey"
            | "access_key"
            | "private_key"
    ) || normalized.starts_with("token_")
        || normalized.starts_with("secret_")
        || normalized.ends_with("_token")
        || normalized.ends_with("_secret")
}

fn is_bearer_marker(token: &str) -> bool {
    let normalized = normalize_key(token);
    normalized == "bearer"
        || normalized.ends_with("_bearer")
        || normalized.ends_with(":bearer")
        || normalized.ends_with("=bearer")
}

fn looks_like_standalone_secret(token: &str) -> bool {
    let trimmed =
        token.trim_matches(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_' && ch != '-');
    trimmed.starts_with("ghp_")
        || trimmed.starts_with("github_pat_")
        || trimmed.starts_with("xoxb-")
        || trimmed.starts_with("xoxp-")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn preview_redacts_secret_looking_assignments() {
        let preview = preview(b"ok token=abc123 password:hunter2 ghp_deadbeef", 200);
        assert_eq!(
            preview.text,
            "ok token=[REDACTED] password:[REDACTED] [REDACTED]"
        );
        assert!(preview.redacted);
        assert!(!preview.truncated);
    }

    #[test]
    fn preview_tracks_truncation() {
        let preview = preview(b"abcdef", 3);
        assert_eq!(preview.text, "abc");
        assert!(preview.truncated);
        assert!(!preview.redacted);
    }

    #[test]
    fn preview_preserves_benign_whitespace() {
        let preview = preview(b"line 1\n  line 2\tstill ok", 200);
        assert_eq!(preview.text, "line 1\n  line 2\tstill ok");
        assert!(!preview.redacted);
    }

    #[test]
    fn preview_redacts_bearer_token_without_collapsing_whitespace() {
        let preview = preview(b"Authorization: Bearer SECRET123\n\tpassword:hunter2", 200);
        assert_eq!(
            preview.text,
            "Authorization: Bearer [REDACTED]\n\tpassword:[REDACTED]"
        );
        assert!(preview.redacted);
    }

    #[test]
    fn redact_argv_redacts_sensitive_flag_values() {
        let argv = vec![
            "curl".to_string(),
            "--token".to_string(),
            "SECRET123".to_string(),
            "-H".to_string(),
            "Authorization: Bearer TOKEN456".to_string(),
        ];
        assert_eq!(
            redact_argv(&argv),
            vec![
                "curl",
                "--token",
                "[REDACTED]",
                "-H",
                "Authorization: Bearer [REDACTED]",
            ]
        );
    }

    #[test]
    fn redact_value_handles_sensitive_attrs_and_command_arrays() {
        let value = json!({
            "token": "abc123",
            "cmd": ["tool", "--password", "hunter2"],
            "message": "ok"
        });
        let redacted = redact_value(value);
        assert_eq!(redacted["token"], REDACTION);
        assert_eq!(redacted["cmd"][2], REDACTION);
        assert_eq!(redacted["message"], "ok");
    }
}
