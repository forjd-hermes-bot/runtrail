# runtrail

[![CI](https://github.com/forjd/runtrail/actions/workflows/ci.yml/badge.svg)](https://github.com/forjd/runtrail/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/forjd/runtrail?include_prereleases&sort=semver)](https://github.com/forjd/runtrail/releases)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

**Portable event trails for agentic dev workflows.**

`runtrail` is a tiny Rust CLI and JSONL event format for recording command evidence, browser QA steps, repository snapshots/diffs, CI context, test results, and agent notes in one local, diffable stream.

Think of it as a cheap black-box recorder for coding agents and CI failures: append structured evidence while work happens, then turn the trail into summaries, diffs, or an agent-ready repair prompt.

## Why runtrail?

Coding agents are most useful when failures arrive with portable context instead of a vague “CI failed”. `runtrail` keeps that context close to the repo:

- **Local-first**: writes plain JSONL to `.runtrail/events.jsonl` by default.
- **Shell-friendly**: inspect with `tail`, `jq`, `lnav`, or normal diffs.
- **Agent-ready**: summarise failures and generate focused repair prompts.
- **CI-safe**: captures only an allowlist of GitHub Actions metadata.
- **Small and boring**: no daemon, database, hosted service, or custom query language required.

## Install

Install an immutable release tag from GitHub releases:

```bash
curl -fsSL https://raw.githubusercontent.com/forjd/runtrail/main/install.sh \
  | RUNTRAIL_INSTALL_TAG=runtrail-v0.3.0 bash
```

The installer detects OS/architecture, downloads the matching release asset, requires `SHA256SUMS` verification by default, and installs to `~/.local/bin`.

Optional environment variables:

```bash
RUNTRAIL_INSTALL_TAG=runtrail-v0.3.0 bash install.sh         # required release tag
RUNTRAIL_INSTALL_DIR=/usr/local/bin bash install.sh          # install directory
RUNTRAIL_INSTALL_REPO=forjd/runtrail bash install.sh         # alternate repo
RUNTRAIL_INSTALL_TAG=latest RUNTRAIL_INSTALL_ALLOW_LATEST=1 bash install.sh
RUNTRAIL_INSTALL_SKIP_CHECKSUM=1 bash install.sh             # explicit integrity opt-out
```

Build from source:

```bash
cargo build --release
./target/release/runtrail --help
```

## Quick start

Capture a command run:

```bash
runtrail run -- cargo test
```

Add repository context:

```bash
runtrail repo snapshot
runtrail repo diff
```

Summarise the trail:

```bash
runtrail summarise --file .runtrail/events.jsonl
```

Generate a repair prompt for an agent:

```bash
runtrail repair-prompt --file .runtrail/events.jsonl > repair.md
```

Capture a GitHub Actions repair fixture:

```bash
runtrail ci capture --file .runtrail/events.jsonl
runtrail repair-prompt --file .runtrail/events.jsonl > .runtrail/repair.md
```

See [`docs/github-actions.md`](docs/github-actions.md) for a copy-paste workflow that uploads enumerated runtrail repair files on CI failure.

A typical failure-capture flow looks like this:

```bash
runtrail repo snapshot
runtrail run -- cargo test
runtrail repo diff
runtrail repair-prompt --file .runtrail/events.jsonl > repair.md
```

## What gets recorded?

Each line in the log is one compact JSON object:

```json
{"schema":"runtrail.v1","id":"01KS...","seq":1,"ts":"2026-05-22T12:34:56Z","event":"agent.note","level":"info","src":"runtrail","attrs":{},"body":{"message":"Investigating failing CI"}}
```

Required envelope fields:

| Field | Meaning |
| --- | --- |
| `schema` | Schema identifier. Current schema is `runtrail.v1`. |
| `id` | 26-character event ULID. |
| `seq` | Positive sequence number within the log file. |
| `ts` | RFC3339 UTC timestamp. |
| `event` | Dotted event name, for example `command.end`. |

Additional envelope fields:

| Field | Meaning |
| --- | --- |
| `level` | Required severity: `trace`, `debug`, `info`, `warn`, or `error`. |
| `src` | Optional event source, for example `runtrail`, `hermes-agent`, or `github-actions`. |
| `attrs` | Required small structured metadata object useful for filtering. Use `{}` when empty. |
| `body` | Required event-specific JSON payload. Use `{}` when empty. |
| `trace_id`, `span_id`, `parent_span_id` | Optional trace correlation fields. |
| `duration_ms` | Optional duration in milliseconds. |

See [`docs/schema-v1.md`](docs/schema-v1.md) for full schema notes and conventions.

Example logs:

- [`examples/browser-qa.jsonl`](examples/browser-qa.jsonl)
- [`examples/ci-failure.jsonl`](examples/ci-failure.jsonl)
- [`examples/agent-session.jsonl`](examples/agent-session.jsonl)

## Command guide

### Append an event

```bash
runtrail log --event agent.note --message "Investigating failing CI"
```

Default log file:

```text
.runtrail/events.jsonl
```

With attributes and a JSON body:

```bash
runtrail log \
  --event command.run \
  --attr tool.name=terminal \
  --attr exit_code=0 \
  --body '{"cmd":"cargo test"}'
```

### Run a command and capture evidence

```bash
runtrail run -- cargo test
runtrail run --file .runtrail/events.jsonl --cwd . --preview-bytes 4096 --env CI -- npm test
```

`runtrail run` emits:

- `command.start`
- `command.end`

The wrapper logs `command.start` before spawning the child and exits with the child command's exit code. Stdout/stderr are streamed into bounded previews so logs stay portable.

### Capture repository evidence

```bash
runtrail repo snapshot
runtrail repo diff
runtrail repo diff --patch
```

`repo snapshot` captures branch, HEAD, dirty state, and `git status --porcelain` file entries.

`repo diff` captures staged and unstaged diff stats by default. Use `--patch` only when full patch content is safe to store.

### Capture GitHub Actions context

```bash
runtrail ci github-context --file .runtrail/events.jsonl
runtrail ci capture --file .runtrail/events.jsonl
```

This records only a safe allowlist of environment variables. For full CI failure capture and artifact upload, see [`docs/github-actions.md`](docs/github-actions.md) and [`examples/github-actions-repair.yml`](examples/github-actions-repair.yml).

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

### Tail recent events

```bash
runtrail tail --lines 5
runtrail tail --lines 5 --json
```

### Summarise a log

```bash
runtrail summarise --file .runtrail/events.jsonl
```

The summary includes:

- total events
- first/last timestamps
- counts by event and level
- warnings/errors
- recent events

### Diff two logs

```bash
runtrail diff before.jsonl after.jsonl
```

The diff reports count deltas, added/removed/changed event IDs, and newly introduced warnings/errors.

### Generate an agent repair prompt

```bash
runtrail repair-prompt --file .runtrail/events.jsonl
```

The prompt includes failure evidence, recent command results, repository context when present, suspected causes, and safe commands to try.

### Build an index, inspect, and replay

```bash
runtrail index --file .runtrail/events.jsonl
runtrail inspect --file .runtrail/events.jsonl --lines 20
runtrail replay --file .runtrail/events.jsonl
```

`index` emits compact JSON query fields, `inspect` shows recent human-readable events, and `replay` prints conservative command hints.

### Validate a log

```bash
runtrail validate --file .runtrail/events.jsonl
runtrail validate --file .runtrail/events.jsonl --strict
```

Validation checks JSONL framing, required fields, schema version, sequence numbers, timestamp parsing, levels, and trace/span ID format. Strict mode also requires each event's `seq` to match its physical JSONL line number, which is useful for CI format hardening before sharing a trail.

### Generate shell completions

```bash
runtrail completions bash > runtrail.bash
runtrail completions zsh > _runtrail
runtrail completions fish > runtrail.fish
```

## Event examples

```bash
runtrail log --event command.run --body '{"cmd":"cargo test","exit_code":0}'
runtrail log --event browser.navigate --attr browser.url=https://example.com
runtrail log --event browser.assert --body '{"text":"Dashboard loaded","ok":true}'
runtrail log --event test.result --body '{"runner":"cargo test","passed":21,"failed":0}'
runtrail log --event repo.change --body '{"files":[{"path":"src/main.rs","status":"M"}]}'
runtrail log --event ci.status --attr github.run_id=123 --body '{"conclusion":"success"}'
runtrail log --event agent.note --message "Failure likely caused by missing env var"
```

Event names are intentionally conventional rather than enforced. Producers can add their own dotted names while keeping the same envelope.

## Interoperability

Because logs are JSONL, they work with normal shell tools:

```bash
jq 'select(.event == "repo.change")' .runtrail/events.jsonl
jq 'select(.level == "error")' .runtrail/events.jsonl
lnav .runtrail/events.jsonl
```

Use `runtrail validate` when you want stricter checks before storing or sharing a log.

## Safety and privacy

`runtrail` records what you ask it to record. A few guardrails are built in:

- command stdout/stderr are captured as bounded previews, not unbounded logs;
- command argv, attrs, and string body values receive best-effort redaction for common token/password patterns;
- GitHub Actions context uses a fixed safe allowlist;
- repo diff stores stats by default, and `--patch` is an explicit full-patch opt-in;
- logs are local files, so you decide if and where to upload them.

Before sharing logs publicly, review `.runtrail/events.jsonl` for secrets, tokens, proprietary patches, or sensitive paths.

## Development

```bash
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
scripts/perf-smoke.sh 10000
```

Release automation is handled by Release Please and GitHub Actions. Binary builds are published for Linux, macOS, and Windows release targets.

## Design notes

Research and planning docs live in:

- [`docs/research/`](docs/research/)
- [`docs/mvp-spec.md`](docs/mvp-spec.md)
- [`docs/plans/`](docs/plans/)

The MVP is intentionally JSONL-first. Binary export, indexes, richer replay, and deeper CI fixture capture are future work.
