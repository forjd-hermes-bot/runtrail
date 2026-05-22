use crate::command_run;
use crate::diff::LogDiff;
use crate::event::{Event, Level, NewEvent};
use crate::git;
use crate::log_io::{append_event, next_seq, read_events, validate_file};
use crate::repair;
use crate::summary::{Summary, format_level};
use anyhow::{Context, Result, anyhow};
use clap::{Parser, Subcommand};
use serde_json::{Map, Value, json};
use std::env;
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
    /// Compare two event logs.
    Diff(DiffArgs),
    /// CI helpers.
    Ci(CiArgs),
    /// Git repository helpers.
    Repo(RepoArgs),
    /// Run a command and log start/end events.
    Run(RunArgs),
    /// Generate an agent-ready repair prompt from an event log.
    RepairPrompt(RepairPromptArgs),
}

#[derive(Debug, Parser)]
struct RepairPromptArgs {
    /// Log file path.
    #[arg(long, default_value = DEFAULT_LOG_FILE)]
    file: PathBuf,
}

#[derive(Debug, Parser)]
struct RunArgs {
    /// Log file path.
    #[arg(long, default_value = DEFAULT_LOG_FILE)]
    file: PathBuf,
    /// Command working directory.
    #[arg(long, default_value = ".")]
    cwd: PathBuf,
    /// Number of stdout/stderr bytes to keep in previews.
    #[arg(long, default_value_t = 4096)]
    preview_bytes: usize,
    /// Command and args to run.
    #[arg(required = true, trailing_var_arg = true, allow_hyphen_values = true)]
    command: Vec<String>,
}

#[derive(Debug, Parser)]
struct RepoArgs {
    #[command(subcommand)]
    command: RepoCommands,
}

#[derive(Debug, Subcommand)]
enum RepoCommands {
    /// Log current git status and metadata.
    Snapshot(RepoSnapshotArgs),
    /// Log current git diff metadata and optional patch.
    Diff(RepoDiffArgs),
}

#[derive(Debug, Parser)]
struct RepoSnapshotArgs {
    /// Log file path.
    #[arg(long, default_value = DEFAULT_LOG_FILE)]
    file: PathBuf,
    /// Repository working directory.
    #[arg(long, default_value = ".")]
    cwd: PathBuf,
}

#[derive(Debug, Parser)]
struct RepoDiffArgs {
    /// Log file path.
    #[arg(long, default_value = DEFAULT_LOG_FILE)]
    file: PathBuf,
    /// Repository working directory.
    #[arg(long, default_value = ".")]
    cwd: PathBuf,
    /// Omit full patch and record only diff --stat.
    #[arg(long)]
    stat_only: bool,
}

#[derive(Debug, Parser)]
struct CiArgs {
    #[command(subcommand)]
    command: CiCommands,
}

#[derive(Debug, Subcommand)]
enum CiCommands {
    /// Capture safe GitHub Actions environment context.
    GithubContext(CiGithubContextArgs),
}

#[derive(Debug, Parser)]
struct CiGithubContextArgs {
    /// Log file path.
    #[arg(long, default_value = DEFAULT_LOG_FILE)]
    file: PathBuf,
}

#[derive(Debug, Parser)]
struct DiffArgs {
    /// Earlier log file.
    before: PathBuf,
    /// Later log file.
    after: PathBuf,
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
        Commands::Diff(args) => diff(args),
        Commands::Ci(args) => ci(args),
        Commands::Repo(args) => repo(args),
        Commands::Run(args) => run_command(args),
        Commands::RepairPrompt(args) => repair_prompt(args),
    }
}

fn log(args: LogArgs) -> Result<()> {
    let seq = next_seq(&args.file)
        .with_context(|| format!("failed to inspect {}", args.file.display()))?;
    let attrs = parse_attrs(&args.attrs)?;
    let body = parse_body(args.body.as_deref(), args.message.as_deref())?;
    append_new_event(AppendNewEvent {
        file: &args.file,
        seq,
        name: args.event,
        level: args.level.into(),
        src: Some(args.src),
        attrs,
        body,
        trace_id: args.trace_id,
        span_id: args.span_id,
        parent_span_id: args.parent_span_id,
        duration_ms: args.duration_ms,
    })
}

struct AppendNewEvent<'a> {
    file: &'a Path,
    seq: u64,
    name: String,
    level: Level,
    src: Option<String>,
    attrs: Map<String, Value>,
    body: Value,
    trace_id: Option<String>,
    span_id: Option<String>,
    parent_span_id: Option<String>,
    duration_ms: Option<u64>,
}

fn append_new_event(args: AppendNewEvent<'_>) -> Result<()> {
    let event = Event::new(NewEvent {
        seq: args.seq,
        event: args.name,
        level: args.level,
        src: args.src,
        attrs: args.attrs,
        body: args.body,
        trace_id: args.trace_id,
        span_id: args.span_id,
        parent_span_id: args.parent_span_id,
        duration_ms: args.duration_ms,
    });
    event.validate().map_err(|message| anyhow!(message))?;
    append_event(args.file, &event)?;
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

fn diff(args: DiffArgs) -> Result<()> {
    let before = read_events(&args.before)?;
    let after = read_events(&args.after)?;
    let diff = LogDiff::between(&before, &after);
    print!("{}", diff.to_markdown());
    Ok(())
}

fn repair_prompt(args: RepairPromptArgs) -> Result<()> {
    let events = read_events(&args.file)?;
    print!("{}", repair::repair_prompt(&events));
    Ok(())
}

fn ci(args: CiArgs) -> Result<()> {
    match args.command {
        CiCommands::GithubContext(args) => github_context(args),
    }
}

fn repo(args: RepoArgs) -> Result<()> {
    match args.command {
        RepoCommands::Snapshot(args) => repo_snapshot(args),
        RepoCommands::Diff(args) => repo_diff(args),
    }
}

fn repo_snapshot(args: RepoSnapshotArgs) -> Result<()> {
    let seq = next_seq(&args.file)
        .with_context(|| format!("failed to inspect {}", args.file.display()))?;
    let body = git::snapshot_body(&args.cwd)?;
    append_new_event(AppendNewEvent {
        file: &args.file,
        seq,
        name: "repo.snapshot".to_string(),
        level: Level::Info,
        src: Some("git".to_string()),
        attrs: Map::new(),
        body,
        trace_id: None,
        span_id: None,
        parent_span_id: None,
        duration_ms: None,
    })
}

fn repo_diff(args: RepoDiffArgs) -> Result<()> {
    let seq = next_seq(&args.file)
        .with_context(|| format!("failed to inspect {}", args.file.display()))?;
    let body = git::diff_body(&args.cwd, args.stat_only)?;
    append_new_event(AppendNewEvent {
        file: &args.file,
        seq,
        name: "repo.diff".to_string(),
        level: Level::Info,
        src: Some("git".to_string()),
        attrs: Map::new(),
        body,
        trace_id: None,
        span_id: None,
        parent_span_id: None,
        duration_ms: None,
    })
}

fn run_command(args: RunArgs) -> Result<()> {
    let start_seq = next_seq(&args.file)
        .with_context(|| format!("failed to inspect {}", args.file.display()))?;
    let result = command_run::run_command(&args.cwd, &args.command, args.preview_bytes)?;
    let mut attrs = Map::new();
    attrs.insert("cmd".to_string(), Value::String(args.command.join(" ")));
    append_new_event(AppendNewEvent {
        file: &args.file,
        seq: start_seq,
        name: "command.start".to_string(),
        level: Level::Info,
        src: Some("cel".to_string()),
        attrs: attrs.clone(),
        body: result.start_body,
        trace_id: None,
        span_id: None,
        parent_span_id: None,
        duration_ms: None,
    })?;
    append_new_event(AppendNewEvent {
        file: &args.file,
        seq: start_seq + 1,
        name: "command.end".to_string(),
        level: if result.exit_code == 0 {
            Level::Info
        } else {
            Level::Error
        },
        src: Some("cel".to_string()),
        attrs,
        body: result.end_body,
        trace_id: None,
        span_id: None,
        parent_span_id: None,
        duration_ms: None,
    })?;
    if result.exit_code == 0 {
        Ok(())
    } else {
        std::process::exit(result.exit_code);
    }
}

fn github_context(args: CiGithubContextArgs) -> Result<()> {
    let seq = next_seq(&args.file)
        .with_context(|| format!("failed to inspect {}", args.file.display()))?;
    let mut attrs = Map::new();
    for key in GITHUB_ENV_ALLOWLIST {
        if let Ok(value) = env::var(key) {
            attrs.insert((*key).to_string(), Value::String(value));
        }
    }
    append_new_event(AppendNewEvent {
        file: &args.file,
        seq,
        name: "ci.github.context".to_string(),
        level: Level::Info,
        src: Some("github-actions".to_string()),
        attrs,
        body: Value::Object(Map::new()),
        trace_id: None,
        span_id: None,
        parent_span_id: None,
        duration_ms: None,
    })
}

const GITHUB_ENV_ALLOWLIST: &[&str] = &[
    "GITHUB_WORKFLOW",
    "GITHUB_RUN_ID",
    "GITHUB_RUN_NUMBER",
    "GITHUB_RUN_ATTEMPT",
    "GITHUB_JOB",
    "GITHUB_ACTION",
    "GITHUB_ACTOR",
    "GITHUB_EVENT_NAME",
    "GITHUB_REF",
    "GITHUB_SHA",
    "GITHUB_REPOSITORY",
    "RUNNER_OS",
    "RUNNER_ARCH",
];

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
