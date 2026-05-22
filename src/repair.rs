use crate::event::{Event, Level};
use serde_json::Value;

pub fn repair_prompt(events: &[Event]) -> String {
    let mut output = String::new();
    output.push_str("# Agent Repair Prompt\n\n");
    output
        .push_str("You are repairing a failing local/CI run using compact-event-log evidence.\n\n");
    output.push_str("## Failure Evidence\n\n");
    let failures: Vec<&Event> = events
        .iter()
        .filter(|event| {
            event.level == Level::Warn || event.level == Level::Error || is_failed_command(event)
        })
        .collect();
    if failures.is_empty() {
        output.push_str("No warnings, errors, or failed command events were found. Inspect the summary and recent events before changing code.\n");
    } else {
        for event in failures.iter().rev().take(10).rev() {
            output.push_str(&format!(
                "- seq={} event={} level={:?} evidence={}\n",
                event.seq,
                event.event,
                event.level,
                evidence_preview(&event.body)
            ));
        }
    }

    output.push_str("\n## Repository Context\n\n");
    if let Some(repo) = events
        .iter()
        .rev()
        .find(|event| event.event == "repo.snapshot")
    {
        output.push_str(&format_repo_snapshot(&repo.body));
    } else {
        output.push_str(
            "No `repo.snapshot` event found. Run `cel repo snapshot` if repo context is needed.\n",
        );
    }

    output.push_str("\n## Recent Command Results\n\n");
    let recent_commands: Vec<&Event> = events
        .iter()
        .filter(|event| event.event == "command.end")
        .rev()
        .take(5)
        .collect();
    for event in recent_commands.iter().rev() {
        output.push_str(&format!(
            "- seq={} exit={} cmd={} stderr={} stdout={}\n",
            event.seq,
            event.body.get("exit_code").unwrap_or(&Value::Null),
            compact_json(event.body.get("cmd")),
            compact_json(event.body.get("stderr_preview")),
            compact_json(event.body.get("stdout_preview")),
        ));
    }

    output.push_str("\n## Suspected Causes\n\n");
    output.push_str("- Failed command or test events usually identify the first broken check.\n");
    output
        .push_str("- Recent `repo.diff` / `repo.snapshot` events identify likely changed files.\n");
    output.push_str("- Prefer minimal fixes backed by a reproducing test or rerun command.\n");

    output.push_str("\n## Safe Commands To Try\n\n");
    output.push_str("```bash\n");
    output.push_str("cel summarise --file .compact-event-log/events.jsonl\n");
    output.push_str("cel tail --file .compact-event-log/events.jsonl --lines 20\n");
    output.push_str("cargo fmt --check\n");
    output.push_str("cargo clippy --all-targets -- -D warnings\n");
    output.push_str("cargo test\n");
    output.push_str("```\n");

    output.push_str("\n## Repair Instructions\n\n");
    output.push_str("1. Reproduce the failure with the narrowest command available.\n");
    output.push_str("2. Inspect changed files and the failing evidence above.\n");
    output.push_str("3. Make the smallest safe fix.\n");
    output.push_str("4. Re-run the failing command and then the full quality gate.\n");
    output.push_str(
        "5. Log follow-up evidence with `cel run`, `cel repo snapshot`, or `cel repo diff`.\n",
    );
    output
}

fn is_failed_command(event: &Event) -> bool {
    event.event == "command.end"
        && event
            .body
            .get("exit_code")
            .and_then(Value::as_i64)
            .is_some_and(|code| code != 0)
}

fn evidence_preview(body: &Value) -> String {
    let rendered = body.to_string();
    let truncated = if rendered.chars().count() <= 240 {
        rendered
    } else {
        format!("{}…", rendered.chars().take(240).collect::<String>())
    };
    truncated.replace('\n', "\\n")
}

fn compact_json(value: Option<&Value>) -> String {
    value.map_or_else(|| "null".to_string(), evidence_preview)
}

fn format_repo_snapshot(body: &Value) -> String {
    let mut output = String::new();
    output.push_str(&format!("- branch: {}\n", compact_json(body.get("branch"))));
    output.push_str(&format!("- head: {}\n", compact_json(body.get("head"))));
    output.push_str(&format!("- dirty: {}\n", compact_json(body.get("dirty"))));
    if let Some(files) = body.get("files").and_then(Value::as_array) {
        output.push_str("- changed files:\n");
        for file in files.iter().take(20) {
            output.push_str(&format!(
                "  - {} {}\n",
                compact_json(file.get("status")),
                compact_json(file.get("path"))
            ));
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{Event, NewEvent};
    use serde_json::{Map, json};

    #[test]
    fn repair_prompt_includes_failed_command_and_safe_commands() {
        let event = Event::new(NewEvent {
            seq: 1,
            event: "command.end".to_string(),
            level: Level::Error,
            src: Some("cel".to_string()),
            attrs: Map::new(),
            body: json!({"cmd":["cargo","test"],"exit_code":101,"stderr_preview":"boom"}),
            trace_id: None,
            span_id: None,
            parent_span_id: None,
            duration_ms: None,
        });
        let prompt = repair_prompt(&[event]);
        assert!(prompt.contains("Failure Evidence"));
        assert!(prompt.contains("cargo test"));
        assert!(prompt.contains("Safe Commands To Try"));
    }
}
