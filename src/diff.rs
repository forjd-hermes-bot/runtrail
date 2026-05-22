use crate::event::{Event, Level};
use crate::summary::{format_level, preview};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Default)]
pub struct LogDiff {
    pub before_total: usize,
    pub after_total: usize,
    pub before_counts: BTreeMap<String, usize>,
    pub after_counts: BTreeMap<String, usize>,
    pub added: Vec<Event>,
    pub removed: Vec<Event>,
    pub new_warnings_and_errors: Vec<Event>,
}

impl LogDiff {
    pub fn between(before: &[Event], after: &[Event]) -> Self {
        let before_ids: BTreeSet<_> = before.iter().map(|event| event.id.as_str()).collect();
        let after_ids: BTreeSet<_> = after.iter().map(|event| event.id.as_str()).collect();
        let added: Vec<Event> = after
            .iter()
            .filter(|event| !before_ids.contains(event.id.as_str()))
            .cloned()
            .collect();
        let removed: Vec<Event> = before
            .iter()
            .filter(|event| !after_ids.contains(event.id.as_str()))
            .cloned()
            .collect();
        let new_warnings_and_errors = added
            .iter()
            .filter(|event| matches!(event.level, Level::Warn | Level::Error))
            .cloned()
            .collect();
        Self {
            before_total: before.len(),
            after_total: after.len(),
            before_counts: counts(before),
            after_counts: counts(after),
            added,
            removed,
            new_warnings_and_errors,
        }
    }

    pub fn to_markdown(&self) -> String {
        let mut out = String::new();
        out.push_str("# compact-event-log diff\n\n");
        out.push_str(&format!("- Before events: {}\n", self.before_total));
        out.push_str(&format!("- After events: {}\n", self.after_total));
        out.push_str(&format!(
            "- Delta: {}\n\n",
            self.after_total as isize - self.before_total as isize
        ));
        out.push_str("## Counts by event\n\n");
        let names: BTreeSet<_> = self
            .before_counts
            .keys()
            .chain(self.after_counts.keys())
            .cloned()
            .collect();
        if names.is_empty() {
            out.push_str("- none\n");
        }
        for name in names {
            let before = self.before_counts.get(&name).copied().unwrap_or(0);
            let after = self.after_counts.get(&name).copied().unwrap_or(0);
            out.push_str(&format!(
                "- `{name}`: {before} -> {after} ({:+})\n",
                after as isize - before as isize
            ));
        }
        render_events(&mut out, "Added events", &self.added);
        render_events(&mut out, "Removed events", &self.removed);
        render_events(
            &mut out,
            "New warnings and errors",
            &self.new_warnings_and_errors,
        );
        out
    }
}

fn counts(events: &[Event]) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for event in events {
        *counts.entry(event.event.clone()).or_insert(0) += 1;
    }
    counts
}

fn render_events(out: &mut String, title: &str, events: &[Event]) {
    out.push_str(&format!("\n## {title}\n\n"));
    if events.is_empty() {
        out.push_str("- none\n");
    } else {
        for event in events {
            out.push_str(&format!(
                "- `{}` #{} `{}` {} — {}\n",
                event.id,
                event.seq,
                format_level(&event.level),
                event.event,
                preview(&event.body)
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{Event, NewEvent};
    use serde_json::{Map, json};

    fn event(seq: u64, name: &str, level: Level) -> Event {
        Event::new(NewEvent {
            seq,
            event: name.to_string(),
            level,
            src: Some("test".to_string()),
            attrs: Map::new(),
            body: json!({"message": name}),
            trace_id: None,
            span_id: None,
            parent_span_id: None,
            duration_ms: None,
        })
    }

    #[test]
    fn diff_finds_added_removed_and_new_errors() {
        let kept = event(1, "agent.note", Level::Info);
        let removed = event(2, "old", Level::Info);
        let added = event(3, "error", Level::Error);
        let diff = LogDiff::between(&[kept.clone(), removed.clone()], &[kept, added.clone()]);
        assert_eq!(diff.added.len(), 1);
        assert_eq!(diff.removed.len(), 1);
        assert_eq!(diff.new_warnings_and_errors.len(), 1);
        assert!(diff.to_markdown().contains("New warnings and errors"));
    }
}
