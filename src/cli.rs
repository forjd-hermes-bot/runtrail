use crate::event::{Event, Level, NewEvent};
use crate::log_io::{append_event, next_seq, read_events, validate_file};
use crate::summary::{Summary, format_level};
use anyhow::{Context, Result, anyhow};
use clap::{Parser, Subcommand};
use serde_json::{Map, Value, json};
use std::path::{Path, PathBuf};

const DEFAULT_LOG_FILE: &str = ".compact-event-log/events.jsonl";

#[derive(Debug, Parser)]
#[command(name = "cel", version, about = "Compact JSONL event logs")]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Append an event to a JSONL log file.
    Log(LogArgs),
    /// Show recent events.
    Tail(TailArgs),
    /// Validate a JSONL event log.
    Validate(FileArg),
    /// Summarise an event log as Markdown.
    Summarise(SummariseArgs),
}

#[derive(Debug, Parser)]
struct SummariseArgs {
    /// Log file path.
    #[arg(long, default_value = DEFAULT_LOG_FILE)]
    file: PathBuf,
    /// Number of recent events to include.
    #[arg(long, default_value_t = 5)]
    recent: usize,
}

#[derive(Debug, Parser)]
struct FileArg {
    /// Log file path.
    #[arg(long, default_value = DEFAULT_LOG_FILE)]
    file: PathBuf,
}

#[derive(Debug, Parser)]
struct TailArgs {
    /// Log file path.
    #[arg(long, default_value = DEFAULT_LOG_FILE)]
    file: PathBuf,
    /// Number of recent lines to show.
    #[arg(long, default_value_t = 20)]
    lines: usize,
    /// Print raw JSONL records.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Parser)]
struct LogArgs {
    /// Log file path.
    #[arg(long, default_value = DEFAULT_LOG_FILE)]
    file: PathBuf,
    /// Event name, e.g. agent.note.
    #[arg(long)]
    event: String,
    /// Event level.
    #[arg(long, default_value = "info")]
    level: LevelArg,
    /// Event source.
    #[arg(long, default_value = "cel")]
    src: String,
    /// Attribute as key=value. Values are parsed as JSON when possible.
    #[arg(long = "attr")]
    attrs: Vec<String>,
    /// JSON body value.
    #[arg(long)]
    body: Option<String>,
    /// Message shorthand for body {"message": text}.
    #[arg(long)]
    message: Option<String>,
    /// Duration in milliseconds.
    #[arg(long)]
    duration_ms: Option<u64>,
    #[arg(long)]
    trace_id: Option<String>,
    #[arg(long)]
    span_id: Option<String>,
    #[arg(long)]
    parent_span_id: Option<String>,
}

#[derive(Clone, Debug, clap::ValueEnum)]
enum LevelArg {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl From<LevelArg> for Level {
    fn from(value: LevelArg) -> Self {
        match value {
            LevelArg::Trace => Self::Trace,
            LevelArg::Debug => Self::Debug,
            LevelArg::Info => Self::Info,
            LevelArg::Warn => Self::Warn,
            LevelArg::Error => Self::Error,
        }
    }
}

pub fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Log(args) => log(args),
        Commands::Tail(args) => tail(args),
        Commands::Validate(args) => validate(&args.file),
        Commands::Summarise(args) => summarise(args),
    }
}

fn log(args: LogArgs) -> Result<()> {
    let seq = next_seq(&args.file)
        .with_context(|| format!("failed to inspect {}", args.file.display()))?;
    let attrs = parse_attrs(&args.attrs)?;
    let body = parse_body(args.body.as_deref(), args.message.as_deref())?;
    let event = Event::new(NewEvent {
        seq,
        event: args.event,
        level: args.level.into(),
        src: Some(args.src),
        attrs,
        body,
        trace_id: args.trace_id,
        span_id: args.span_id,
        parent_span_id: args.parent_span_id,
        duration_ms: args.duration_ms,
    });
    event.validate().map_err(|message| anyhow!(message))?;
    append_event(&args.file, &event)?;
    println!("{}", serde_json::to_string(&event)?);
    Ok(())
}

fn tail(args: TailArgs) -> Result<()> {
    let events = read_events(&args.file)?;
    let start = events.len().saturating_sub(args.lines);
    for event in &events[start..] {
        if args.json {
            println!("{}", serde_json::to_string(event)?);
        } else {
            println!(
                "{} {} {} {}",
                event.seq,
                event.ts,
                format_level(&event.level),
                event.event
            );
        }
    }
    Ok(())
}

fn summarise(args: SummariseArgs) -> Result<()> {
    let events = read_events(&args.file)?;
    let summary = Summary::from_events(&events, args.recent);
    print!("{}", summary.to_markdown());
    Ok(())
}

fn validate(path: &Path) -> Result<()> {
    let issues = validate_file(path);
    if issues.is_empty() {
        println!("valid: {}", path.display());
        Ok(())
    } else {
        for issue in &issues {
            eprintln!("line {}: {}", issue.line, issue.message);
        }
        Err(anyhow!("validation failed with {} issue(s)", issues.len()))
    }
}

pub fn parse_attrs(values: &[String]) -> Result<Map<String, Value>> {
    let mut attrs = Map::new();
    for value in values {
        let (key, raw) = value
            .split_once('=')
            .ok_or_else(|| anyhow!("attribute must be key=value: {value}"))?;
        if key.trim().is_empty() {
            return Err(anyhow!("attribute key must not be empty"));
        }
        attrs.insert(key.to_string(), parse_jsonish(raw));
    }
    Ok(attrs)
}

fn parse_jsonish(raw: &str) -> Value {
    serde_json::from_str(raw).unwrap_or_else(|_| Value::String(raw.to_string()))
}

fn parse_body(body: Option<&str>, message: Option<&str>) -> Result<Value> {
    if let Some(body) = body {
        serde_json::from_str(body).context("--body must be valid JSON")
    } else if let Some(message) = message {
        Ok(json!({ "message": message }))
    } else {
        Ok(Value::Object(Map::new()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_attrs_decodes_json_scalars() {
        let attrs = parse_attrs(&[
            "a=1".to_string(),
            "b=true".to_string(),
            "c=text".to_string(),
        ])
        .unwrap();
        assert_eq!(attrs["a"], 1);
        assert_eq!(attrs["b"], true);
        assert_eq!(attrs["c"], "text");
    }
}
