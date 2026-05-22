# runtrail

`runtrail` (`runtrail`) is a tiny Rust CLI and JSONL event format for recording agent actions, browser QA steps, repo changes, command/test results, notes, and CI events in one local, portable, diffable stream.

It is meant to be a cheap black-box recorder for agentic development workflows: easy to append to, easy to inspect with shell tools, and easy to summarise into repair prompts.

## Status

MVP complete:

- JSONL schema v1
- `log`, `tail`, `summarise`, `diff`, `validate`, `repair-prompt`
- `run` command wrapper for start/end command evidence
- `repo snapshot` and `repo diff` helpers for Git metadata
- `ci github-context` helper for safe GitHub Actions metadata capture
- Rust tests and performance smoke script

## Install / build

Install the latest binary from GitHub:

```bash
curl -fsSL https://raw.githubusercontent.com/forjd-hermes-bot/runtrail/main/install.sh | bash
```

Optional environment variables:

- `RUNTRAIL_INSTALL_DIR` — install directory, default `~/.local/bin`
- `RUNTRAIL_INSTALL_TAG` — release tag, default `latest`
- `RUNTRAIL_INSTALL_REPO` — GitHub repo, default `forjd-hermes-bot/runtrail`

Build from source:

```bash
cargo build --release
./target/release/runtrail --help
```

During development:

```bash
cargo test
cargo clippy --all-targets -- -D warnings
scripts/perf-smoke.sh
```

## Event schema v1

Each line is one compact JSON object. See [`docs/schema-v1.md`](docs/schema-v1.md) for the full envelope and event body conventions.

Required fields:

- `schema`: schema version, currently `cel.v1`
- `id`: event ULID
- `seq`: positive integer sequence number within the log file
- `ts`: RFC3339 UTC timestamp
- `event`: dotted event name

Common fields:

- `level`: `trace`, `debug`, `info`, `warn`, `error`
- `src`: event source, e.g. `runtrail`, `hermes-agent`, `github-actions`
- `attrs`: structured metadata object
- `body`: event-specific JSON payload
- `trace_id`, `span_id`, `parent_span_id`: optional trace correlation fields
- `duration_ms`: optional duration

Example:

```json
{"schema":"cel.v1","id":"01KS...","seq":1,"ts":"2026-05-22T12:34:56Z","event":"agent.note","level":"info","src":"runtrail","attrs":{},"body":{"message":"hello"}}
```

Example logs live in:

- [`examples/browser-qa.jsonl`](examples/browser-qa.jsonl)
- [`examples/ci-failure.jsonl`](examples/ci-failure.jsonl)
- [`examples/agent-session.jsonl`](examples/agent-session.jsonl)

## Commands

### Log an event

```bash
runtrail log --event agent.note --message "Investigating failing CI"
```

Default file: `.runtrail/events.jsonl`.

With attributes and JSON body:

```bash
runtrail log \
  --event command.run \
  --attr tool.name=terminal \
  --attr exit_code=0 \
  --body '{"cmd":"cargo test"}'
```

### Tail recent events

```bash
runtrail tail --lines 5
runtrail tail --lines 5 --json
```

### Summarise for humans or agents

```bash
runtrail summarise --file .runtrail/events.jsonl
```

The summary includes total events, first/last timestamps, counts by event and level, warnings/errors, and recent events.

### Diff two logs

```bash
runtrail diff before.jsonl after.jsonl
```

The diff reports count deltas, added/removed event IDs, and newly introduced warnings/errors.

### Run a command and capture evidence

```bash
runtrail run -- cargo test
runtrail run --file .runtrail/events.jsonl --cwd . --preview-bytes 4096 -- npm test
```

`runtrail run` emits `command.start` and `command.end` events. It returns the child command exit code.

### Capture repository evidence

```bash
runtrail repo snapshot
runtrail repo diff
runtrail repo diff --stat-only
```

`repo snapshot` captures branch, HEAD, dirty state, and `git status --porcelain` files. `repo diff` captures `git diff --stat` plus the patch unless `--stat-only` is used.

### Generate an agent repair prompt

```bash
runtrail repair-prompt --file .runtrail/events.jsonl
```

The prompt includes failure evidence, recent command results, repository context when present, suspected causes, and safe commands to try.

### Validate a log

```bash
runtrail validate --file .runtrail/events.jsonl
```

Validation checks JSONL framing, required fields, schema version, sequence numbers, timestamp parsing, levels, and trace/span ID format.

### Capture GitHub Actions context

```bash
runtrail ci github-context --file .runtrail/events.jsonl
```

This records only a safe allowlist of environment variables:

- `GITHUB_WORKFLOW`
- `GITHUB_RUN_ID`
- `GITHUB_RUN_NUMBER`
- `GITHUB_RUN_ATTEMPT`
- `GITHUB_JOB`
- `GITHUB_ACTION`
- `GITHUB_ACTOR`
- `GITHUB_EVENT_NAME`
- `GITHUB_REF`
- `GITHUB_SHA`
- `GITHUB_REPOSITORY`
- `RUNNER_OS`
- `RUNNER_ARCH`

### Generate shell completions

```bash
runtrail completions bash > runtrail.bash
runtrail completions zsh > _runtrail
runtrail completions fish > runtrail.fish
```

## Example event types

```bash
runtrail log --event command.run --body '{"cmd":"cargo test","exit_code":0}'
runtrail log --event browser.navigate --attr browser.url=https://example.com
runtrail log --event browser.assert --body '{"text":"Dashboard loaded","ok":true}'
runtrail log --event test.result --body '{"runner":"cargo test","passed":21,"failed":0}'
runtrail log --event repo.change --body '{"files":[{"path":"src/main.rs","status":"M"}]}'
runtrail log --event ci.status --attr github.run_id=123 --body '{"conclusion":"success"}'
runtrail log --event agent.note --message "Failure likely caused by missing env var"
```

## Interoperability

Because logs are JSONL, they work with normal shell tools:

```bash
jq 'select(.event == "repo.change")' .runtrail/events.jsonl
jq 'select(.level == "error")' .runtrail/events.jsonl
lnav .runtrail/events.jsonl
```

## Design notes

Research and plans live in:

- `docs/research/`
- `docs/mvp-spec.md`
- `docs/plans/`

The MVP is intentionally JSONL-first. Binary export, FlatBuffers, indexes, custom query languages, and full CI replay are future work.
