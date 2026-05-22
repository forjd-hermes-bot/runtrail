# runtrail MVP Specification

## Pitch

`runtrail` (`runtrail`) is a tiny Rust CLI and JSONL event format for recording agent actions, browser QA steps, repository changes, command/test results, notes, and CI events in one local, portable, diffable stream.

## Goals

- Provide a JSONL-first event schema that is easy for humans, agents, and shell tools to read.
- Provide a Rust CLI with `log`, `tail`, `summarise`, `diff`, and `validate` commands.
- Keep events append-only and deterministic enough for local replay/summarisation.
- Capture common event shapes for commands, browser steps, test results, file/repo changes, agent notes, and GitHub/CI statuses.
- Remain interoperable with `jq`, `lnav`, Git, and CI systems.

## Non-goals for MVP

- FlatBuffers or binary storage.
- Custom query language.
- Full CI replay engine.
- Full OpenTelemetry or CloudEvents exporter.
- Native Git object parsing.
- Long-lived daemon/server.

## Package and binary

- Crate name: `runtrail`
- Binary name: `runtrail`
- Language: Rust 2021/2024-compatible edition selected by Cargo.

## Event schema v1

Each log line is one compact JSON object.

Required fields:

- `v`: integer schema version, initially `1`.
- `id`: unique event ID, generated as ULID.
- `seq`: positive integer sequence number within the log file.
- `ts`: RFC3339 UTC timestamp.
- `event`: event name string.

Recommended fields:

- `level`: `trace`, `debug`, `info`, `warn`, or `error`; default `info`.
- `src`: source string, e.g. `runtrail`, `hermes-agent`, `github-actions`.
- `attrs`: object containing string keys and JSON scalar/object/array values.
- `body`: event-specific JSON value, usually object.

Optional fields:

- `trace_id`: 32 lowercase hex chars.
- `span_id`: 16 lowercase hex chars.
- `parent_span_id`: 16 lowercase hex chars.
- `duration_ms`: non-negative integer.

Example:

```json
{"v":1,"id":"01J...","seq":1,"ts":"2026-05-22T12:34:56Z","level":"info","event":"agent.tool.end","src":"hermes-agent","attrs":{"tool.name":"terminal"},"body":{"cmd":"cargo test","exit_code":0}}
```

## Event naming

Use dotted names:

- `command.run`
- `browser.navigate`
- `browser.assert`
- `test.result`
- `repo.change`
- `repo.commit`
- `agent.note`
- `ci.status`
- `ci.github.context`
- `artifact.created`
- `error`

The CLI must accept arbitrary event names, not only built-ins.

## CLI commands

### `runtrail log`

Append an event to a JSONL log file.

Required option:

- `--event <name>`

Common options:

- `--file <path>`: default `.runtrail/events.jsonl`.
- `--level <level>`: default `info`.
- `--src <source>`: default `runtrail`.
- `--attr key=value`: repeatable, parses values as JSON when possible, otherwise string.
- `--body <json>`: JSON body value; default `{}`.
- `--message <text>`: shorthand body `{ "message": text }` unless `--body` is supplied.
- `--duration-ms <n>`.
- `--trace-id`, `--span-id`, `--parent-span-id`.

Behavior:

- Create parent directories automatically.
- Determine `seq` by reading the existing file and using max sequence + 1.
- Append one compact JSON object plus newline.
- Print the event JSON to stdout.

### `runtrail tail`

Show recent events.

Options:

- `--file <path>` default `.runtrail/events.jsonl`.
- `--lines <n>` default `20`.
- `--json`: print raw JSONL records instead of human text.

### `runtrail summarise`

Summarise a log for humans/agents.

Options:

- `--file <path>` default `.runtrail/events.jsonl`.
- `--markdown`: output Markdown; default true for MVP.

Summary content:

- Total events.
- First and last timestamp.
- Counts by event name.
- Counts by level.
- Error/warn events with short previews.
- Recent events.

### `runtrail diff`

Compare two event logs.

Arguments:

- `before`
- `after`

Output:

- Counts by event for both logs and deltas.
- New events in `after` by ID.
- Removed events by ID.
- New warn/error events.

### `runtrail validate`

Validate a JSONL event log.

Checks:

- Each non-empty line is a JSON object.
- Required fields are present.
- `v == 1`.
- `seq` is a positive integer.
- `ts` parses as RFC3339.
- `event` is non-empty.
- `level` is valid if present.
- trace/span IDs have valid lowercase hex lengths if present.

Output:

- Human-readable validation report.
- Non-zero exit code if invalid.

## Repo layout

```text
Cargo.toml
README.md
src/
  main.rs
  event.rs
  log_io.rs
  cli.rs
  summary.rs
  diff.rs
tests/
  cli.rs
docs/
  research/
  plans/
  mvp-spec.md
```

## Testing strategy

- Unit tests for event validation, attr parsing, summary, diff, and JSONL IO.
- CLI integration tests using temp directories and `assert_cmd`.
- Golden-ish assertions on JSON output without brittle timestamp IDs.
- Performance smoke test over at least 10k events.

## Performance targets

On the local development machine:

- Validate 10k events in under 1 second in release mode.
- Summarise 10k events in under 1 second in release mode.
- Append should avoid rewriting the whole file; MVP may scan for max `seq`, but should be acceptable for moderate logs.

## Release readiness

Before public GitHub publish:

- `cargo fmt --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test`
- release-mode performance smoke test
- README includes install, usage examples, schema, and status
- docs/research and docs/plans are committed
