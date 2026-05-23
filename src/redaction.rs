use serde_json::json;

const REDACTION: &str = "[REDACTED]";

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

pub fn preview(bytes: &[u8], limit: usize) -> Preview {
    let truncated = bytes.len() > limit;
    let slice = if truncated { &bytes[..limit] } else { bytes };
    let raw = String::from_utf8_lossy(slice).to_string();
    let text = redact_secrets(&raw);
    let redacted = text != raw;
    Preview {
        text,
        truncated,
        redacted,
    }
}

pub fn redact_secrets(input: &str) -> String {
    input
        .split_whitespace()
        .map(redact_token)
        .collect::<Vec<_>>()
        .join(" ")
}

fn redact_token(token: &str) -> String {
    let lower = token.to_ascii_lowercase();
    if looks_like_assignment_secret(&lower) || looks_like_bearer_token(&lower, token) {
        return redact_assignment_or_token(token);
    }
    token.to_string()
}

fn looks_like_assignment_secret(lower: &str) -> bool {
    const NAMES: &[&str] = &["token", "secret", "password", "passwd", "api_key", "apikey"];
    NAMES.iter().any(|name| {
        lower.starts_with(&format!("{name}="))
            || lower.starts_with(&format!("{name}:"))
            || lower.contains(&format!("_{name}="))
            || lower.contains(&format!("-{name}="))
    })
}

fn looks_like_bearer_token(lower: &str, token: &str) -> bool {
    lower == "bearer" || token.starts_with("ghp_") || token.starts_with("github_pat_")
}

fn redact_assignment_or_token(token: &str) -> String {
    if let Some((key, _)) = token.split_once('=') {
        format!("{key}={REDACTION}")
    } else if let Some((key, _)) = token.split_once(':') {
        format!("{key}:{REDACTION}")
    } else {
        REDACTION.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
